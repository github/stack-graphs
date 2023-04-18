// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2023, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use clap::Args;
use stack_graphs::storage::SQLiteWriter;
use std::path::PathBuf;
use std::sync::Arc;
use tower_lsp::jsonrpc::Error;
use tower_lsp::jsonrpc::ErrorCode;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::Client;
use tower_lsp::LanguageServer;
use tower_lsp::LspService;
use tower_lsp::Server;

use crate::loader::Loader;

#[derive(Args)]
pub struct LspArgs {}

impl LspArgs {
    pub fn run(self, db_path: PathBuf, loader: Loader) -> anyhow::Result<()> {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            let stdin = tokio::io::stdin();
            let stdout = tokio::io::stdout();
            let (service, socket) = LspService::new(|client| Backend {
                client,
                db_path,
                _loader: Arc::new(std::sync::Mutex::new(loader)),
            });
            Server::new(stdin, stdout, socket).serve(service).await;
        });
        Ok(())
    }
}

struct Backend {
    client: Client,
    db_path: PathBuf,
    _loader: Arc<std::sync::Mutex<Loader>>,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        self.info("Initializing").await;

        let _ = match SQLiteWriter::open(&self.db_path) {
            Ok(db) => {
                self.info(format!("Using database {}", self.db_path.display()))
                    .await;
                db
            }
            Err(err) => {
                self.error(format!(
                    "Failed to open database {}: {}",
                    self.db_path.display(),
                    err,
                ))
                .await;
                return Err(err).from_error();
            }
        };

        if let Some(folders) = params.workspace_folders {
            for folder in &folders {
                self.info(format!("Initial workspace folder {}", folder.uri))
                    .await;
            }
        }

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
        self.info("Initialized").await;
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        self.info(format!("Saved document {}", params.text_document.uri))
            .await;
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        self.info(format!(
            "Goto definition {}:{}:{}",
            params.text_document_position_params.text_document.uri,
            params.text_document_position_params.position.line + 1,
            params.text_document_position_params.position.character + 1
        ))
        .await;
        Ok(None)
    }

    async fn did_change_workspace_folders(&self, params: DidChangeWorkspaceFoldersParams) {
        for folder in &params.event.removed {
            self.info(format!("Removed workspace folder {}", folder.uri))
                .await;
        }
        for folder in &params.event.added {
            self.info(format!("Added workspace folder {}", folder.uri))
                .await;
        }
    }

    async fn shutdown(&self) -> Result<()> {
        self.info("Shutting down").await;
        Ok(())
    }
}

impl Backend {
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
