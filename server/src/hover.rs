// Copyright 2025-2026 Nokia
// Licensed under the BSD 3-Clause License.
// SPDX-License-Identifier: BSD-3-Clause

use once_cell::sync::Lazy;
use rust_embed::{Embed, EmbeddedFile};
use serde::Deserialize;
use std::{collections::HashMap, str::FromStr};
use tower_lsp_server::{
    jsonrpc,
    ls_types::{
        Hover, HoverContents, HoverParams, HoverProviderCapability, MarkedString, MarkupContent,
        MarkupKind,
    },
};
use tree_sitter_freemarker::grammar::Rule;

//use crate::symbol::MacroNamespace;
use crate::{reactor::Reactor, server::HoverFeature, utils};

#[derive(Embed)]
#[folder = "assets/hover/"]
#[include = "built-ins/*"]
#[include = "types/*"]
struct HoverAssetPath;

#[derive(Debug, Default, Deserialize)]
struct HoverAssetItem {
    // static markdown text
    identifier: String,
    category: String,
    markdown: Option<String>,
    // TODO: dynamic text "rendering"
}

impl HoverAssetItem {
    #[tracing::instrument(skip_all)]
    fn from_bytes(bytes: &[u8]) -> Option<HoverAssetItem> {
        match std::str::from_utf8(bytes) {
            Ok(s) => match toml::from_str::<HoverAssetItem>(s) {
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
    fn from_embed(file: EmbeddedFile) -> Option<HoverAssetItem> {
        HoverAssetItem::from_bytes(file.data.as_ref())
    }
}

#[derive(Debug, Clone)]
struct HoverAsset {
    built_in: HashMap<String, Hover>,
    types: HashMap<String, Hover>,
    // TODO: other hovers
}

fn insert_to_hover_map(item: HoverAssetItem, hovers: &mut HashMap<String, Hover>) {
    hovers.insert(
        item.identifier,
        Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: item.markdown.unwrap_or_default(),
            }),
            range: None,
        },
    );
}

impl HoverAsset {
    fn new() -> Self {
        let mut built_in: HashMap<String, Hover> = HashMap::new();
        let mut types: HashMap<String, Hover> = HashMap::new();
        HoverAssetPath::iter().for_each(|file| {
            if let Some(embedded_file) = HoverAssetPath::get(&file)
                && let Some(item) = HoverAssetItem::from_embed(embedded_file)
            {
                match item.category.as_str() {
                    "built-in" => insert_to_hover_map(item, &mut built_in),
                    "types" => insert_to_hover_map(item, &mut types),
                    _ => {}
                }
            }
        });
        HoverAsset { built_in, types }
    }
}

static STATIC_ASSETS: Lazy<HoverAsset> = Lazy::new(HoverAsset::new);

pub fn hover_capability() -> HoverProviderCapability {
    HoverProviderCapability::Simple(true)
}

impl HoverFeature for Reactor {
    async fn on_hover(&self, params: HoverParams) -> jsonrpc::Result<Option<Hover>> {
        let point =
            utils::lsp_position_to_parser_point(&params.text_document_position_params.position);
        if let Some(node) = self.get_parser().get_node_at_point(point)
            && let Ok(rule) = Rule::from_str(node.kind())
        {
            return match rule {
                Rule::Number | Rule::StringLiteral | Rule::BooleanTrue | Rule::BooleanFalse => {
                    let rule_str = match matches!(rule, Rule::BooleanTrue | Rule::BooleanFalse) {
                        true => "boolean",
                        false => &rule.to_string(),
                    };
                    if let Some(hover) = STATIC_ASSETS.types.get(rule_str) {
                        return Ok(Some(Hover {
                            contents: hover.contents.clone(),
                            range: Some(utils::parser_node_to_document_range(&node)),
                        }));
                    }
                    return Ok(None);
                }
                Rule::BuiltinName => {
                    let node_text = self
                        .get_document()
                        .get_ranged_text(node.start_byte()..node.end_byte());
                    if let Some(hover) = STATIC_ASSETS.built_in.get(&node_text) {
                        return Ok(Some(Hover {
                            contents: hover.contents.clone(),
                            range: Some(utils::parser_node_to_document_range(&node)),
                        }));
                    }
                    return Ok(None);
                }
                Rule::MacroNamespace => {
                    let node_text = self
                        .get_document()
                        .get_ranged_text(node.start_byte()..node.end_byte());
                    match self.get_analysis().find_symbol_definition(&node_text) {
                        Ok(symbols) => {
                            let sym = symbols[0];
                            let definition_line = self
                                .get_document()
                                .get_line_text(sym.range.start.line as usize);
                            return Ok(Some(Hover {
                                contents: HoverContents::Scalar(MarkedString::LanguageString(
                                    utils::ftl_to_rust(definition_line.trim()),
                                )),
                                range: Some(utils::parser_node_to_document_range(&node)),
                            }));
                        }
                        _ => Ok(None),
                    }
                }
                _ => Ok(None),
            };
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use crate::hover::{HoverAsset, HoverAssetItem, HoverAssetPath};

    #[test]
    fn test_asset_builtin_from_str() {
        if let Some(item) = HoverAssetItem::from_bytes(
            r#"identifier = "foo"
category = "bar"
markdown = """baz"""
"#
            .as_bytes(),
        ) {
            assert_eq!(item.identifier, "foo".to_string());
            assert_eq!(item.category, "bar".to_string());
            assert!(item.markdown.is_some());
            assert_eq!(item.markdown.unwrap(), "baz".to_string());
        }
    }

    #[test]
    fn test_asset_builtin_from_file() {
        let embedded_file = HoverAssetPath::get("built-ins/c.toml").expect("must success");
        if let Some(item) = HoverAssetItem::from_embed(embedded_file) {
            assert_eq!(item.identifier, "c".to_string());
            assert_eq!(item.category, "built-in".to_string());
            assert!(item.markdown.is_some());
        }
    }

    #[test]
    fn test_asset_builtin() {
        let asset = HoverAsset::new();
        assert!(!asset.built_in.is_empty());
    }
}
