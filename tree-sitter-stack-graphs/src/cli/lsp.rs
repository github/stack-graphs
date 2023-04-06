// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2023, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use capture_it::capture;
use clap::Args;
use crossbeam_channel::RecvTimeoutError;
use crossbeam_channel::Sender;
use stack_graphs::storage::SQLiteWriter;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tokio::runtime::Handle;
use tower_lsp::jsonrpc::Error;
use tower_lsp::jsonrpc::ErrorCode;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::Client;
use tower_lsp::LanguageServer;
use tower_lsp::LspService;
use tower_lsp::Server;

use crate::loader::Loader;
use crate::AtomicCancellationFlag;
use crate::CancelAfterDuration;
use crate::CancellationFlag;

use super::index::Indexer;
use super::util::duration_from_seconds_str;
use super::util::FileLogger;
use super::util::Logger;

#[derive(Args, Clone)]
pub struct LspArgs {
    /// Maximum index runtime per workspace folder in seconds.
    #[clap(
        long,
        value_name = "SECONDS",
        parse(try_from_str = duration_from_seconds_str),
    )]
    pub max_folder_index_time: Option<Duration>,

    /// Maximum index runtime per file in seconds.
    #[clap(
        long,
        value_name = "SECONDS",
        parse(try_from_str = duration_from_seconds_str),
    )]
    pub max_file_index_time: Option<Duration>,
}

impl LspArgs {
    pub fn run(self, db_path: PathBuf, loader: Loader) -> anyhow::Result<()> {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            let (service, socket) = LspService::new(|client| Backend {
                _client: client.clone(),
                db_path,
                args: self,
                loader: Arc::new(std::sync::Mutex::new(loader)),
                jobs: Arc::new(tokio::sync::Mutex::new(None)),
                logger: BackendLogger { client },
            });

            let stdin = tokio::io::stdin();
            let stdout = tokio::io::stdout();
            Server::new(stdin, stdout, socket).serve(service).await;
        });
        Ok(())
    }
}

#[derive(Clone)]
struct Backend {
    _client: Client,
    db_path: PathBuf,
    loader: Arc<std::sync::Mutex<Loader>>,
    args: LspArgs,
    jobs: Arc<tokio::sync::Mutex<Option<(Sender<Job>, AtomicCancellationFlag)>>>,
    logger: BackendLogger,
}

impl Backend {
    async fn start_job_handler(&self) -> (Sender<Job>, AtomicCancellationFlag) {
        let handle = Handle::current();
        let backend = self.clone();
        let (sender, receiver) = crossbeam_channel::unbounded::<Job>();
        let cancellation_flag = AtomicCancellationFlag::new();
        let thread_cancellation_flag = cancellation_flag.clone();
        thread::spawn(move || {
            handle.block_on(capture!([logger = &backend.logger], async move {
                logger.info("started job handler").await;
            }));
            loop {
                match receiver.recv_timeout(Duration::from_millis(10)) {
                    Ok(job) => job.run(&backend, handle.clone(), &thread_cancellation_flag),
                    Err(RecvTimeoutError::Timeout) => {
                        if thread_cancellation_flag.check("").is_err() {
                            break;
                        }
                    }
                    Err(RecvTimeoutError::Disconnected) => break,
                }
            }
            handle.block_on(capture!([logger = &backend.logger], async move {
                logger.info("stopped job handler").await;
            }));
        });
        (sender, cancellation_flag)
    }

    fn index(&self, path: &Path, handle: Handle, cancellation_flag: &dyn CancellationFlag) {
        handle.block_on(capture!([logger = &self.logger, path], async move {
            logger.info(format!("indexing {}", path.display())).await;
        }));

        let mut db = match SQLiteWriter::open(&self.db_path) {
            Ok(db) => db,
            Err(err) => {
                handle.block_on(capture!(
                    [logger = &self.logger, db_path = &self.db_path],
                    async move {
                        logger
                            .error(format!(
                                "failed to open database {}: {}",
                                db_path.display(),
                                err
                            ))
                            .await;
                    }
                ));
                return;
            }
        };

        let mut loader = match self.loader.lock() {
            Ok(l) => l,
            Err(e) => {
                handle.block_on(capture!([logger = &self.logger], async move {
                    logger.error(format!("failed to lock loader: {}", e)).await;
                }));
                return;
            }
        };

        let logger = LspLogger {
            handle: handle.clone(),
            logger: self.logger.clone(),
        };
        let folder_cancellation_flag =
            CancelAfterDuration::from_option(self.args.max_folder_index_time);
        let cancellation_flag = cancellation_flag | folder_cancellation_flag.as_ref();
        let mut indexer = Indexer::new(&mut db, &mut loader, &logger);
        indexer.max_file_time = self.args.max_file_index_time;
        let result = indexer.index_all(vec![path], None::<&Path>, &cancellation_flag);

        handle.block_on(capture!([logger = &self.logger, path], async move {
            match result {
                Ok(_) => logger.info(format!("indexed {}", path.display())).await,
                Err(err) => {
                    logger
                        .info(format!("indexing failed {}: {}", path.display(), err))
                        .await
                }
            }
        }));
    }

