// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2023, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use clap::Args;
use crossbeam_channel::{RecvTimeoutError, Sender};
use lsp_server::{Connection, ErrorCode, Message, Notification, Request, RequestId, Response, ResponseError};
use lsp_types::notification::{DidChangeWorkspaceFolders, DidSaveTextDocument, Initialized, Progress};
use lsp_types::request::{GotoDefinition, Initialize, Shutdown};
use lsp_types::*;
use serde_json::Value;
use stack_graphs::storage::{SQLiteReader, SQLiteWriter, StorageError};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::cli::index::Indexer;
use crate::cli::query::Querier;
use crate::cli::query::QueryError;
use crate::cli::util::duration_from_milliseconds_str;
use crate::cli::util::duration_from_seconds_str;
use crate::cli::util::reporter::Reporter;
use crate::cli::util::SourcePosition;
use crate::cli::util::SourceSpan;
use crate::loader::Loader;
use crate::AtomicCancellationFlag;
use crate::CancelAfterDuration;
use crate::CancellationFlag;

/// Command line arguments for the LSP server.
///
/// These arguments control the behavior of the LSP server, such as
/// timeouts for indexing and querying operations.
#[derive(Args, Clone)]
pub struct LspArgs {
    /// Maximum index runtime per workspace folder in seconds.
    ///
    /// If specified, indexing of a workspace folder will be cancelled
    /// after this duration.
    #[clap(
        long,
        value_name = "SECONDS",
        value_parser = duration_from_seconds_str,
    )]
    pub max_folder_index_time: Option<Duration>,

    /// Maximum index runtime per file in seconds.
    ///
    /// If specified, indexing of a single file will be cancelled
    /// after this duration.
    #[clap(
        long,
        value_name = "SECONDS",
        value_parser = duration_from_seconds_str,
    )]
    pub max_file_index_time: Option<Duration>,

    /// Maximum query runtime in milliseconds.
    ///
    /// If specified, a query (e.g., goto-definition) will be cancelled
    /// after this duration.
    #[clap(
        long,
        value_name = "MILLISECONDS",
        value_parser = duration_from_milliseconds_str,
    )]
    pub max_query_time: Option<Duration>,
}

impl LspArgs {
    /// Run the LSP server with the given arguments.
    ///
    /// This method creates and runs an LSP server with the given database path
    /// and loader. It sets up the transport, creates the backend, and runs the
    /// server until it shuts down.
    ///
    /// # Arguments
    ///
    /// * `db_path` - Path to the SQLite database file
    /// * `loader` - Loader for tree-sitter parsers
    ///
    /// # Returns
    ///
    /// * `anyhow::Result<()>` - Result indicating success or failure
    pub fn run(self, db_path: PathBuf, loader: Loader) -> anyhow::Result<()> {
        // Create the transport
        let (connection, io_threads) = Connection::stdio();

        // Create the backend
        let backend = Backend {
            db_path,
            args: self,
            loader: Arc::new(Mutex::new(loader)),
            jobs: Arc::new(Mutex::new(None)),
            client: connection.sender.clone(),
        };

        // Run the server
        backend.run(connection)?;

        // Wait for the IO threads to finish
        io_threads.join()?;
        Ok(())
    }
}

impl std::fmt::Display for LspArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(max_folder_index_time) = self.max_folder_index_time {
            write!(
                f,
                " --max-folder-index-time {}",
                max_folder_index_time.as_secs()
            )?;
        }
        if let Some(max_file_index_time) = self.max_file_index_time {
            write!(
                f,
                " --max-file-index-time {}",
                max_file_index_time.as_secs()
            )?;
        }
        if let Some(max_query_time) = self.max_query_time {
            write!(f, " --max-query-time {}", max_query_time.as_millis())?;
        }
        Ok(())
    }
}

/// Main backend for the LSP server.
///
/// This struct handles the LSP protocol communication and dispatches
/// requests and notifications to the appropriate handlers.
struct Backend {
    /// Path to the SQLite database file
    db_path: PathBuf,
    /// Loader for tree-sitter parsers
    loader: Arc<Mutex<Loader>>,
    /// Command line arguments
    args: LspArgs,
    /// Background job handler and cancellation flag
    jobs: Arc<Mutex<Option<(Sender<Job>, AtomicCancellationFlag)>>>,
    /// Channel for sending messages to the client
    client: Sender<Message>,
}

