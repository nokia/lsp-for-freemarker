// Copyright 2025-2026 Nokia
// Licensed under the BSD 3-Clause License.
// SPDX-License-Identifier: BSD-3-Clause

use std::{str::FromStr, sync::Arc};

use moka::future::Cache;
use tokio::sync::RwLock;
use tower_lsp_server::{
    Client, LanguageServer, jsonrpc,
    ls_types::{
        CodeActionOrCommand, CodeActionParams, CompletionParams, CompletionResponse,
        DeleteFilesParams, DidChangeTextDocumentParams, DidChangeWatchedFilesParams,
        DidCloseTextDocumentParams, DidOpenTextDocumentParams, DocumentDiagnosticParams,
        DocumentDiagnosticReportResult, DocumentFormattingParams, FileChangeType, FoldingRange,
        FoldingRangeParams, GotoDefinitionParams, GotoDefinitionResponse, Hover, HoverParams,
        InitializeParams, InitializeResult, InitializedParams, SemanticTokensParams,
        SemanticTokensResult, TextEdit, Uri,
    },
};

use crate::{
    client::{self, Window},
    document::Document,
    features::{
        ActionFeature, CompletionFeature, DiagnosticFeature, FoldingFeature, FormatFeature,
        GotoFeature, HoverFeature, SemanticTokenFeature,
    },
    info, init, warning,
};

#[derive(Debug)]
pub struct Server {
    documents: Cache<String, Arc<RwLock<Document>>>,
}

impl Server {
    pub const NAME: &str = "Freemarker Language Server";

    pub fn new(client: Client) -> Self {
        client::init(client);
        Self {
            documents: Cache::new(64),
        }
    }

    /// Runs `func` against the document for `uri`, holding a read lock for the
    /// duration of the call. Returns an internal error when the document is
    /// not open.
    async fn with_document<T>(
        &self,
        uri: &Uri,
        func: impl AsyncFnOnce(&Document) -> jsonrpc::Result<T>,
    ) -> jsonrpc::Result<T> {
        let Some(document) = self.documents.get(&uri.to_string()).await else {
            return Err(jsonrpc::Error::internal_error());
        };
        let guard = document.read().await;
        func(&guard).await
    }
}

impl LanguageServer for Server {
    async fn initialize(&self, _: InitializeParams) -> jsonrpc::Result<InitializeResult> {
        Window::log(info!("[Server] initializing...")).await;
        Ok(init::do_initialize())
    }

    async fn initialized(&self, _: InitializedParams) {
        Window::log(info!("[Server] initialized.")).await;
    }

    async fn shutdown(&self) -> jsonrpc::Result<()> {
        Window::log(info!("[Server] shutdown :)")).await;
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let version = params.text_document.version;
        Window::log(info!(format!("on_did_open: {:?}", uri.to_string()))).await;
        Window::log(info!(format!("document version: {:?}", version))).await;
        self.documents
            .insert(
                uri.to_string(),
                Arc::new(RwLock::new(Document::open(
                    &uri,
                    params.text_document.text.as_str(),
                    version,
                ))),
            )
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let version = params.text_document.version;
        Window::log(info!(format!("on_did_change: {}", uri.to_string()))).await;
        let Some(document) = self.documents.get(&uri.to_string()).await else {
            return;
        };
        let mut guard = document.write().await;
        for change in &params.content_changes {
            guard.apply_change(version, change);
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        Window::log(info!(format!("did_close: {:?}", uri.to_string()))).await;
        self.documents.invalidate(&uri.to_string()).await;
    }

    async fn did_change_watched_files(&self, params: DidChangeWatchedFilesParams) {
        let DidChangeWatchedFilesParams { changes } = params;
        for change in changes {
            if change.typ == FileChangeType::DELETED {
                Window::log(info!(format!(
                    "did change(delete) file: {}",
                    change.uri.to_string()
                )))
                .await;
                self.documents.invalidate(&change.uri.to_string()).await;
            }
        }
    }

    async fn did_delete_files(&self, params: DeleteFilesParams) {
        for file in &params.files {
            let Ok(uri) = Uri::from_str(&file.uri) else {
                Window::log(warning!(format!("invalid file uri: {}", file.uri))).await;
                continue;
            };
            Window::log(info!(format!("did delete file: {}", uri.to_string()))).await;
            self.documents.invalidate(&uri.to_string()).await;
        }
    }

    // LSP request/response
    async fn diagnostic(
        &self,
        params: DocumentDiagnosticParams,
    ) -> jsonrpc::Result<DocumentDiagnosticReportResult> {
        let uri = params.text_document.uri.clone();
        self.with_document(&uri, async move |document| {
            document.on_diagnostic(params).await
        })
        .await
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> jsonrpc::Result<Option<SemanticTokensResult>> {
        let uri = params.text_document.uri.clone();
        self.with_document(&uri, async move |document| {
            document.on_semantic_tokens_full(params).await
        })
        .await
    }

    async fn hover(&self, params: HoverParams) -> jsonrpc::Result<Option<Hover>> {
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        self.with_document(&uri, async move |document| document.on_hover(params).await)
            .await
    }

    async fn completion(
        &self,
        params: CompletionParams,
    ) -> jsonrpc::Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri.clone();
        self.with_document(&uri, async move |document| {
            document.on_completion(params).await
        })
        .await
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> jsonrpc::Result<Option<GotoDefinitionResponse>> {
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        self.with_document(&uri, async move |document| {
            document.on_goto_definition(params).await
        })
        .await
    }

    async fn formatting(
        &self,
        params: DocumentFormattingParams,
    ) -> jsonrpc::Result<Option<Vec<TextEdit>>> {
        let uri = params.text_document.uri.clone();
        self.with_document(&uri, async move |document| {
            document.on_formatting(params).await
        })
        .await
    }

    async fn folding_range(
        &self,
        params: FoldingRangeParams,
    ) -> jsonrpc::Result<Option<Vec<FoldingRange>>> {
        let uri = params.text_document.uri.clone();
        self.with_document(&uri, async move |document| {
            document.on_folding_range(params).await
        })
        .await
    }

    async fn code_action(
        &self,
        params: CodeActionParams,
    ) -> jsonrpc::Result<Option<Vec<CodeActionOrCommand>>> {
        let uri = params.text_document.uri.clone();
        self.with_document(&uri, async move |document| {
            document.on_code_action(params).await
        })
        .await
    }
}
