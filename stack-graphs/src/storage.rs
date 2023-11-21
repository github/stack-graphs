// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2023, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use bincode::error::DecodeError;
use bincode::error::EncodeError;
use itertools::Itertools;
use rusqlite::functions::FunctionFlags;
use rusqlite::types::ValueRef;
use rusqlite::Connection;
use rusqlite::OptionalExtension;
use rusqlite::Params;
use rusqlite::Statement;
use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;
use thiserror::Error;

use crate::arena::Handle;
use crate::graph::Degree;
use crate::graph::File;
use crate::graph::Node;
use crate::graph::StackGraph;
use crate::partial::PartialPath;
use crate::partial::PartialPaths;
use crate::partial::PartialSymbolStack;
use crate::serde;
use crate::serde::FileFilter;
use crate::stitching::Database;
use crate::stitching::ForwardCandidates;
use crate::CancellationError;
use crate::CancellationFlag;

const VERSION: usize = 6;

const SCHEMA: &str = r#"
        CREATE TABLE metadata (
            version INTEGER NOT NULL
        ) STRICT;
        CREATE TABLE graphs (
            file   TEXT PRIMARY KEY,
            tag    TEXT NOT NULL,
            error  TEXT,
            value  BLOB NOT NULL
        ) STRICT;
        CREATE TABLE file_paths (
            file     TEXT NOT NULL,
            local_id INTEGER NOT NULL,
            value    BLOB NOT NULL,
            FOREIGN KEY(file) REFERENCES graphs(file)
        ) STRICT;
        CREATE TABLE root_paths (
            file         TEXT NOT NULL,
            symbol_stack TEXT NOT NULL,
            value        BLOB NOT NULL,
            FOREIGN KEY(file) REFERENCES graphs(file)
        ) STRICT;
    "#;

const INDEXES: &str = r#"
        CREATE INDEX IF NOT EXISTS idx_graphs_file ON graphs(file);
        CREATE INDEX IF NOT EXISTS idx_file_paths_local_id ON file_paths(file, local_id);
        CREATE INDEX IF NOT EXISTS idx_root_paths_symbol_stack ON root_paths(symbol_stack);
    "#;

const PRAGMAS: &str = r#"
        PRAGMA journal_mode = WAL;
        PRAGMA foreign_keys = false;
        PRAGMA secure_delete = false;
    "#;

pub static BINCODE_CONFIG: bincode::config::Configuration = bincode::config::standard();

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
    SerializeFail(#[from] EncodeError),
    #[error(transparent)]
    DeserializeFail(#[from] DecodeError),
}

pub type Result<T> = std::result::Result<T, StorageError>;

impl From<CancellationError> for StorageError {
    fn from(value: CancellationError) -> Self {
        Self::Cancelled(value.0)
    }
}

/// The status of a file in the database.
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

/// A file entry in the database.
pub struct FileEntry {
    pub path: PathBuf,
    pub tag: String,
    pub status: FileStatus,
}

/// An iterator over a query returning rows with (path,tag,error) tuples.
pub struct Files<'a, P: Params>(Statement<'a>, P);

impl<'a, P: Params + Clone> Files<'a, P> {
    pub fn try_iter<'b>(&'b mut self) -> Result<impl Iterator<Item = Result<FileEntry>> + 'b> {
        let entries = self.0.query_map(self.1.clone(), |r| {
            Ok(FileEntry {
                path: PathBuf::from(r.get::<_, String>(0)?),
                tag: r.get::<_, String>(1)?,
                status: r.get_ref(2)?.into(),
            })
        })?;
        let entries = entries.map(|r| -> Result<FileEntry> { Ok(r?) });
        Ok(entries)
    }
}

/// Writer to store stack graphs and partial paths in a SQLite database.
pub struct SQLiteWriter {
    conn: Connection,
}

impl SQLiteWriter {
    /// Open an in-memory database.
    pub fn open_in_memory() -> Result<Self> {
        let mut conn = Connection::open_in_memory()?;
        Self::init(&mut conn)?;
        init_indexes(&mut conn)?;
        Ok(Self { conn })
    }

