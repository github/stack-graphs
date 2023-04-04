// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2023, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use capture_it::capture;
use clap::Args;
use crossbeam_channel::Sender;
use stack_graphs::storage::SQLiteWriter;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
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

use super::index::Indexer;
use super::util::FileLogger;
use super::util::Logger;

#[derive(Args)]
pub struct LspArgs {}

impl LspArgs {
    pub fn run(self, db_path: PathBuf, loader: Loader) -> anyhow::Result<()> {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            let (service, socket) = LspService::new(|client| Backend {
                _client: client.clone(),
                db_path,
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
    jobs: Arc<tokio::sync::Mutex<Option<Sender<Job>>>>,
    logger: BackendLogger,
}

impl Backend {
    async fn start_job_handler(&self) -> Sender<Job> {
        let handle = Handle::current();
        let backend = self.clone();
        let (sender, receiver) = crossbeam_channel::unbounded::<Job>();
        thread::spawn(move || {
            handle.block_on(capture!([logger = &backend.logger], async move {
                logger.info("started job handler").await;
            }));
            while let Ok(job) = receiver.recv() {
                job.run(&backend, handle.clone());
            }
            handle.block_on(capture!([logger = &backend.logger], async move {
                logger.info("stopped job handler").await;
            }));
        });
        sender
    }

    fn index(&self, path: &Path, handle: Handle) {
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

        let logger = LspLogger {};
        let _indexer = Indexer::new(&mut db, &mut loader, &logger);

        handle.block_on(capture!([logger = &self.logger, path], async move {
            logger.info(format!("indexed {}", path.display())).await;
        }));
    }

    fn clean(&self, path: &Path, handle: Handle) {
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
            if let Err(e) = jobs.as_ref().unwrap().send(Job::IndexPath(path)) {
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
                if let Err(e) = jobs.as_ref().unwrap().send(Job::CleanPath(path)) {
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
                if let Err(e) = jobs.as_ref().unwrap().send(Job::IndexPath(path)) {
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
    fn run(self, backend: &Backend, handle: Handle) {
        match self {
            Self::IndexPath(path) => backend.index(&path, handle),
            Self::CleanPath(path) => backend.clean(&path, handle),
        }
    }
}

struct LspLogger {}
struct LspFileLogger {}

impl Logger for LspLogger {
    fn file<'a>(&self, _path: &'a Path) -> Box<dyn super::util::FileLogger + 'a> {
        Box::new(LspFileLogger {})
    }
}

impl FileLogger for LspFileLogger {}