    fn clean(&self, path: &Path, handle: Handle, _cancellation_flag: &dyn CancellationFlag) {
        handle.block_on(capture!([logger = &self.logger, path], async move {
            logger.info(format!("cleaning {}", path.display())).await;
        }));

        let mut db = match SQLiteWriter::open(&self.db_path) {
            Ok(db) => db,
            Err(err) => {
                handle.block_on(capture!(
                    [logger = &self.logger, db_path = &self.db_path],
                    async move {
                        logger
                            .error(format!(
                                "failed to open database {}: {}",
                                db_path.display(),
                                err
                            ))
                            .await;
                    }
                ));
                return;
            }
        };

        match db.clean(Some(path)) {
            Ok(_) => handle.block_on(capture!([logger = &self.logger, path], async move {
                logger.info(format!("cleaned {}", path.display())).await;
            })),
            Err(e) => handle.block_on(capture!([logger = &self.logger, path], async move {
                logger
                    .error(format!("error cleaning {}: {}", path.display(), e))
                    .await;
            })),
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        self.logger.info("Initializing").await;

        let mut jobs = self.jobs.lock().await;
        *jobs = Some(self.start_job_handler().await);
        if let Some(folders) = params.workspace_folders {
            for folder in &folders {
                self.logger
                    .info(format!("Initial workspace folder {}", folder.uri))
                    .await;
                if let Ok(path) = folder.uri.to_file_path() {
                    jobs.as_ref()
                        .unwrap()
                        .0
                        .send(Job::IndexPath(path))
                        .from_error()?;
                } else {
                    self.logger
                        .error(format!("No local path for workspace folder {}", folder.uri))
                        .await;
                }
            }
        }
        drop(jobs);

        let result = InitializeResult {
            capabilities: ServerCapabilities {
                definition_provider: Some(OneOf::Left(true)),
                text_document_sync: Some(
                    TextDocumentSyncOptions {
                        save: Some(true.into()),
                        ..Default::default()
                    }
                    .into(),
                ),
                workspace: Some(WorkspaceServerCapabilities {
                    workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                        supported: Some(true),
                        change_notifications: Some(OneOf::Left(true)),
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        };
        Ok(result)
    }

    async fn initialized(&self, _: InitializedParams) {
        self.logger.info("Initialized").await;
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let jobs = self.jobs.lock().await;
        self.logger
            .info(format!("Saved document {}", params.text_document.uri))
            .await;
        if let Ok(path) = params.text_document.uri.to_file_path() {
            if let Err(e) = jobs.as_ref().unwrap().0.send(Job::IndexPath(path)) {
                self.logger
                    .error(format!("Scheduling index job failed: {}", e))
                    .await;
            }
        } else {
            self.logger
                .error(format!(
                    "No local path for document {}",
                    params.text_document.uri
                ))
                .await;
        }
        drop(jobs);
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        self.logger
            .info(format!(
                "Go to definition {}:{}:{}",
                params.text_document_position_params.text_document.uri,
                params.text_document_position_params.position.line + 1,
                params.text_document_position_params.position.character + 1
            ))
            .await;
        Ok(None)
    }

    async fn did_change_workspace_folders(&self, params: DidChangeWorkspaceFoldersParams) {
        let jobs = self.jobs.lock().await;
        for folder in &params.event.removed {
            self.logger
                .info(format!("Removed workspace folder {}", folder.uri))
                .await;
            if let Ok(path) = folder.uri.to_file_path() {
                if let Err(e) = jobs.as_ref().unwrap().0.send(Job::CleanPath(path)) {
                    self.logger
                        .error(format!("Scheduling clean job failed: {}", e))
                        .await;
                }
            } else {
                self.logger
                    .error(format!("No local path for workspace folder {}", folder.uri))
                    .await;
            }
        }
        for folder in &params.event.added {
            self.logger
                .info(format!("Added workspace folder {}", folder.uri))
                .await;
            if let Ok(path) = folder.uri.to_file_path() {
                if let Err(e) = jobs.as_ref().unwrap().0.send(Job::IndexPath(path)) {
                    self.logger
                        .error(format!("Scheduling index job failed: {}", e))
                        .await;
                }
            } else {
                self.logger
                    .error(format!("No local path for workspace folder {}", folder.uri))
                    .await;
            }
        }
        drop(jobs);
    }

    async fn shutdown(&self) -> Result<()> {
        self.logger.info("Shutting down").await;
        let jobs = self.jobs.lock().await;
        jobs.as_ref().unwrap().1.cancel();
        drop(jobs);
        Ok(())
    }
}

#[derive(Clone)]
struct BackendLogger {
    client: Client,
}

impl BackendLogger {
    async fn info<M: std::fmt::Display>(&self, message: M) {
        self.client.log_message(MessageType::INFO, message).await
    }

    async fn warning<M: std::fmt::Display>(&self, message: M) {
        self.client.log_message(MessageType::WARNING, message).await
    }

    async fn error<M: std::fmt::Display>(&self, message: M) {
        self.client.log_message(MessageType::ERROR, message).await
    }
}

trait FromStdError<T> {
    #[must_use]
    fn from_error(self) -> Result<T>;
}

impl<T, E: std::error::Error> FromStdError<T> for std::result::Result<T, E> {
    #[must_use]
    fn from_error(self) -> Result<T> {
        match self {
            Ok(value) => Ok(value),
            Err(err) => Err(Error {
                code: ErrorCode::ServerError(-1),
                message: err.to_string(),
                data: None,
            }),
        }
    }
}

trait FromAnyhowError<T> {
    #[must_use]
    fn from_error(self) -> Result<T>;
}

impl<T> FromAnyhowError<T> for std::result::Result<T, anyhow::Error> {
    #[must_use]
    fn from_error(self) -> Result<T> {
        match self {
            Ok(value) => Ok(value),
            Err(err) => Err(Error {
                code: ErrorCode::ServerError(-1),
                message: err.to_string(),
                data: None,
            }),
        }
    }
}

#[derive(Debug)]
pub enum Job {
    IndexPath(PathBuf),
    CleanPath(PathBuf),
}

impl Job {
    fn run(self, backend: &Backend, handle: Handle, cancellation_flag: &dyn CancellationFlag) {
        match self {
            Self::IndexPath(path) => backend.index(&path, handle, cancellation_flag),
            Self::CleanPath(path) => backend.clean(&path, handle, cancellation_flag),
        }
    }
}

struct LspLogger {
    handle: Handle,
    logger: BackendLogger,
}
struct LspFileLogger<'a> {
    path: &'a Path,
    handle: Handle,
    logger: BackendLogger,
}

impl Logger for LspLogger {
    fn file<'a>(&self, path: &'a Path) -> Box<dyn super::util::FileLogger + 'a> {
        Box::new(LspFileLogger {
            path,
            handle: self.handle.clone(),
            logger: self.logger.clone(),
        })
    }
}

impl FileLogger for LspFileLogger<'_> {
    fn default_failure(&mut self, status: &str, _details: Option<&dyn std::fmt::Display>) {
        self.handle.block_on(capture!(
            [logger = &self.logger, path = self.path],
            async move {
                logger
                    .error(format!("{}: {}", path.display(), status))
                    .await;
            }
        ));
    }

