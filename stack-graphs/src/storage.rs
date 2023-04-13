// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2023, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use rusqlite::functions::FunctionFlags;
use rusqlite::Connection;
use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;
use thiserror::Error;

use crate::arena::Handle;
use crate::graph::File;
use crate::graph::Node;
use crate::graph::StackGraph;
use crate::partial::PartialPath;
use crate::partial::PartialPaths;
use crate::partial::PartialSymbolStack;
use crate::serde;
use crate::serde::FileFilter;
use crate::stitching::Database;
use crate::stitching::ForwardPartialPathStitcher;
use crate::CancellationError;
use crate::CancellationFlag;

const VERSION: usize = 1;

const SCHEMA: &str = r#"
        CREATE TABLE metadata (
            version INTEGER NOT NULL
        ) STRICT;
        CREATE TABLE graphs (
            file TEXT PRIMARY KEY,
            tag  TEXT NOT NULL,
            json BLOB NOT NULL
        ) STRICT;
        CREATE TABLE file_paths (
            file     TEXT NOT NULL,
            local_id INTEGER NOT NULL,
            json     BLOB NOT NULL,
            FOREIGN KEY(file) REFERENCES graphs(file)
        ) STRICT;
        CREATE TABLE root_paths (
            file         TEXT NOT NULL,
            symbol_stack TEXT NOT NULL,
            json         BLOB NOT NULL,
            FOREIGN KEY(file) REFERENCES graphs(file)
        ) STRICT;
    "#;

