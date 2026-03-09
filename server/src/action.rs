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

use tree_sitter_freemarker::grammar::Rule;

use crate::{reactor::Reactor, server::ActionFeature};

#[allow(clippy::mutable_key_type)]
fn create_fix_warning_action(
    code: &String,
    uri: &Uri,
    diagnostic: Diagnostic,
) -> Option<CodeActionOrCommand> {
    let rule = Rule::from_str(code.as_str());
    if rule.is_err() {
        return None;
    }
    // The TextEdit describes replacing the diagnostic's range with the correct text
    let text_edit = TextEdit {
        range: diagnostic.range,
        new_text: match rule.unwrap() {
            Rule::DeprecatedEqualOperator => "==".to_string(),
            Rule::UndocumentedCloseTag => ">".to_string(),
            _ => return None,
        },
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

impl ActionFeature for Reactor {
    async fn on_code_action(
        &self,
        params: CodeActionParams,
    ) -> JsonRpcResult<Option<Vec<CodeActionOrCommand>>> {
        let mut actions: Vec<CodeActionOrCommand> = Vec::new();
        for diagnostic in params.context.diagnostics {
            if let Some(NumberOrString::String(code)) = &diagnostic.code {
                // string codes
                if let Some(fix_action) =
                    create_fix_warning_action(code, &params.text_document.uri, diagnostic.clone())
                {
                    // Create a CodeAction for this specific diagnostic
                    actions.push(fix_action);
                }
            }
        }
        Ok(Some(actions))
    }
}