/// Backend for use in worker threads.
///
/// This is a lightweight version of the Backend struct that can be
/// safely cloned and passed to worker threads. It contains only the
/// data needed for indexing and querying operations.
struct ThreadBackend {
    /// Path to the SQLite database file
    db_path: PathBuf,
    /// Loader for tree-sitter parsers
    loader: Arc<Mutex<Loader>>,
    /// Command line arguments
    args: LspArgs,
}

impl Backend {
    /// Run the LSP server.
    ///
    /// This method starts the main loop of the LSP server, processing
    /// incoming messages from the client and dispatching them to the
    /// appropriate handlers.
    ///
    /// # Arguments
    ///
    /// * `connection` - The LSP connection
    ///
    /// # Returns
    ///
    /// * `anyhow::Result<()>` - Result indicating success or failure
    fn run(mut self, connection: Connection) -> anyhow::Result<()> {
        // Process messages
        log::info!("Starting LSP server");

        for msg in &connection.receiver {
            match msg {
                Message::Request(req) => {
                    if connection.handle_shutdown(&req)? {
                        log::info!("Shutting down LSP server");
                        // Shutdown the job handler
                        if let Ok(mut jobs) = self.jobs.lock() {
                            if let Some((_, flag)) = jobs.as_ref() {
                                flag.cancel();
                            }
                        }
                        return Ok(());
                    }
                    self.handle_request(req);
                }
                Message::Response(resp) => {
                    log::info!("Response: {:?}", resp);
                }
                Message::Notification(not) => {
                    self.handle_notification(not);
                }
            }
        }
        Ok(())
    }

    /// Start a background job handler thread.
    ///
    /// This method creates a new thread that processes indexing and cleaning
    /// jobs in the background. It returns a sender channel for submitting
    /// jobs and a cancellation flag for stopping the thread.
    ///
    /// # Returns
    ///
    /// * `(Sender<Job>, AtomicCancellationFlag)` - A channel for sending jobs and a cancellation flag
    fn start_job_handler(&self) -> (Sender<Job>, AtomicCancellationFlag) {
        let (sender, receiver) = crossbeam_channel::unbounded::<Job>();
        let cancellation_flag = AtomicCancellationFlag::new();
        let thread_cancellation_flag = cancellation_flag.clone();
        let backend = self.clone_for_thread();

        thread::spawn(move || {
            log::info!("Started job handler");
            loop {
                match receiver.recv_timeout(Duration::from_millis(10)) {
                    Ok(job) => job.run(&backend, &thread_cancellation_flag),
                    Err(RecvTimeoutError::Timeout) => {
                        if thread_cancellation_flag.check("").is_err() {
                            break;
                        }
                    }
                    Err(RecvTimeoutError::Disconnected) => break,
                }
            }
            log::info!("Stopped job handler");
        });

        (sender, cancellation_flag)
    }

    /// Create a ThreadBackend for use in worker threads.
    ///
    /// This method creates a lightweight version of the Backend struct
    /// that can be safely cloned and passed to worker threads.
    ///
    /// # Returns
    ///
    /// * `ThreadBackend` - A thread-safe backend for worker threads
    fn clone_for_thread(&self) -> ThreadBackend {
        ThreadBackend {
            db_path: self.db_path.clone(),
            args: self.args.clone(),
            loader: self.loader.clone(),
        }
    }

    /// Opens or creates the database. If the database exists with an incompatible
    /// version, it is recreated.
    fn ensure_compatible_database(&self) -> anyhow::Result<()> {
        match SQLiteWriter::open(&self.db_path) {
            Ok(_) => {}
            Err(StorageError::IncorrectVersion(_)) => {
                log::error!(
                    "Recreating database with new version {}",
                    self.db_path.display()
                );

                std::fs::remove_file(&self.db_path)?;
                SQLiteWriter::open(&self.db_path)?;
            }
            Err(err) => return Err(err.into()),
        };
        Ok(())
    }

    /// Handle an LSP request.
    ///
    /// This method dispatches requests to the appropriate handler based on the method.
    ///
    /// # Arguments
    ///
    /// * `req` - The LSP request
    fn handle_request(&mut self, req: Request) {
        match req.method.as_str() {
            Initialize::METHOD => {
                let params: InitializeParams = match serde_json::from_value(req.params) {
                    Ok(params) => params,
                    Err(err) => {
                        self.send_error_response(req.id, err);
                        return;
                    }
                };

                self.handle_initialize(req.id, params);
            }
            GotoDefinition::METHOD => {
                let params: GotoDefinitionParams = match serde_json::from_value(req.params) {
                    Ok(params) => params,
                    Err(err) => {
                        self.send_error_response(req.id, err);
                        return;
                    }
                };

                self.handle_goto_definition(req.id, params);
            }
            _ => {
                log::warn!("Unhandled request: {}", req.method);
                let result = serde_json::json!({
                    "error": format!("Unhandled request: {}", req.method)
                });
                self.send_response(req.id, result);
            }
        }
    }

