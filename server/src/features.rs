// Copyright 2025-2026 Nokia
// Licensed under the BSD 3-Clause License.
// SPDX-License-Identifier: BSD-3-Clause

//! LSP feature traits implemented by the per-document [`crate::document::Document`].

use tower_lsp_server::{
    jsonrpc,
    ls_types::{
        CodeActionOrCommand, CodeActionParams, CompletionItem, CompletionParams,
        CompletionResponse, DocumentDiagnosticParams, DocumentDiagnosticReportResult,
        DocumentFormattingParams, FoldingRange, FoldingRangeParams, GotoDefinitionParams,
        GotoDefinitionResponse, Hover, HoverParams, SemanticTokensParams, SemanticTokensResult,
        TextEdit,
    },
};

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
    async fn on_hover(&self, params: HoverParams) -> jsonrpc::Result<Option<Hover>>;
}

pub trait SemanticTokenFeature {
    async fn on_semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> jsonrpc::Result<Option<SemanticTokensResult>>;
}
