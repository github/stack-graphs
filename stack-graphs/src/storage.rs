// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2023, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use rusqlite::Connection;
use rusqlite::OptionalExtension;
use std::collections::HashSet;
use std::path::Path;
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
    /// An error is thrown if the database version is not supported.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let is_new = !path.as_ref().exists();
        let conn = Connection::open(path)?;
        if is_new {
            Self::init(&conn)?;
        } else {
            check_version(&conn)?;
        }
        Ok(Self { conn })
    }

    /// Clean file data from the database.  If a path is given, data for all descendants of
    /// that path is cleaned.  Otherwise, data for all files is cleaned.
    pub fn clean<P: AsRef<Path>>(&mut self, path: Option<P>) -> Result<()> {
        if let Some(path) = path {
            let file = format!("{}%", path.as_ref().to_string_lossy());
            self.conn.execute("BEGIN;", [])?;
            self.conn
                .execute("DELETE FROM file_paths WHERE file LIKE ?", [&file])?;
            self.conn
                .execute("DELETE FROM root_paths WHERE file LIKE ?", [&file])?;
            self.conn
                .execute("DELETE FROM graphs WHERE file LIKE ?", [&file])?;
            self.conn.execute("COMMIT;", [])?;
        } else {
            self.conn.execute_batch(
                r#"
                        BEGIN;
                        DELETE FROM file_paths;
                        DELETE FROM root_paths;
                        DELETE FROM graphs;
                        COMMIT;
                    "#,
            )?;
        }
        Ok(())
    }

    fn init(conn: &Connection) -> Result<()> {
        conn.execute("BEGIN;", [])?;
        conn.execute_batch(SCHEMA)?;
        conn.execute("INSERT INTO metadata (version) VALUES (?)", [VERSION])?;
        conn.execute("COMMIT;", [])?;
        Ok(())
    }

    /// Check if a graph for the file exists in the database.  If a tag is provided, returns true only
    /// if the tag matches.
    pub fn file_exists(&mut self, file: &str, tag: Option<&str>) -> Result<bool> {
        file_exists(&self.conn, file, tag)
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
        let graph = serde::StackGraph::from_graph_filter(graph, &FileFilter(file));
        self.conn.execute("BEGIN;", ())?;
        // insert or update graph
        let mut stmt = self
            .conn
            .prepare_cached("INSERT OR REPLACE INTO graphs (file, tag, json) VALUES (?, ?, ?)")?;
        stmt.execute((file_str, tag, &serde_json::to_vec(&graph)?))?;
        // remove stale file paths
        let mut stmt = self
            .conn
            .prepare_cached("DELETE FROM file_paths WHERE file = ?")?;
        stmt.execute([file_str])?;
        // remove stale file paths
        let mut stmt = self
            .conn
            .prepare_cached("DELETE FROM root_paths WHERE file = ?")?;
        stmt.execute([file_str])?;
        self.conn.execute("END;", ())?;
        Ok(())
    }

    /// Add a partial path for a file to the database.  Throws an error if the file does not exist in
    /// the database.
    pub fn add_partial_path_for_file(
        &mut self,
        graph: &StackGraph,
        partials: &mut PartialPaths,
        path: &PartialPath,
        file: Handle<File>,
    ) -> Result<()> {
        let file_str = graph[file].name();
        let start_node = graph[path.start_node].id();
        if start_node.is_in_file(file) {
            let local_id = start_node.local_id();
            let path = serde::PartialPath::from_partial_path(graph, partials, path);
            let mut stmt = self
                .conn
                .prepare_cached("INSERT INTO file_paths (file, local_id, json) VALUES (?, ?, ?)")?;
            stmt.execute((file_str, local_id, &serde_json::to_vec(&path)?))?;
        } else if start_node.is_root() {
            let symbol_stack = path.symbol_stack_precondition.storage_key(graph, partials);
            let path = serde::PartialPath::from_partial_path(graph, partials, path);
            let mut stmt = self.conn.prepare_cached(
                "INSERT INTO root_paths (file, symbol_stack, json) VALUES (?, ?, ?)",
            )?;
            stmt.execute((file_str, symbol_stack, &serde_json::to_vec(&path)?))?;
        } else {
            todo!();
        }
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
        if !self.loaded_graphs.insert(file.to_string()) {
            return Ok(());
        }
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
        if !self.loaded_node_paths.insert(node) {
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
            self.db
                .add_partial_path(&self.graph, &mut self.partials, path);
        }
        Ok(())
    }

    /// Ensure the paths starting at the root and matching the given symbol stack are loaded.
    fn load_paths_for_root(&mut self, symbol_stack: PartialSymbolStack) -> Result<()> {
        let symbol_stack_prefixes =
            symbol_stack.storage_key_prefixes(&self.graph, &mut self.partials);
        for symbol_stack in symbol_stack_prefixes {
            if !self.loaded_root_paths.insert(symbol_stack.to_string()) {
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
                self.db
                    .add_partial_path(&self.graph, &mut self.partials, path);
            }
        }
        Ok(())
    }

    /// Ensure all possible extensions for the given partial path are loaded.
    pub fn load_partial_path_extensions(&mut self, path: &PartialPath) -> Result<()> {
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
        let mut stitcher = ForwardPartialPathStitcher::from_nodes(
            &self.graph,
            &mut self.partials,
            &mut self.db,
            starting_nodes,
        );
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

fn check_version(conn: &Connection) -> Result<()> {
    let version = conn.query_row("SELECT version FROM metadata", [], |r| r.get::<_, usize>(0))?;
    if version != VERSION {
        return Err(StorageError::IncorrectVersion(version));
    }
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

#[allow(dead_code)]
fn get_file_tag(conn: &Connection, file: &str) -> Result<Option<String>> {
    let mut stmt = conn.prepare_cached("SELECT tag FROM graphs WHERE file = ?")?;
    let tag = stmt
        .query_row([file], |r| r.get::<_, String>(0))
        .optional()?;
    Ok(tag)
}