    /// Handle an LSP notification.
    ///
    /// This method dispatches notifications to the appropriate handler based on the method.
    ///
    /// # Arguments
    ///
    /// * `not` - The LSP notification
    fn handle_notification(&mut self, not: Notification) {
        match not.method.as_str() {
            Initialized::METHOD => {
                log::info!("Initialized with database {}", self.db_path.display());
            }
            DidSaveTextDocument::METHOD => {
                let params: DidSaveTextDocumentParams = match serde_json::from_value(not.params) {
                    Ok(params) => params,
                    Err(err) => {
                        log::error!("Invalid params for didSave: {}", err);
                        return;
                    }
                };

                self.handle_did_save(params);
            }
            DidChangeWorkspaceFolders::METHOD => {
                let params: DidChangeWorkspaceFoldersParams = match serde_json::from_value(not.params) {
                    Ok(params) => params,
                    Err(err) => {
                        log::error!("Invalid params for didChangeWorkspaceFolders: {}", err);
                        return;
                    }
                };

                self.handle_did_change_workspace_folders(params);
            }
            _ => {
                log::warn!("Unhandled notification: {}", not.method);
            }
        }
    }

    /// Send a successful response to the client.
    ///
    /// # Arguments
    ///
    /// * `id` - The request ID
    /// * `result` - The result value
    fn send_response(&self, id: RequestId, result: Value) {
        let response = Response {
            id,
            result: Some(result),
            error: None,
        };
        self.client.send(Message::Response(response)).unwrap();
    }

    /// Send an error response to the client.
    ///
    /// # Arguments
    ///
    /// * `id` - The request ID
    /// * `err` - The error
    fn send_error_response<E: std::fmt::Display>(&self, id: RequestId, err: E) {
        let response = Response {
            id,
            result: None,
            error: Some(ResponseError {
                code: ErrorCode::InvalidParams as i32,
                message: err.to_string(),
                data: None,
            }),
        };
        self.client.send(Message::Response(response)).unwrap();
    }

    fn handle_initialize(&mut self, id: RequestId, params: InitializeParams) {
        log::info!("Initialize:{}", self.args);

        // Ensure the database is compatible
        if let Err(err) = self.ensure_compatible_database() {
            self.send_error_response(id, err);
            return;
        }

        // Start the job handler
        let mut jobs = self.jobs.lock().unwrap();
        *jobs = Some(self.start_job_handler());

        // Process workspace folders
        if let Some(folders) = params.workspace_folders {
            for folder in &folders {
                log::info!("Initial workspace folder {}", folder.uri);
                if let Ok(path) = folder.uri.to_file_path() {
                    if let Err(e) = jobs.as_ref().unwrap().0.send(Job::IndexPath(path)) {
                        log::error!("Scheduling index job failed: {}", e);
                    }
                } else {
                    log::error!("No local path for workspace folder {}", folder.uri);
                }
            }
        }
        drop(jobs);

        // Send the server capabilities
        let result = serde_json::json!({
            "capabilities": ServerCapabilities {
                definition_provider: Some(OneOf::Right(DefinitionOptions {
                    work_done_progress_options: WorkDoneProgressOptions {
                        work_done_progress: Some(true),
                    },
                })),
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        save: Some(SaveOptions::Supported(true)),
                        ..Default::default()
                    }
                )),
                workspace: Some(WorkspaceServerCapabilities {
                    workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                        supported: Some(true),
                        change_notifications: Some(OneOf::Left(true)),
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            }
        });

