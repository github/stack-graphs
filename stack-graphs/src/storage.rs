// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2023, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use rusqlite::functions::FunctionFlags;
use rusqlite::types::ValueRef;
use rusqlite::Connection;
use rusqlite::OptionalExtension;
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

const VERSION: usize = 2;

const SCHEMA: &str = r#"
        CREATE TABLE metadata (
            version INTEGER NOT NULL
        ) STRICT;
        CREATE TABLE graphs (
            file   TEXT PRIMARY KEY,
            tag    TEXT NOT NULL,
            error  TEXT,
            json   BLOB NOT NULL
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
        PRAGMA foreign_keys = false;
        PRAGMA secure_delete = false;
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

pub enum FileStatus {
    Missing,
    Indexed,
    Error(String),
}

impl<'a> From<ValueRef<'a>> for FileStatus {
    fn from(value: ValueRef<'a>) -> Self {
        match value {
            ValueRef::Null => Self::Indexed,
            ValueRef::Text(error) => Self::Error(
                std::str::from_utf8(error)
                    .expect("invalid error encoding in database")
                    .to_string(),
            ),
            _ => panic!("invalid value type in database"),
        }
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

    /// Clean all data from the database.
    pub fn clean_all(&mut self) -> Result<usize> {
        self.conn.execute("BEGIN", [])?;
        let count = self.clean_all_inner()?;
        self.conn.execute("COMMIT", [])?;
        Ok(count)
    }

    /// Clean all data from the database.
    ///
    /// This is an inner method, which does not wrap individual SQL statements in a transaction.
    fn clean_all_inner(&mut self) -> Result<usize> {
        {
            let mut stmt = self.conn.prepare_cached("DELETE FROM file_paths")?;
            stmt.execute([])?;
        }
        {
            let mut stmt = self.conn.prepare_cached("DELETE FROM root_paths")?;
            stmt.execute([])?;
        }
        let count = {
            let mut stmt = self.conn.prepare_cached("DELETE FROM graphs")?;
            stmt.execute([])?
        };
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
        {
            let mut stmt = self
                .conn
                .prepare_cached("DELETE FROM file_paths WHERE file=?")?;
            stmt.execute([&file])?;
        }
        {
            let mut stmt = self
                .conn
                .prepare_cached("DELETE FROM root_paths WHERE file=?")?;
            stmt.execute([&file])?;
        }
        let count = {
            let mut stmt = self
                .conn
                .prepare_cached("DELETE FROM graphs WHERE file=?")?;
            stmt.execute([&file])?
        };
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
        {
            let mut stmt = self
                .conn
                .prepare_cached("DELETE FROM file_paths WHERE path_descendant_of(file, ?)")?;
            stmt.execute([&file_or_directory])?;
        }
        {
            let mut stmt = self
                .conn
                .prepare_cached("DELETE FROM root_paths WHERE path_descendant_of(file, ?)")?;
            stmt.execute([&file_or_directory])?;
        }
        let count = {
            let mut stmt = self
                .conn
                .prepare_cached("DELETE FROM graphs WHERE path_descendant_of(file, ?)")?;
            stmt.execute([&file_or_directory])?
        };
        Ok(count)
    }

    /// Store an error, indicating that indexing this file failed.
    pub fn store_error_for_file(&mut self, file: &Path, tag: &str, error: &str) -> Result<()> {
        self.conn.execute("BEGIN", [])?;
        self.store_error_for_file_inner(file, tag, error)?;
        self.conn.execute("COMMIT", [])?;
        Ok(())
    }

    /// Store an error, indicating that indexing this file failed.
    ///
    /// This is an inner method, which does not wrap individual SQL statements in a transaction.
    fn store_error_for_file_inner(&mut self, file: &Path, tag: &str, error: &str) -> Result<()> {
        copious_debugging!("--> Store error for {}", file.display());
        let mut stmt = self
            .conn
            .prepare_cached("INSERT INTO graphs (file, tag, error, json) VALUES (?, ?, ?, ?)")?;
        let graph = crate::serde::StackGraph::default();
        stmt.execute((
            &file.to_string_lossy(),
            tag,
            error,
            &serde_json::to_vec(&graph)?,
        ))?;
        Ok(())
    }

    /// Store the result of a successful file index.
    pub fn store_result_for_file<'a, IP>(
        &mut self,
        graph: &StackGraph,
        file: Handle<File>,
        tag: &str,
        partials: &mut PartialPaths,
        paths: IP,
    ) -> Result<()>
    where
        IP: IntoIterator<Item = &'a PartialPath>,
    {
        let path = Path::new(graph[file].name());
        self.conn.execute("BEGIN", [])?;
        self.clean_file_inner(path)?;
        self.store_graph_for_file_inner(graph, file, tag)?;
        self.store_partial_paths_for_file_inner(graph, file, partials, paths)?;
        self.conn.execute("COMMIT", [])?;
        Ok(())
    }

    /// Store the file graph.
    ///
    /// This is an inner method, which does not wrap individual SQL statements in a transaction.
    fn store_graph_for_file_inner(
        &mut self,
        graph: &StackGraph,
        file: Handle<File>,
        tag: &str,
    ) -> Result<()> {
        let file_str = graph[file].name();
        copious_debugging!("--> Store graph for {}", file_str);
        let mut stmt = self
            .conn
            .prepare_cached("INSERT INTO graphs (file, tag, json) VALUES (?, ?, ?)")?;
        let graph = serde::StackGraph::from_graph_filter(graph, &FileFilter(file));
        stmt.execute((file_str, tag, &serde_json::to_vec(&graph)?))?;
        Ok(())
    }

    /// Store the file partial paths.
    ///
    /// This is an inner method, which does not wrap individual SQL statements in a transaction.
    fn store_partial_paths_for_file_inner<'a, IP>(
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
        Ok(())
    }

    /// Get the file's status in the database. If a tag is provided, it must match or the file
    /// is reported missing.
    pub fn status_for_file(&mut self, file: &str, tag: Option<&str>) -> Result<FileStatus> {
        file_status(&self.conn, file, tag)
    }

    /// Convert this writer into a reader for the same database.
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

    /// Get the file's status in the database. If a tag is provided, it must match or the file
    /// is reported missing.
    pub fn file_status(&mut self, file: &str, tag: Option<&str>) -> Result<FileStatus> {
        file_status(&self.conn, file, tag)
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

fn file_status<'a>(conn: &'a Connection, file: &str, tag: Option<&str>) -> Result<FileStatus> {
    let result = if let Some(tag) = tag {
        let mut stmt =
            conn.prepare_cached("SELECT error FROM graphs WHERE file = ? AND tag = ?")?;
        stmt.query_row([file, tag], |r| r.get_ref(0).map(FileStatus::from))
            .optional()?
            .unwrap_or(FileStatus::Missing)
    } else {
        let mut stmt = conn.prepare_cached("SELECT status FROM graphs WHERE file = ?")?;
        stmt.query_row([file], |r| r.get_ref(0).map(FileStatus::from))
            .optional()?
            .unwrap_or(FileStatus::Missing)
    };
    Ok(result)
}
