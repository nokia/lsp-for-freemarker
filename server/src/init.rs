// Copyright 2025-2026 Nokia
// Licensed under the BSD 3-Clause License.
// SPDX-License-Identifier: BSD-3-Clause

use tower_lsp_server::ls_types::{
    FileOperationFilter, FileOperationPattern, FileOperationRegistrationOptions, InitializeParams,
    InitializeResult, ServerCapabilities, ServerInfo, TextDocumentSyncCapability,
    TextDocumentSyncKind, WorkspaceFileOperationsServerCapabilities, WorkspaceServerCapabilities,
};
use tracing::{Level, event};

use crate::server::{Initializer, Server};
use crate::{
    action, completion, diagnosis, folding, format, goto, hover, tokenizer, window_log_info,
};

fn do_initialize() -> InitializeResult {
    InitializeResult {
        capabilities: ServerCapabilities {
            text_document_sync: Some(TextDocumentSyncCapability::Kind(
                TextDocumentSyncKind::INCREMENTAL,
            )),
            definition_provider: Some(goto::definition_capability()),
            hover_provider: Some(hover::hover_capability()),
            code_action_provider: Some(action::code_action_capability()),
            completion_provider: Some(completion::completion_capability()),
            diagnostic_provider: Some(diagnosis::diagnostic_capability()),
            document_formatting_provider: Some(format::formatting_capability()),
            semantic_tokens_provider: Some(tokenizer::semantic_token_capability()),
            folding_range_provider: Some(folding::folding_capability()),
            workspace: Some(WorkspaceServerCapabilities {
                file_operations: Some(WorkspaceFileOperationsServerCapabilities {
                    did_delete: Some(FileOperationRegistrationOptions {
                        filters: vec![FileOperationFilter {
                            pattern: FileOperationPattern {
                                glob: "**".to_string(),
                                ..Default::default()
                            },
                            ..Default::default()
                        }],
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        },
        server_info: Some(ServerInfo {
            name: Server::NAME.to_owned(),
            version: Some(env!("CARGO_PKG_VERSION").to_string()),
        }),
    }
}

impl Initializer for Server {
    #[allow(deprecated)]
    async fn on_initialize(&self, params: InitializeParams) -> InitializeResult {
        window_log_info!("[Server] initializing...");
        if let Ok(mut root_path) = self.root_path.try_write() {
            event!(
                Level::DEBUG,
                process = params.process_id,
                pwd = params.root_path
            );
            root_path.clone_from(&params.root_path.unwrap_or_default());
        }
        do_initialize()
    }
}
