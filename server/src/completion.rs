// Copyright 2025-2026 Nokia
// Licensed under the BSD 3-Clause License.
// SPDX-License-Identifier: BSD-3-Clause

use tower_lsp_server::{
    jsonrpc::Result as JsonRpcResult,
    ls_types::{
        CompletionItem, CompletionItemKind, CompletionItemLabelDetails, CompletionOptions,
        CompletionOptionsCompletionItem, CompletionParams, CompletionResponse, Documentation,
        InsertTextFormat, InsertTextMode, MarkupContent, MarkupKind, Position,
    },
};

use once_cell::sync::Lazy;
use rust_embed::Embed;
use serde::Deserialize;
use strum::IntoEnumIterator;
use tree_sitter_freemarker::grammar::{Builtin, Rule};

use crate::reactor::Reactor;
use crate::server::CompletionFeature;

#[derive(Embed)]
#[folder = "assets/completion"]
struct CompletionAssetPath;

#[derive(Debug, Default, Deserialize)]
struct LabelDetails {
    detail: Option<String>,
    description: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct CompletionAssetItem {
    category: String,
    deprecated: Option<bool>,
    label: String,
    insert_text: String,
    documentation: String,
    label_details: Option<LabelDetails>,
}

impl CompletionAssetItem {
    #[tracing::instrument(skip_all)]
    fn from_bytes(bytes: &[u8]) -> Option<CompletionAssetItem> {
        match std::str::from_utf8(bytes) {
            Ok(s) => match toml::from_str::<CompletionAssetItem>(s) {
                Ok(item) => Some(item),
                Err(e) => {
                    panic!("rust-embed deserialization error: {}:", e)
                }
            },
            Err(e) => {
                panic!("utf-8 parsing error: {}", e)
            }
        }
    }

    #[tracing::instrument(skip_all)]
    fn from_embed(file: &str) -> Option<CompletionAssetItem> {
        if let Some(completion_file) = CompletionAssetPath::get(file) {
            return CompletionAssetItem::from_bytes(completion_file.data.as_ref());
        }
        tracing::error!("rust-embed file not found: {}:", file);
        None
    }

    fn as_directive_completion(&self) -> CompletionItem {
        assert_eq!(self.category, "directive");
        CompletionItem {
            documentation: Some(Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: self.documentation.clone(),
            })),
            deprecated: self.deprecated,
            label: self.label.clone(),
            label_details: self
                .label_details
                .as_ref()
                .map(|ld| CompletionItemLabelDetails {
                    detail: ld.detail.clone(),
                    description: ld.description.clone(),
                }),
            kind: Some(CompletionItemKind::METHOD),
            insert_text: Some(self.insert_text.clone()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            insert_text_mode: Some(InsertTextMode::ADJUST_INDENTATION),
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone)]
struct CompletionAsset {
    directive_completion: Vec<CompletionItem>,
    // TODO: other completions
}

impl CompletionAsset {
    fn new() -> Self {
        let mut directive_completion: Vec<CompletionItem> = vec![];
        CompletionAssetPath::iter().for_each(|file| {
            if let Some(item) = CompletionAssetItem::from_embed(&file)
                && item.category.as_str() == "directive"
            {
                directive_completion.push(item.as_directive_completion())
            }
        });
        CompletionAsset {
            directive_completion,
        }
    }
}

static STATIC_ASSETS: Lazy<CompletionAsset> = Lazy::new(CompletionAsset::new);

fn completion_for_builtin() -> Vec<CompletionItem> {
    // todo: improve filter result by partial identifier
    Builtin::iter()
        .map(|i| CompletionItem {
            label: i.to_string(),
            kind: Some(CompletionItemKind::FIELD),
            ..Default::default()
        })
        .collect()
}

pub fn completion_capability() -> CompletionOptions {
    CompletionOptions {
        resolve_provider: Some(false),
        trigger_characters: Some(vec![
            "#".to_string(), // '<#' --> trigger directive
            "{".to_string(), // '${' --> trigger interpolation
            "?".to_string(), // '?' --> trigger built-ins
            "@".to_string(), // "<@" --> trigger macro call
        ]),
        completion_item: Some(CompletionOptionsCompletionItem {
            label_details_support: Some(true),
        }),
        ..Default::default()
    }
}

impl CompletionFeature for Reactor {
    fn list_macro_definitions(&self) -> Vec<CompletionItem> {
        let mut macro_definitions = vec![];
        self.get_analysis().foreach_symbol(|symbol_name, symbols| {
            let first_definition = symbols[0];
            if matches!(first_definition.rule, Rule::MacroName | Rule::ImportAlias) {
                macro_definitions.push(CompletionItem {
                    label: symbol_name.to_owned(),
                    kind: Some(CompletionItemKind::MODULE),
                    documentation: Some(Documentation::MarkupContent(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: self
                            .get_document()
                            .get_ranged_text(first_definition.start_byte..first_definition.end_byte)
                            .to_string(),
                    })),
                    insert_text: Some(symbol_name.to_owned()),
                    insert_text_format: Some(InsertTextFormat::SNIPPET),
                    insert_text_mode: Some(InsertTextMode::AS_IS),
                    ..Default::default()
                });
            }
        });
        macro_definitions
    }

    async fn on_completion(
        &self,
        params: CompletionParams,
    ) -> JsonRpcResult<Option<CompletionResponse>> {
        if params
            .context
            .as_ref()
            .is_none_or(|ctx| ctx.trigger_character.is_none())
        {
            return Ok(None);
        }
        // the position has point to 1 char after trigger
        let position = params.text_document_position.position;
        assert!(position.character > 0);
        let trigger_position = Position {
            line: position.line,
            character: position.character - 1,
        };
        let prev_char = self.get_document().get_prev_char_at(&trigger_position);
        if prev_char.as_ref().is_none() {
            return Ok(None);
        }
        let prev_char = prev_char.unwrap();
        let ctx = params.context.unwrap();
        let trigger = ctx.trigger_character.unwrap();
        let mut result: Option<CompletionResponse> = None;

        match trigger.as_str() {
            "#" if prev_char == '<' => {
                // triggered by '<#', expect a directive keyword
                result = Some(CompletionResponse::Array(
                    STATIC_ASSETS.directive_completion.clone(),
                ));
            }
            "@" if prev_char == '<' => {
                // triggered by '<@', expect a macro call
                result = Some(CompletionResponse::Array(self.list_macro_definitions()));
            }
            "?" => {
                // triggered by '?', expect a built-in
                result = Some(CompletionResponse::Array(completion_for_builtin()));
            }
            _ => {}
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use crate::completion::{CompletionAsset, CompletionAssetItem};

    #[test]
    fn test_asset_assign_directive() {
        let item = CompletionAssetItem::from_embed("assign.toml");
        assert!(item.is_some());
        let item = item.unwrap();
        assert_eq!(item.category.as_str(), "directive");
        assert_eq!(item.label.as_str(), "assign");
        assert!(item.label_details.is_none());
    }

    #[test]
    fn test_asset_assign_capture_directive() {
        let item = CompletionAssetItem::from_embed("assign(capture).toml");
        assert!(item.is_some());
        let item = item.unwrap();
        assert_eq!(item.category.as_str(), "directive");
        assert_eq!(item.label.as_str(), "assign");
        match item.label_details {
            Some(label_details) => {
                assert_eq!(label_details.detail.unwrap_or_default(), "(capture)");
                assert!(label_details.description.is_none());
            }
            None => unreachable!(),
        }
    }

    #[test]
    fn test_asset_directives() {
        let asset = CompletionAsset::new();
        assert!(!asset.directive_completion.is_empty());
    }
}