    /// Open a file database.  If the file does not exist, it is automatically created.
    /// An error is returned if the database version is not supported.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let is_new = !path.as_ref().exists();
        let mut conn = Connection::open(path)?;
        set_pragmas_and_functions(&conn)?;
        if is_new {
            Self::init(&mut conn)?;
        } else {
            check_version(&conn)?;
        }
        init_indexes(&mut conn)?;
        Ok(Self { conn })
    }

    /// Create database tables and write metadata.
    fn init(conn: &mut Connection) -> Result<()> {
        let tx = conn.transaction()?;
        tx.execute_batch(SCHEMA)?;
        tx.execute("INSERT INTO metadata (version) VALUES (?)", [VERSION])?;
        tx.commit()?;
        Ok(())
    }

    /// Clean all data from the database.
    pub fn clean_all(&mut self) -> Result<usize> {
        let tx = self.conn.transaction()?;
        let count = Self::clean_all_inner(&tx)?;
        tx.commit()?;
        Ok(count)
    }

    /// Clean all data from the database.
    ///
    /// This is an inner method, which does not wrap individual SQL statements in a transaction.
    fn clean_all_inner(conn: &Connection) -> Result<usize> {
        {
            let mut stmt = conn.prepare_cached("DELETE FROM file_paths")?;
            stmt.execute([])?;
        }
        {
            let mut stmt = conn.prepare_cached("DELETE FROM root_paths")?;
            stmt.execute([])?;
        }
        let count = {
            let mut stmt = conn.prepare_cached("DELETE FROM graphs")?;
            stmt.execute([])?
        };
        Ok(count)
    }

    /// Clean file data from the database.  If recursive is true, data for all descendants of
    /// that file is cleaned.
    pub fn clean_file(&mut self, file: &Path) -> Result<usize> {
        let tx = self.conn.transaction()?;
        let count = Self::clean_file_inner(&tx, file)?;
        tx.commit()?;
        Ok(count)
    }

    /// Clean file data from the database.
    ///
    /// This is an inner method, which does not wrap individual SQL statements in a transaction.
    fn clean_file_inner(conn: &Connection, file: &Path) -> Result<usize> {
        let file = file.to_string_lossy();
        {
            let mut stmt = conn.prepare_cached("DELETE FROM file_paths WHERE file=?")?;
            stmt.execute([&file])?;
        }
        {
            let mut stmt = conn.prepare_cached("DELETE FROM root_paths WHERE file=?")?;
            stmt.execute([&file])?;
        }
        let count = {
            let mut stmt = conn.prepare_cached("DELETE FROM graphs WHERE file=?")?;
            stmt.execute([&file])?
        };
        Ok(count)
    }

    /// Clean file or directory data from the database.  Data for all decendants of the given path
    /// is cleaned.
    pub fn clean_file_or_directory(&mut self, file_or_directory: &Path) -> Result<usize> {
        let tx = self.conn.transaction()?;
        let count = Self::clean_file_or_directory_inner(&tx, file_or_directory)?;
        tx.commit()?;
        Ok(count)
    }

    /// Clean file or directory data from the database.  Data for all decendants of the given path
    /// is cleaned.
    ///
    /// This is an inner method, which does not wrap individual SQL statements in a transaction.
    fn clean_file_or_directory_inner(conn: &Connection, file_or_directory: &Path) -> Result<usize> {
        let file_or_directory = file_or_directory.to_string_lossy();
        {
            let mut stmt =
                conn.prepare_cached("DELETE FROM file_paths WHERE path_descendant_of(file, ?)")?;
            stmt.execute([&file_or_directory])?;
        }
        {
            let mut stmt =
                conn.prepare_cached("DELETE FROM root_paths WHERE path_descendant_of(file, ?)")?;
            stmt.execute([&file_or_directory])?;
        }
        let count = {
            let mut stmt =
                conn.prepare_cached("DELETE FROM graphs WHERE path_descendant_of(file, ?)")?;
            stmt.execute([&file_or_directory])?
        };
        Ok(count)
    }

    /// Store an error, indicating that indexing this file failed.
    pub fn store_error_for_file(&mut self, file: &Path, tag: &str, error: &str) -> Result<()> {
        let tx = self.conn.transaction()?;
        Self::store_error_for_file_inner(&tx, file, tag, error)?;
        tx.commit()?;
        Ok(())
    }

    /// Store an error, indicating that indexing this file failed.
    ///
    /// This is an inner method, which does not wrap individual SQL statements in a transaction.
    fn store_error_for_file_inner(
        conn: &Connection,
        file: &Path,
        tag: &str,
        error: &str,
    ) -> Result<()> {
        copious_debugging!("--> Store error for {}", file.display());
        let mut stmt = conn
            .prepare_cached("INSERT INTO graphs (file, tag, error, value) VALUES (?, ?, ?, ?)")?;
        let graph = crate::serde::StackGraph::default();
        let serialized = bincode::encode_to_vec(&graph, BINCODE_CONFIG)?;
        stmt.execute((&file.to_string_lossy(), tag, error, serialized))?;
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
        let tx = self.conn.transaction()?;
        Self::clean_file_inner(&tx, path)?;
        Self::store_graph_for_file_inner(&tx, graph, file, tag)?;
        Self::store_partial_paths_for_file_inner(&tx, graph, file, partials, paths)?;
        tx.commit()?;
        Ok(())
    }

    /// Store the file graph.
    ///
    /// This is an inner method, which does not wrap individual SQL statements in a transaction.
    fn store_graph_for_file_inner(
        conn: &Connection,
        graph: &StackGraph,
        file: Handle<File>,
        tag: &str,
    ) -> Result<()> {
        let file_str = graph[file].name();
        copious_debugging!("--> Store graph for {}", file_str);
        let mut stmt =
            conn.prepare_cached("INSERT INTO graphs (file, tag, value) VALUES (?, ?, ?)")?;
        let graph = serde::StackGraph::from_graph_filter(graph, &FileFilter(file));
        let serialized = bincode::encode_to_vec(&graph, BINCODE_CONFIG)?;
        stmt.execute((file_str, tag, &serialized))?;
        Ok(())
    }

    /// Store the file partial paths.
    ///
    /// This is an inner method, which does not wrap individual SQL statements in a transaction.
    fn store_partial_paths_for_file_inner<'a, IP>(
        conn: &Connection,
        graph: &StackGraph,
        file: Handle<File>,
        partials: &mut PartialPaths,
        paths: IP,
    ) -> Result<()>
    where
        IP: IntoIterator<Item = &'a PartialPath>,
    {
        let file_str = graph[file].name();
        let mut node_stmt =
            conn.prepare_cached("INSERT INTO file_paths (file, local_id, value) VALUES (?, ?, ?)")?;
        let mut root_stmt = conn.prepare_cached(
            "INSERT INTO root_paths (file, symbol_stack, value) VALUES (?, ?, ?)",
        )?;
        #[cfg_attr(not(feature = "copious-debugging"), allow(unused))]
        let mut node_path_count = 0usize;
        #[cfg_attr(not(feature = "copious-debugging"), allow(unused))]
        let mut root_path_count = 0usize;
        for path in paths {
            copious_debugging!(
                "--> Add {} partial path {}",
                file_str,
                path.display(graph, partials)
            );
            let start_node = graph[path.start_node].id();
            if start_node.is_root() {
                copious_debugging!(
                    " * Add as root path with symbol stack {}",
                    path.symbol_stack_precondition.display(graph, partials),
                );
                let symbol_stack = path.symbol_stack_precondition.storage_key(graph, partials);
                let path = serde::PartialPath::from_partial_path(graph, partials, path);
                let serialized = bincode::encode_to_vec(&path, BINCODE_CONFIG)?;
                root_stmt.execute((file_str, symbol_stack, serialized))?;
                root_path_count += 1;
            } else if start_node.is_in_file(file) {
                copious_debugging!(
                    " * Add as node path from node {}",
                    path.start_node.display(graph),
                );
                let path = serde::PartialPath::from_partial_path(graph, partials, path);
                let serialized = bincode::encode_to_vec(&path, BINCODE_CONFIG)?;
                node_stmt.execute((file_str, path.start_node.local_id, serialized))?;
                node_path_count += 1;
            } else {
                panic!(
                    "added path {} must start in given file {} or at root",
                    path.display(graph, partials),
                    graph[file].name()
                );
            }
            copious_debugging!(
                " * Added {} node paths and {} root paths",
                node_path_count,
                root_path_count,
            );
        }
        Ok(())
    }

    /// Get the file's status in the database. If a tag is provided, it must match or the file
    /// is reported missing.
    pub fn status_for_file(&mut self, file: &str, tag: Option<&str>) -> Result<FileStatus> {
        status_for_file(&self.conn, file, tag)
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
            stats: Stats::default(),
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
    stats: Stats,
}