const PRAGMAS: &str = r#"
        PRAGMA journal_mode = WAL;
    "#;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("cancelled at {0}")]
    Cancelled(&'static str),
    #[error("unsupported database version {0}")]
    IncorrectVersion(usize),
    #[error("database does not exist {0}")]
    MissingDatabase(String),
    #[error(transparent)]
    Rusqlite(#[from] rusqlite::Error),
    #[error(transparent)]
    Serde(#[from] serde::Error),
    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, StorageError>;

impl From<CancellationError> for StorageError {
    fn from(value: CancellationError) -> Self {
        Self::Cancelled(value.0)
    }
}

/// Writer to store stack graphs and partial paths in a SQLite database.
pub struct SQLiteWriter {
    conn: Connection,
}

impl SQLiteWriter {
    /// Open an in-memory database.
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        Self::init(&conn)?;
        Ok(Self { conn })
    }

    /// Open a file database.  If the file does not exist, it is automatically created.
    /// An error is returned if the database version is not supported.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let is_new = !path.as_ref().exists();
        let conn = Connection::open(path)?;
        set_pragmas_and_functions(&conn)?;
        if is_new {
            Self::init(&conn)?;
        } else {
            check_version(&conn)?;
        }
        Ok(Self { conn })
    }

    /// Create database tables and write metadata.
    fn init(conn: &Connection) -> Result<()> {
        conn.execute("BEGIN", [])?;
        conn.execute_batch(SCHEMA)?;
        conn.execute("INSERT INTO metadata (version) VALUES (?)", [VERSION])?;
        conn.execute("COMMIT", [])?;
        Ok(())
    }

    /// Check if a graph for the file exists in the database.  If a tag is provided, returns true only
    /// if the tag matches.
    pub fn file_exists(&mut self, file: &str, tag: Option<&str>) -> Result<bool> {
        file_exists(&self.conn, file, tag)
    }

    /// Clean all data from the database.
    pub fn clean_all(&mut self) -> Result<usize> {
        self.conn.execute("BEGIN", [])?;
        let count = self.clean_all_inner()?;
        self.conn.execute("COMMIT", [])?;
        Ok(count)
    }

    /// Clean all data from the database.
    fn clean_all_inner(&mut self) -> Result<usize> {
        self.conn.execute("DELETE FROM file_paths", [])?;
        self.conn.execute("DELETE FROM root_paths", [])?;
        let count = self.conn.execute("DELETE FROM graphs", [])?;
        Ok(count)
    }

    /// Clean file data from the database.  If recursive is true, data for all descendants of
    /// that file is cleaned.
    pub fn clean_file(&mut self, file: &Path) -> Result<usize> {
        self.conn.execute("BEGIN", [])?;
        let count = self.clean_file_inner(file)?;
        self.conn.execute("COMMIT", [])?;
        Ok(count)
    }

    /// Clean file data from the database.
    ///
    /// This is an inner method, which does not wrap individual SQL statements in a transaction.
    fn clean_file_inner(&mut self, file: &Path) -> Result<usize> {
        let file = file.to_string_lossy();
        let mut count = 0usize;
        self.conn
            .execute("DELETE FROM file_paths WHERE file=?", [&file])?;
        self.conn
            .execute("DELETE FROM root_paths WHERE file=?", [&file])?;
        count += self
            .conn
            .execute("DELETE FROM graphs WHERE file=?", [&file])?;
        Ok(count)
    }

    /// Clean file or directory data from the database.  Data for all decendants of the given path
    /// is cleaned.
    pub fn clean_file_or_directory(&mut self, file_or_directory: &Path) -> Result<usize> {
        self.conn.execute("BEGIN", [])?;
        let count = self.clean_file_or_directory_inner(file_or_directory)?;
        self.conn.execute("COMMIT", [])?;
        Ok(count)
    }

    /// Clean file or directory data from the database.  Data for all decendants of the given path
    /// is cleaned.
    ///
    /// This is an inner method, which does not wrap individual SQL statements in a transaction.
    fn clean_file_or_directory_inner(&mut self, file_or_directory: &Path) -> Result<usize> {
        let file_or_directory = file_or_directory.to_string_lossy();
        let mut count = 0usize;
        self.conn.execute(
            "DELETE FROM file_paths WHERE path_descendant_of(file, ?)",
            [&file_or_directory],
        )?;
        self.conn.execute(
            "DELETE FROM root_paths WHERE path_descendant_of(file, ?)",
            [&file_or_directory],
        )?;
        count += self.conn.execute(
            "DELETE FROM graphs WHERE path_descendant_of(file, ?)",
            [&file_or_directory],
        )?;
        Ok(count)
    }

    /// Add the stack graph of a file to the database.  If the file already exists, its previous graph
    /// and paths are removed.
    pub fn add_graph_for_file(
        &mut self,
        graph: &StackGraph,
        file: Handle<File>,
        tag: &str,
    ) -> Result<()> {
        let file_str = graph[file].name();
        let mut graph_stmt = self
            .conn
            .prepare_cached("INSERT OR REPLACE INTO graphs (file, tag, json) VALUES (?, ?, ?)")?;
        let mut node_paths_stmt = self
            .conn
            .prepare_cached("DELETE FROM file_paths WHERE file = ?")?;
        let mut file_paths_stmt = self
            .conn
            .prepare_cached("DELETE FROM root_paths WHERE file = ?")?;
        copious_debugging!("--> Add graph for {}", file_str);
        let graph = serde::StackGraph::from_graph_filter(graph, &FileFilter(file));
        self.conn.execute("BEGIN", ())?;
        // insert or update graph
        graph_stmt.execute((file_str, tag, &serde_json::to_vec(&graph)?))?;
        // remove stale file paths
        node_paths_stmt.execute([file_str])?;
        // remove stale file paths
        file_paths_stmt.execute([file_str])?;
        self.conn.execute("COMMIT", ())?;
        Ok(())
    }

    /// Add a partial path for a file to the database.  Returns an error if the file does not exist in
    /// the database.  The start node of `path` must be in `file` or be the root node, otherwise the
    /// method panics.
    pub fn add_partial_path_for_file(
        &mut self,
        graph: &StackGraph,
        file: Handle<File>,
        partials: &mut PartialPaths,
        path: &PartialPath,
    ) -> Result<()> {
        self.add_partial_paths_for_file(graph, file, partials, std::iter::once(path))
    }

    /// Add partial paths for a file to the database.  Panics if the file does not exist in
    /// the database, or if a path starts at a node that doesn't belong to the given file.
    pub fn add_partial_paths_for_file<'a, IP>(
        &mut self,
        graph: &StackGraph,
        file: Handle<File>,
        partials: &mut PartialPaths,
        paths: IP,
    ) -> Result<()>
    where
        IP: IntoIterator<Item = &'a PartialPath>,
    {
        let file_str = graph[file].name();
        self.conn.execute("BEGIN", [])?;
        let mut node_stmt = self
            .conn
            .prepare_cached("INSERT INTO file_paths (file, local_id, json) VALUES (?, ?, ?)")?;
        let mut root_stmt = self
            .conn
            .prepare_cached("INSERT INTO root_paths (file, symbol_stack, json) VALUES (?, ?, ?)")?;
        for path in paths {
            copious_debugging!(
                "--> Add {} partial path {}",
                file_str,
                path.display(graph, partials)
            );
            let start_node = graph[path.start_node].id();
            if start_node.is_in_file(file) {
                copious_debugging!(
                    " * Add as node path from node {}",
                    path.start_node.display(graph),
                );
                let path = serde::PartialPath::from_partial_path(graph, partials, path);
                node_stmt.execute((
                    file_str,
                    path.start_node.local_id,
                    &serde_json::to_vec(&path)?,
                ))?;
            } else if start_node.is_root() {
                copious_debugging!(
                    " * Add as root path with symbol stack {}",
                    path.symbol_stack_precondition.display(graph, partials),
                );
                let symbol_stack = path.symbol_stack_precondition.storage_key(graph, partials);
                let path = serde::PartialPath::from_partial_path(graph, partials, path);
                root_stmt.execute((file_str, symbol_stack, &serde_json::to_vec(&path)?))?;
            } else {
                panic!(
                    "added path {} must start in given file {} or at root",
                    path.display(graph, partials),
                    graph[file].name()
                );
            }
        }
        self.conn.execute("COMMIT", [])?;
        Ok(())
    }

    pub fn into_reader(self) -> SQLiteReader {
        SQLiteReader {
            conn: self.conn,
            loaded_graphs: HashSet::new(),
            loaded_node_paths: HashSet::new(),
            loaded_root_paths: HashSet::new(),
            graph: StackGraph::new(),
            partials: PartialPaths::new(),
            db: Database::new(),
        }
    }
}

