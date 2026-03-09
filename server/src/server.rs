// Copyright 2025-2026 Nokia
// Licensed under the BSD 3-Clause License.
// SPDX-License-Identifier: BSD-3-Clause

use std::sync::Arc;

use tokio::sync::RwLock;
use tower_lsp_server::{
    Client, LanguageServer, jsonrpc,
    ls_types::{
        CodeActionOrCommand, CodeActionParams, CompletionItem, CompletionParams,
        CompletionResponse, DeleteFilesParams, DidChangeTextDocumentParams,
        DidChangeWatchedFilesParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
        DocumentDiagnosticParams, DocumentDiagnosticReportResult, DocumentFormattingParams,
        FoldingRange, FoldingRangeParams, GotoDefinitionParams, GotoDefinitionResponse, Hover,
        HoverParams, InitializeParams, InitializeResult, InitializedParams, SemanticTokensParams,
        SemanticTokensResult, TextEdit,
    },
};
use tracing::{self, instrument};

use crate::{client::save_client, window_log_info, workspace::Workspace};

#[derive(Debug)]
pub struct Server {
    pub(crate) root_path: Arc<RwLock<String>>,
    pub(crate) workspace: Workspace,
}

impl Server {
    pub const NAME: &str = "Freemarker Language Server";
    pub const CODE_NAME: &str = "lsp-for-freemarker";

    pub fn new(client: Client) -> Self {
        let _ = save_client(client);
        Self {
            root_path: Arc::new(RwLock::new(String::new())),
            workspace: Workspace::new(),
        }
    }
}

pub trait Initializer {
    async fn on_initialize(&self, params: InitializeParams) -> InitializeResult;
}

//#[tower_lsp_server::async_trait]
impl LanguageServer for Server {
    async fn initialize(&self, params: InitializeParams) -> jsonrpc::Result<InitializeResult> {
        return Ok(self.on_initialize(params).await);
    }

    async fn initialized(&self, _: InitializedParams) {
        window_log_info!("[Server] initialized.");
    }

    async fn shutdown(&self) -> jsonrpc::Result<()> {
        window_log_info!("[Server] shutdown :)");
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.workspace.on_did_open(&params).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        self.workspace.on_did_change(&params).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = &params.text_document.uri;
        window_log_info!(format!("did_close: {:?}", uri.to_string()));
    }

    async fn did_change_watched_files(&self, params: DidChangeWatchedFilesParams) {
        self.workspace.on_did_change_watched_files(params).await;
    }

    async fn did_delete_files(&self, params: DeleteFilesParams) {
        self.workspace.on_did_delete_files(params).await;
    }

    // LSP request/response
    async fn diagnostic(
        &self,
        params: DocumentDiagnosticParams,
    ) -> jsonrpc::Result<DocumentDiagnosticReportResult> {
        self.workspace.on_diagnostic(params).await
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> jsonrpc::Result<Option<SemanticTokensResult>> {
        self.workspace.on_semantic_tokens_full(params).await
    }

    async fn hover(&self, params: HoverParams) -> jsonrpc::Result<Option<Hover>> {
        self.workspace.on_hover(params).await
    }

    async fn completion(
        &self,
        params: CompletionParams,
    ) -> jsonrpc::Result<Option<CompletionResponse>> {
        self.workspace.on_completion(params).await
    }

    #[instrument(skip_all)]
    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> jsonrpc::Result<Option<GotoDefinitionResponse>> {
        self.workspace.on_goto_definition(params).await
    }

    #[instrument(skip_all)]
    async fn formatting(
        &self,
        params: DocumentFormattingParams,
    ) -> jsonrpc::Result<Option<Vec<TextEdit>>> {
        self.workspace.on_formatting(params).await
    }

    async fn folding_range(
        &self,
        params: FoldingRangeParams,
    ) -> jsonrpc::Result<Option<Vec<FoldingRange>>> {
        self.workspace.on_folding_range(params).await
    }

    async fn code_action(
        &self,
        params: CodeActionParams,
    ) -> jsonrpc::Result<Option<Vec<CodeActionOrCommand>>> {
        self.workspace.on_code_action(params).await
    }
}

// LSP features
pub trait ActionFeature {
    async fn on_code_action(
        &self,
        params: CodeActionParams,
    ) -> jsonrpc::Result<Option<Vec<CodeActionOrCommand>>>;
}

pub trait CompletionFeature {
    async fn on_completion(
        &self,
        params: CompletionParams,
    ) -> jsonrpc::Result<Option<CompletionResponse>>;

    fn list_macro_definitions(&self) -> Vec<CompletionItem>;
}

pub trait DiagnosticFeature {
    async fn on_diagnostic(
        &self,
        params: DocumentDiagnosticParams,
    ) -> jsonrpc::Result<DocumentDiagnosticReportResult>;
}

pub trait FoldingFeature {
    async fn on_folding_range(
        &self,
        params: FoldingRangeParams,
    ) -> jsonrpc::Result<Option<Vec<FoldingRange>>>;
}

pub trait FormatFeature {
    async fn on_formatting(
        &self,
        params: DocumentFormattingParams,
    ) -> jsonrpc::Result<Option<Vec<TextEdit>>>;
}

pub trait GotoFeature {
    async fn on_goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> jsonrpc::Result<Option<GotoDefinitionResponse>>;
}

pub trait HoverFeature {
    //fn on_node(&self, position: Position) -> Option<Node<'_>>;
    async fn on_hover(&self, params: HoverParams) -> jsonrpc::Result<Option<Hover>>;
}

pub trait SemanticTokenFeature {
    async fn on_semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> jsonrpc::Result<Option<SemanticTokensResult>>;
}