impl SQLiteReader {
    /// Open a file database.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        if !path.as_ref().exists() {
            return Err(StorageError::MissingDatabase(
                path.as_ref().to_string_lossy().to_string(),
            ));
        }
        let mut conn = Connection::open(path)?;
        set_pragmas_and_functions(&conn)?;
        check_version(&conn)?;
        init_indexes(&mut conn)?;
        Ok(Self {
            conn,
            loaded_graphs: HashSet::new(),
            loaded_node_paths: HashSet::new(),
            loaded_root_paths: HashSet::new(),
            graph: StackGraph::new(),
            partials: PartialPaths::new(),
            db: Database::new(),
            stats: Stats::default(),
        })
    }

    /// Clear all data that has been loaded into this reader instance.
    /// After this call, all existing handles from this reader are invalid.
    pub fn clear(&mut self) {
        self.loaded_graphs.clear();
        self.graph = StackGraph::new();

        self.loaded_node_paths.clear();
        self.loaded_root_paths.clear();
        self.partials.clear();
        self.db.clear();

        self.stats.clear();
    }

    /// Clear path data that has been loaded into this reader instance.
    /// After this call, all node handles remain valid, but all path data
    /// is invalid.
    pub fn clear_paths(&mut self) {
        self.loaded_node_paths.clear();
        self.loaded_root_paths.clear();
        self.partials.clear();
        self.db.clear();

        self.stats.clear_paths();
    }

    /// Get the file's status in the database. If a tag is provided, it must match or the file
    /// is reported missing.
    pub fn status_for_file<T: AsRef<str>>(
        &mut self,
        file: &str,
        tag: Option<T>,
    ) -> Result<FileStatus> {
        status_for_file(&self.conn, file, tag)
    }

    /// Returns a [`Files`][] value that can be used to iterate over all files in the database.
    pub fn list_all<'a>(&'a mut self) -> Result<Files<'a, ()>> {
        self.conn
            .prepare("SELECT file, tag, error FROM graphs")
            .map(|stmt| Files(stmt, ()))
            .map_err(|e| e.into())
    }

    /// Returns a [`Files`][] value that can be used to iterate over all descendants of a
    /// file or directory in the database.
    pub fn list_file_or_directory<'a>(
        &'a self,
        file_or_directory: &Path,
    ) -> Result<Files<'a, [String; 1]>> {
        Self::list_file_or_directory_inner(&self.conn, file_or_directory)
    }

    fn list_file_or_directory_inner<'a>(
        conn: &'a Connection,
        file_or_directory: &Path,
    ) -> Result<Files<'a, [String; 1]>> {
        let file_or_directory = file_or_directory.to_string_lossy().to_string();
        conn.prepare("SELECT file, tag, error FROM graphs WHERE path_descendant_of(file, ?)")
            .map(|stmt| Files(stmt, [file_or_directory]))
            .map_err(|e| e.into())
    }

    /// Ensure the graph for the given file is loaded.
    pub fn load_graph_for_file(&mut self, file: &str) -> Result<Handle<File>> {
        Self::load_graph_for_file_inner(
            file,
            &mut self.graph,
            &mut self.loaded_graphs,
            &self.conn,
            &mut self.stats,
        )
    }

    fn load_graph_for_file_inner(
        file: &str,
        graph: &mut StackGraph,
        loaded_graphs: &mut HashSet<String>,
        conn: &Connection,
        stats: &mut Stats,
    ) -> Result<Handle<File>> {
        copious_debugging!("--> Load graph for {}", file);
        if !loaded_graphs.insert(file.to_string()) {
            copious_debugging!(" * Already loaded");
            stats.file_cached += 1;
            return Ok(graph.get_file(file).expect("loaded file to exist"));
        }
        copious_debugging!(" * Load from database");
        stats.file_loads += 1;
        let mut stmt = conn.prepare_cached("SELECT value FROM graphs WHERE file = ?")?;
        let value = stmt.query_row([file], |row| row.get::<_, Vec<u8>>(0))?;
        let (file_graph, _): (serde::StackGraph, usize) =
            bincode::decode_from_slice(&value, BINCODE_CONFIG)?;
        file_graph.load_into(graph)?;
        Ok(graph.get_file(file).expect("loaded file to exist"))
    }

    pub fn load_graphs_for_file_or_directory(
        &mut self,
        file_or_directory: &Path,
        cancellation_flag: &dyn CancellationFlag,
    ) -> Result<()> {
        for file in Self::list_file_or_directory_inner(&self.conn, file_or_directory)?.try_iter()? {
            cancellation_flag.check("loading graphs")?;
            let file = file?;
            Self::load_graph_for_file_inner(
                &file.path.to_string_lossy(),
                &mut self.graph,
                &mut self.loaded_graphs,
                &self.conn,
                &mut self.stats,
            )?;
        }
        Ok(())
    }

    /// Ensure the paths starting a the given node are loaded.
    fn load_paths_for_node(
        &mut self,
        node: Handle<Node>,
        cancellation_flag: &dyn CancellationFlag,
    ) -> Result<()> {
        copious_debugging!(" * Load extensions from node {}", node.display(&self.graph));
        if !self.loaded_node_paths.insert(node) {
            copious_debugging!("   > Already loaded");
            self.stats.node_path_cached += 1;
            return Ok(());
        }
        self.stats.node_path_loads += 1;
        let id = self.graph[node].id();
        let file = id.file().expect("file node required");
        let file = self.graph[file].name();
        let mut stmt = self
            .conn
            .prepare_cached("SELECT file,value from file_paths WHERE file = ? AND local_id = ?")?;
        let paths = stmt.query_map((file, id.local_id()), |row| {
            let file = row.get::<_, String>(0)?;
            let value = row.get::<_, Vec<u8>>(1)?;
            Ok((file, value))
        })?;
        #[cfg_attr(not(feature = "copious-debugging"), allow(unused))]
        let mut count = 0usize;
        for path in paths {
            cancellation_flag.check("loading node paths")?;
            let (file, value) = path?;
            Self::load_graph_for_file_inner(
                &file,
                &mut self.graph,
                &mut self.loaded_graphs,
                &self.conn,
                &mut self.stats,
            )?;
            let (path, _): (serde::PartialPath, usize) =
                bincode::decode_from_slice(&value, BINCODE_CONFIG)?;
            let path = path.to_partial_path(&mut self.graph, &mut self.partials)?;
            copious_debugging!(
                "   > Loaded {}",
                path.display(&self.graph, &mut self.partials)
            );
            self.db
                .add_partial_path(&self.graph, &mut self.partials, path);
            count += 1;
        }
        copious_debugging!("   > Loaded {}", count);
        Ok(())
    }

    /// Ensure the paths starting at the root and matching the given symbol stack are loaded.
    fn load_paths_for_root(
        &mut self,
        symbol_stack: PartialSymbolStack,
        cancellation_flag: &dyn CancellationFlag,
    ) -> Result<()> {
        copious_debugging!(
            " * Load extensions from root with symbol stack {}",
            symbol_stack.display(&self.graph, &mut self.partials)
        );
        let mut stmt = self.conn.prepare_cached(
            "SELECT file,value from root_paths WHERE symbol_stack LIKE ? ESCAPE ?",
        )?;
        let (symbol_stack_patterns, escape) =
            symbol_stack.storage_key_patterns(&self.graph, &mut self.partials);
        for symbol_stack in symbol_stack_patterns {
            copious_debugging!(
                " * Load extensions from root with prefix symbol stack {}",
                symbol_stack
            );
            if !self.loaded_root_paths.insert(symbol_stack.clone()) {
                copious_debugging!("   > Already loaded");
                self.stats.root_path_cached += 1;
                continue;
            }
            self.stats.root_path_loads += 1;
            let paths = stmt.query_map([symbol_stack, escape.clone()], |row| {
                let file = row.get::<_, String>(0)?;
                let value = row.get::<_, Vec<u8>>(1)?;
                Ok((file, value))
            })?;
            #[cfg_attr(not(feature = "copious-debugging"), allow(unused))]
            let mut count = 0usize;
            for path in paths {
                cancellation_flag.check("loading root paths")?;
                let (file, value) = path?;
                Self::load_graph_for_file_inner(
                    &file,
                    &mut self.graph,
                    &mut self.loaded_graphs,
                    &self.conn,
                    &mut self.stats,
                )?;
                let (path, _): (serde::PartialPath, usize) =
                    bincode::decode_from_slice(&value, BINCODE_CONFIG)?;
                let path = path.to_partial_path(&mut self.graph, &mut self.partials)?;
                copious_debugging!(
                    "   > Loaded {}",
                    path.display(&self.graph, &mut self.partials)
                );
                self.db
                    .add_partial_path(&self.graph, &mut self.partials, path);
                count += 1;
            }
            copious_debugging!("   > Loaded {}", count);
        }
        Ok(())
    }

    /// Ensure all possible extensions for the given partial path are loaded.
    pub fn load_partial_path_extensions(
        &mut self,
        path: &PartialPath,
        cancellation_flag: &dyn CancellationFlag,
    ) -> Result<()> {
        copious_debugging!(
            "--> Load extensions for {}",
            path.display(&self.graph, &mut self.partials)
        );
        let end_node = self.graph[path.end_node].id();
        if self.graph[path.end_node].file().is_some() {
            self.load_paths_for_node(path.end_node, cancellation_flag)?;
        } else if end_node.is_root() {
            self.load_paths_for_root(path.symbol_stack_postcondition, cancellation_flag)?;
        }
        Ok(())
    }

    /// Get the stack graph, partial paths arena, and path database for the currently loaded data.
    pub fn get(&mut self) -> (&mut StackGraph, &mut PartialPaths, &mut Database) {
        (&mut self.graph, &mut self.partials, &mut self.db)
    }

    /// Return stats about this database reader.
    pub fn stats(&self) -> Stats {
        self.stats.clone()
    }
}