/// Reader to load stack graphs and partial paths from a SQLite database.
pub struct SQLiteReader {
    conn: Connection,
    loaded_graphs: HashSet<String>,
    loaded_node_paths: HashSet<Handle<Node>>,
    loaded_root_paths: HashSet<String>,
    graph: StackGraph,
    partials: PartialPaths,
    db: Database,
}

impl SQLiteReader {
    /// Open a file database.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        if !path.as_ref().exists() {
            return Err(StorageError::MissingDatabase(
                path.as_ref().to_string_lossy().to_string(),
            ));
        }
        let conn = Connection::open(path)?;
        set_pragmas_and_functions(&conn)?;
        check_version(&conn)?;
        Ok(Self {
            conn,
            loaded_graphs: HashSet::new(),
            loaded_node_paths: HashSet::new(),
            loaded_root_paths: HashSet::new(),
            graph: StackGraph::new(),
            partials: PartialPaths::new(),
            db: Database::new(),
        })
    }

    /// Check if a graph for the file exists in the database.  If a tag is provided, returns true only
    /// if the tag matches.
    pub fn file_exists(&mut self, file: &str, tag: Option<&str>) -> Result<bool> {
        file_exists(&self.conn, file, tag)
    }

    /// Ensure the graph for the given file is loaded.
    pub fn load_graph_for_file(&mut self, file: &str) -> Result<()> {
        copious_debugging!("--> Load graph for {}", file);
        if !self.loaded_graphs.insert(file.to_string()) {
            copious_debugging!(" * Already loaded");
            return Ok(());
        }
        copious_debugging!(" * Load from database");
        let mut stmt = self
            .conn
            .prepare_cached("SELECT json FROM graphs WHERE file = ?")?;
        let json_graph = stmt.query_row([file], |row| row.get::<_, Vec<u8>>(0))?;
        let graph = serde_json::from_slice::<serde::StackGraph>(&json_graph)?;
        graph.load_into(&mut self.graph)?;
        Ok(())
    }

    /// Ensure the paths starting a the given node are loaded.
    fn load_paths_for_node(&mut self, node: Handle<Node>) -> Result<()> {
        copious_debugging!(" * Load extensions from node {}", node.display(&self.graph));
        if !self.loaded_node_paths.insert(node) {
            copious_debugging!("   > Already loaded");
            return Ok(());
        }
        let file = self.graph[node].file().expect("file node required");
        let file = self.graph[file].name();
        let paths = {
            let mut stmt = self
                .conn
                .prepare_cached("SELECT file,json from file_paths WHERE file = ?")?;
            let paths = stmt
                .query_map([file], |row| {
                    let file = row.get::<_, String>(0)?;
                    let json = row.get::<_, Vec<u8>>(1)?;
                    Ok((file, json))
                })?
                .into_iter()
                .collect::<std::result::Result<Vec<_>, _>>()?;
            paths
        };
        for (file, json) in paths {
            self.load_graph_for_file(&file)?;
            let path = serde_json::from_slice::<serde::PartialPath>(&json)?;
            let path = path.to_partial_path(&mut self.graph, &mut self.partials)?;
            copious_debugging!(
                "   > Loaded {}",
                path.display(&self.graph, &mut self.partials)
            );
            self.db
                .add_partial_path(&self.graph, &mut self.partials, path);
        }
        Ok(())
    }

    /// Ensure the paths starting at the root and matching the given symbol stack are loaded.
    fn load_paths_for_root(&mut self, symbol_stack: PartialSymbolStack) -> Result<()> {
        copious_debugging!(
            " * Load extensions from root with symbol stack {}",
            symbol_stack.display(&self.graph, &mut self.partials)
        );
        let symbol_stack_prefixes =
            symbol_stack.storage_key_prefixes(&self.graph, &mut self.partials);
        for symbol_stack in symbol_stack_prefixes {
            copious_debugging!(
                " * Load extensions from root with prefix symbol stack {}",
                symbol_stack
            );
            if !self.loaded_root_paths.insert(symbol_stack.to_string()) {
                copious_debugging!("   > Already loaded");
                return Ok(());
            }
            let paths = {
                let mut stmt = self
                    .conn
                    .prepare_cached("SELECT file,json from root_paths WHERE symbol_stack = ?")?;
                let paths = stmt
                    .query_map([symbol_stack], |row| {
                        let file = row.get::<_, String>(0)?;
                        let json = row.get::<_, Vec<u8>>(1)?;
                        Ok((file, json))
                    })?
                    .into_iter()
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                paths
            };
            for (file, json) in paths {
                self.load_graph_for_file(&file)?;
                let path = serde_json::from_slice::<serde::PartialPath>(&json)?;
                let path = path.to_partial_path(&mut self.graph, &mut self.partials)?;
                copious_debugging!(
                    "   > Loaded {}",
                    path.display(&self.graph, &mut self.partials)
                );
                self.db
                    .add_partial_path(&self.graph, &mut self.partials, path);
            }
        }
        Ok(())
    }

    /// Ensure all possible extensions for the given partial path are loaded.
    pub fn load_partial_path_extensions(&mut self, path: &PartialPath) -> Result<()> {
        copious_debugging!(
            "--> Load extensions for {}",
            path.display(&self.graph, &mut self.partials)
        );
        let end_node = self.graph[path.end_node].id();
        if self.graph[path.end_node].file().is_some() {
            self.load_paths_for_node(path.end_node)?;
        } else if end_node.is_root() {
            self.load_paths_for_root(path.symbol_stack_postcondition)?;
        }
        Ok(())
    }

    /// Get the stack graph, partial paths arena, and path database for the currently loaded data.
    pub fn get(&mut self) -> (&StackGraph, &mut PartialPaths, &mut Database) {
        (&self.graph, &mut self.partials, &mut self.db)
    }

    /// Find all paths using the given path stitcher.  Data is lazily loaded if necessary.
    pub fn find_all_complete_partial_paths<I, F>(
        &mut self,
        starting_nodes: I,
        cancellation_flag: &dyn CancellationFlag,
        mut visit: F,
    ) -> Result<()>
    where
        I: IntoIterator<Item = Handle<Node>>,
        F: FnMut(&StackGraph, &mut PartialPaths, &PartialPath),
    {
        let mut stitcher =
            ForwardPartialPathStitcher::from_nodes(&self.graph, &mut self.partials, starting_nodes);
        while !stitcher.is_complete() {
            cancellation_flag.check("find_all_complete_partial_paths")?;
            for path in stitcher.previous_phase_partial_paths() {
                self.load_partial_path_extensions(path)?;
            }
            stitcher.process_next_phase(&self.graph, &mut self.partials, &mut self.db);
            for path in stitcher.previous_phase_partial_paths() {
                if path.is_complete(&self.graph) {
                    visit(&self.graph, &mut self.partials, path);
                }
            }
        }
        Ok(())
    }
}