        self.send_response(id, result);
    }

    fn handle_goto_definition(&self, id: RequestId, params: GotoDefinitionParams) {
        log::info!(
            "Go to definition {}:{}:{}",
            params.text_document_position_params.text_document.uri,
            params.text_document_position_params.position.line + 1,
            params.text_document_position_params.position.character + 1
        );

        // Send progress notification if requested
        if let Some(token) = &params.work_done_progress_params.work_done_token {
            let progress_params = ProgressParams {
                token: token.clone(),
                value: ProgressParamsValue::WorkDone(WorkDoneProgress::Begin(
                    WorkDoneProgressBegin {
                        title: "Querying".to_string(),
                        ..Default::default()
                    },
                )),
            };

            let notification = Notification {
                method: Progress::METHOD.to_string(),
                params: serde_json::to_value(progress_params).unwrap(),
            };

            self.client.send(Message::Notification(notification)).unwrap();
        }

        // Get the file path
        let path = match params
            .text_document_position_params
            .text_document
            .uri
            .to_file_path()
        {
            Ok(path) => path,
            Err(_) => {
                log::error!(
                    "Not a supported file path: {}",
                    params.text_document_position_params.text_document.uri,
                );
                self.send_response(id, serde_json::json!(null));
                return;
            }
        };

        // Create the source position
        let line = params.text_document_position_params.position.line as usize;
        let column = params.text_document_position_params.position.character as usize;
        let reference = SourcePosition { path, line, column };

        // Find definitions
        let locations = self
            .definitions(reference)
            .into_iter()
            .filter_map(|l| l.try_into_location().ok())
            .collect::<Vec<_>>();

        log::info!(
            "Found {} definitions for {}:{}:{}",
            locations.len(),
            params.text_document_position_params.text_document.uri,
            params.text_document_position_params.position.line + 1,
            params.text_document_position_params.position.character + 1
        );

        // Send progress completion if requested
        if let Some(token) = &params.work_done_progress_params.work_done_token {
            let progress_params = ProgressParams {
                token: token.clone(),
                value: ProgressParamsValue::WorkDone(WorkDoneProgress::End(
                    WorkDoneProgressEnd {
                        ..Default::default()
                    },
                )),
            };

            let notification = Notification {
                method: Progress::METHOD.to_string(),
                params: serde_json::to_value(progress_params).unwrap(),
            };

            self.client.send(Message::Notification(notification)).unwrap();
        }

        // Send the response
        let result = match locations.len() {
            0 => serde_json::json!(null),
            1 => serde_json::to_value(locations[0].clone()).unwrap(),
            _ => serde_json::to_value(locations).unwrap(),
        };

        self.send_response(id, result);
    }

    fn handle_did_save(&self, params: DidSaveTextDocumentParams) {
        let jobs = self.jobs.lock().unwrap();
        log::info!("Saved document {}", params.text_document.uri);

        if let Ok(path) = params.text_document.uri.to_file_path() {
            if let Err(e) = jobs.as_ref().unwrap().0.send(Job::IndexPath(path)) {
                log::error!("Scheduling index job failed: {}", e);
            }
        } else {
            log::error!("No local path for document {}", params.text_document.uri);
        }
    }

    fn handle_did_change_workspace_folders(&self, params: DidChangeWorkspaceFoldersParams) {
        let jobs = self.jobs.lock().unwrap();

        for folder in &params.event.removed {
            log::info!("Removed workspace folder {}", folder.uri);
            if let Ok(path) = folder.uri.to_file_path() {
                if let Err(e) = jobs.as_ref().unwrap().0.send(Job::CleanPath(path)) {
                    log::error!("Scheduling clean job failed: {}", e);
                }
            } else {
                log::error!("No local path for workspace folder {}", folder.uri);
            }
        }

        for folder in &params.event.added {
            log::info!("Added workspace folder {}", folder.uri);
            if let Ok(path) = folder.uri.to_file_path() {
                if let Err(e) = jobs.as_ref().unwrap().0.send(Job::IndexPath(path)) {
                    log::error!("Scheduling index job failed: {}", e);
                }
            } else {
                log::error!("No local path for workspace folder {}", folder.uri);
            }
        }
    }

    fn definitions(&self, reference: SourcePosition) -> Vec<SourceSpan> {
        let mut db = match SQLiteReader::open(&self.db_path) {
            Ok(db) => db,
            Err(err) => {
                log::error!(
                    "failed to open database {}: {}",
                    self.db_path.display(),
                    err
                );
                return Vec::default();
            }
        };

        let reporter = LogReporter {};
        let result = {
            let mut querier = Querier::new(&mut db, &reporter);
            let cancellation_flag = CancelAfterDuration::from_option(self.args.max_query_time);
            querier.definitions(reference, cancellation_flag.as_ref())
        };

        match result {
            Ok(result) => result.into_iter().flat_map(|r| r.targets).collect(),
            Err(QueryError::Cancelled(at)) => {
                log::error!("query timed out at {}", at);
                Vec::default()
            }
            Err(err) => {
                log::error!("query failed {}", err);
                Vec::default()
            }
        }
    }
}