    fn failure(&mut self, status: &str, _details: Option<&dyn std::fmt::Display>) {
        self.handle.block_on(capture!(
            [logger = &self.logger, path = self.path],
            async move {
                logger
                    .error(format!("{}: {}", path.display(), status))
                    .await;
            }
        ));
    }

    fn skipped(&mut self, status: &str, _details: Option<&dyn std::fmt::Display>) {
        self.handle.block_on(capture!(
            [logger = &self.logger, path = self.path],
            async move {
                logger
                    .info(format!("{}: skipped: {}", path.display(), status))
                    .await;
            }
        ));
    }

    fn warning(&mut self, status: &str, _details: Option<&dyn std::fmt::Display>) {
        self.handle.block_on(capture!(
            [logger = &self.logger, path = self.path],
            async move {
                logger
                    .warning(format!("{}: {}", path.display(), status))
                    .await;
            }
        ));
    }

    fn success(&mut self, status: &str, _details: Option<&dyn std::fmt::Display>) {
        self.handle.block_on(capture!(
            [logger = &self.logger, path = self.path],
            async move {
                logger
                    .info(format!("{}: success: {}", path.display(), status))
                    .await;
            }
        ));
    }

    fn processing(&mut self) {
        self.handle.block_on(capture!(
            [logger = &self.logger, path = self.path],
            async move {
                logger
                    .info(format!("{}: processing...", path.display()))
                    .await;
            }
        ));
    }
}