// Methods for computing keys and patterns for a symbol stack. The format of a storage key is:
//
//     has-var GS ( symbol (US symbol)* )?
//
// where has-var is "V" if the symbol stack has a variable, "X" otherwise.
impl PartialSymbolStack {
    /// Returns a string representation of this symbol stack for indexing in the database.
    fn storage_key(self, graph: &StackGraph, partials: &mut PartialPaths) -> String {
        let mut key = String::new();
        match self.has_variable() {
            true => key += "V\u{241E}",
            false => key += "X\u{241E}",
        }
        key += &self
            .iter(partials)
            .map(|s| &graph[s.symbol])
            .join("\u{241F}");
        key
    }

    /// Returns string representations for all prefixes of this symbol stack for querying the
    /// index in the database.
    fn storage_key_patterns(
        mut self,
        graph: &StackGraph,
        partials: &mut PartialPaths,
    ) -> (Vec<String>, String) {
        let mut key_patterns = Vec::new();
        let mut symbols = String::new();
        while let Some(symbol) = self.pop_front(partials) {
            if !symbols.is_empty() {
                symbols += "\u{241F}";
            }
            let symbol = graph[symbol.symbol]
                .replace("%", "\\%")
                .replace("_", "\\_")
                .to_string();
            symbols += &symbol;
            // patterns for paths matching a prefix of this stack
            key_patterns.push("V\u{241E}".to_string() + &symbols);
        }
        // pattern for paths matching exactly this stack
        key_patterns.push("X\u{241E}".to_string() + &symbols);
        if self.has_variable() {
            // patterns for paths for which this stack is a prefix
            key_patterns.push("_\u{241E}".to_string() + &symbols + "\u{241F}%");
        }
        (key_patterns, "\\".to_string())
    }
}