impl PartialSymbolStack {
    /// Returns a string representation of this symbol stack for indexing in the database.
    fn storage_key(mut self, graph: &StackGraph, partials: &mut PartialPaths) -> String {
        let mut key = String::new();
        while let Some(symbol) = self.pop_front(partials) {
            if !key.is_empty() {
                key += "\u{241F}";
            }
            key += &graph[symbol.symbol];
        }
        key
    }

    /// Returns string representations for all prefixes of this symbol stack for querying the
    /// index in the database.
    fn storage_key_prefixes(
        mut self,
        graph: &StackGraph,
        partials: &mut PartialPaths,
    ) -> Vec<String> {
        let mut key_prefixes = vec![String::new()];
        while let Some(symbol) = self.pop_front(partials) {
            let mut key = key_prefixes.last().unwrap().to_string();
            if !key.is_empty() {
                key += "\u{241F}";
            }
            key += &graph[symbol.symbol];
            key_prefixes.push(key);
        }
        key_prefixes
    }
}

/// Check if the database has the version supported by this library version.
fn check_version(conn: &Connection) -> Result<()> {
    let version = conn.query_row("SELECT version FROM metadata", [], |r| r.get::<_, usize>(0))?;
    if version != VERSION {
        return Err(StorageError::IncorrectVersion(version));
    }
    Ok(())
}

fn set_pragmas_and_functions(conn: &Connection) -> Result<()> {
    conn.execute_batch(PRAGMAS)?;
    conn.create_scalar_function(
        "path_descendant_of",
        2,
        FunctionFlags::SQLITE_DETERMINISTIC | FunctionFlags::SQLITE_UTF8,
        move |ctx| {
            assert_eq!(ctx.len(), 2, "called with unexpected number of arguments");
            let path = PathBuf::from(ctx.get::<String>(0)?);
            let parent = PathBuf::from(ctx.get::<String>(1)?);
            let result = path.starts_with(&parent);
            Ok(result)
        },
    )?;
    Ok(())
}

fn file_exists(conn: &Connection, file: &str, tag: Option<&str>) -> Result<bool> {
    let result = if let Some(tag) = tag {
        let mut stmt = conn.prepare_cached("SELECT 1 FROM graphs WHERE file = ? AND tag = ?")?;
        stmt.exists([file, tag])?
    } else {
        let mut stmt = conn.prepare_cached("SELECT 1 FROM graphs WHERE file = ?")?;
        stmt.exists([file])?
    };
    Ok(result)
}