impl ThreadBackend {
    /// Index a file or directory.
    ///
    /// This method indexes the given path and adds the results to the database.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to index
    /// * `cancellation_flag` - Flag to check for cancellation
    fn index(&self, path: &Path, cancellation_flag: &dyn CancellationFlag) {
        log::info!("Indexing {}", path.display());

        let mut db = match SQLiteWriter::open(&self.db_path) {
            Ok(db) => db,
            Err(err) => {
                log::error!("Failed to open database {}: {}", self.db_path.display(), err);
                return;
            }
        };

        let mut loader = match self.loader.lock() {
            Ok(l) => l,
            Err(e) => {
                log::error!("Failed to lock loader: {}", e);
                return;
            }
        };

        let reporter = LogReporter {};
        let folder_cancellation_flag = CancelAfterDuration::from_option(self.args.max_folder_index_time);
        let cancellation_flag = cancellation_flag | folder_cancellation_flag.as_ref();
        let mut indexer = Indexer::new(&mut db, &mut loader, &reporter);
        indexer.max_file_time = self.args.max_file_index_time;
        let result = indexer.index_all(vec![path], None::<&Path>, &cancellation_flag);

        match result {
            Ok(_) => log::info!("Indexed {}", path.display()),
            Err(err) => log::info!("Indexing failed {}: {}", path.display(), err),
        }
    }

    /// Clean a file or directory from the database.
    ///
    /// This method removes all stack graphs for the given path from the database.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to clean
    /// * `_cancellation_flag` - Flag to check for cancellation (unused)
    fn clean(&self, path: &Path, _cancellation_flag: &dyn CancellationFlag) {
        log::info!("Cleaning {}", path.display());

        let mut db = match SQLiteWriter::open(&self.db_path) {
            Ok(db) => db,
            Err(err) => {
                log::error!("Failed to open database {}: {}", self.db_path.display(), err);
                return;
            }
        };

        match db.clean_file_or_directory(path) {
            Ok(_) => log::info!("Cleaned {}", path.display()),
            Err(e) => log::error!("Error cleaning {}: {}", path.display(), e),
        }
    }
}

/// Background job types that can be executed by the job handler.
#[derive(Debug)]
pub enum Job {
    /// Index a file or directory
    IndexPath(PathBuf),
    /// Clean a file or directory from the database
    CleanPath(PathBuf),
}

impl Job {
    /// Run the job with the given backend and cancellation flag.
    ///
    /// # Arguments
    ///
    /// * `backend` - The backend to use for the job
    /// * `cancellation_flag` - Flag to check for cancellation
    fn run(self, backend: &ThreadBackend, cancellation_flag: &dyn CancellationFlag) {
        match self {
            Self::IndexPath(path) => backend.index(&path, cancellation_flag),
            Self::CleanPath(path) => backend.clean(&path, cancellation_flag),
        }
    }
}

/// Reporter implementation that logs messages using the log crate.
///
/// This reporter is used by the indexer and querier to report progress
/// and errors.
struct LogReporter {}

impl Reporter for LogReporter {
    /// Report that a file was skipped.
    fn skipped(&self, path: &Path, summary: &str, _details: Option<&dyn std::fmt::Display>) {
        log::info!("{}: {}", path.display(), summary);
    }

    /// Report that processing of a file has started.
    fn started(&self, path: &Path) {
        log::info!("{}: started", path.display());
    }

    /// Report that processing of a file has succeeded.
    fn succeeded(&self, path: &Path, summary: &str, _details: Option<&dyn std::fmt::Display>) {
        log::info!("{}: {}", path.display(), summary);
    }

    /// Report that processing of a file has failed.
    fn failed(&self, path: &Path, summary: &str, _details: Option<&dyn std::fmt::Display>) {
        log::error!("{}: {}", path.display(), summary);
    }

    /// Report that processing of a file was cancelled.
    fn cancelled(&self, path: &Path, summary: &str, _details: Option<&dyn std::fmt::Display>) {
        log::warn!("{}: {}", path.display(), summary);
    }
}

impl SourceSpan {
    /// Convert a SourceSpan to an LSP Location.
    ///
    /// This method converts a SourceSpan to an LSP Location, which can be
    /// sent to the client for highlighting or navigation.
    ///
    /// # Returns
    ///
    /// * `std::result::Result<Location, ()>` - The converted Location or an error
    fn try_into_location(self) -> std::result::Result<Location, ()> {
        let uri = Url::from_file_path(self.path)?;
        let start = Position {
            line: self.span.start.line as u32,
            character: self.span.start.column.grapheme_offset as u32,
        };
        let end = Position {
            line: self.span.end.line as u32,
            character: self.span.end.column.grapheme_offset as u32,
        };
        let range = Range { start, end };
        Ok(Location { uri, range })
    }
}