impl ForwardCandidates<Handle<PartialPath>, PartialPath, Database, StorageError> for SQLiteReader {
    fn load_forward_candidates(
        &mut self,
        path: &PartialPath,
        cancellation_flag: &dyn CancellationFlag,
    ) -> std::result::Result<(), StorageError> {
        self.load_partial_path_extensions(path, cancellation_flag)
    }

    fn get_forward_candidates<R>(&mut self, path: &PartialPath, result: &mut R)
    where
        R: std::iter::Extend<Handle<PartialPath>>,
    {
        self.db
            .find_candidate_partial_paths(&self.graph, &mut self.partials, path, result);
    }

    fn get_joining_candidate_degree(&self, path: &PartialPath) -> Degree {
        self.db.get_incoming_path_degree(path.end_node)
    }

    fn get_graph_partials_and_db(&mut self) -> (&StackGraph, &mut PartialPaths, &Database) {
        (&self.graph, &mut self.partials, &self.db)
    }
}

#[derive(Clone, Debug, Default)]
pub struct Stats {
    pub file_loads: usize,
    pub file_cached: usize,
    pub root_path_loads: usize,
    pub root_path_cached: usize,
    pub node_path_loads: usize,
    pub node_path_cached: usize,
}

impl Stats {
    fn clear(&mut self) {
        *self = Stats::default();
    }

    fn clear_paths(&mut self) {
        *self = Stats {
            file_loads: self.file_loads,
            file_cached: self.file_cached,
            ..Stats::default()
        }
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

fn init_indexes(conn: &mut Connection) -> Result<()> {
    let tx = conn.transaction()?;
    tx.execute_batch(INDEXES)?;
    tx.commit()?;
    Ok(())
}

fn status_for_file<T: AsRef<str>>(
    conn: &Connection,
    file: &str,
    tag: Option<T>,
) -> Result<FileStatus> {
    let result = if let Some(tag) = tag {
        let mut stmt =
            conn.prepare_cached("SELECT error FROM graphs WHERE file = ? AND tag = ?")?;
        stmt.query_row([file, tag.as_ref()], |r| r.get_ref(0).map(FileStatus::from))
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
