// Copyright 2025-2026 Nokia
// Licensed under the BSD 3-Clause License.
// SPDX-License-Identifier: BSD-3-Clause

use std::str::FromStr;
use tower_lsp_server::{
    jsonrpc::Result as JsonRpcResult,
    ls_types::{
        CodeAction, CodeActionKind, CodeActionOptions, CodeActionOrCommand, CodeActionParams,
        CodeActionProviderCapability, Diagnostic, NumberOrString, TextEdit, Uri, WorkspaceEdit,
    },
};

use crate::grammar::Rule;

use crate::{document::Document, features::ActionFeature};

#[allow(clippy::mutable_key_type)]
fn create_fix_warning_action(
    code: &String,
    uri: &Uri,
    diagnostic: Diagnostic,
) -> Option<CodeActionOrCommand> {
    // The TextEdit describes replacing the diagnostic's range with the correct text
    let new_text = match Rule::from_str(code.as_str()) {
        Ok(Rule::LegacyEqualOperator) => "==".to_string(),
        Ok(Rule::SelfClosingTag) => ">".to_string(),
        _ => return None,
    };
    let text_edit = TextEdit {
        range: diagnostic.range,
        new_text,
    };

    Some(CodeActionOrCommand::CodeAction(CodeAction {
        title: format!("fix warning: {}", code),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: Some(vec![diagnostic]), // Link this action to the diagnostic it fixes
        edit: Some(WorkspaceEdit {
            changes: Some(vec![(uri.clone(), vec![text_edit])].into_iter().collect()),
            ..Default::default()
        }),
        is_preferred: Some(true), // Suggest this as the primary fix
        ..Default::default()
    }))
}

pub fn code_action_capability() -> CodeActionProviderCapability {
    CodeActionProviderCapability::Options(CodeActionOptions {
        code_action_kinds: Some(vec![CodeActionKind::QUICKFIX]),
        ..Default::default()
    })
}

impl ActionFeature for Document {
    async fn on_code_action(
        &self,
        params: CodeActionParams,
    ) -> JsonRpcResult<Option<Vec<CodeActionOrCommand>>> {
        Ok(params
            .context
            .diagnostics
            .iter()
            .map(|diagnostic| {
                if let Some(NumberOrString::String(code)) = &diagnostic.code {
                    // string codes
                    create_fix_warning_action(code, &params.text_document.uri, diagnostic.clone())
                } else {
                    None
                }
            })
            .collect())
    }
}
