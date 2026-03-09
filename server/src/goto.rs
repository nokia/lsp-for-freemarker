// Copyright 2025-2026 Nokia
// Licensed under the BSD 3-Clause License.
// SPDX-License-Identifier: BSD-3-Clause

use std::str::FromStr;

use tower_lsp_server::{
    jsonrpc::Result as JsonRpcResult,
    ls_types::{
        DefinitionOptions, GotoDefinitionParams, GotoDefinitionResponse, Location, OneOf, Range,
    },
};
use tree_sitter_freemarker::grammar::Rule;

use crate::{reactor::Reactor, server::GotoFeature, utils};

pub fn definition_capability() -> OneOf<bool, DefinitionOptions> {
    OneOf::Left(true)
}

impl GotoFeature for Reactor {
    async fn on_goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> JsonRpcResult<Option<GotoDefinitionResponse>> {
        let point =
            utils::lsp_position_to_parser_point(&params.text_document_position_params.position);
        if let Some(node) = self.get_parser().get_node_at_point(point)
            && let Ok(rule) = Rule::from_str(node.kind())
        {
            return match rule {
                Rule::ImportPath => {
                    // import path is always quoted
                    let path_text = self
                        .get_document()
                        .get_ranged_text(node.start_byte() + 1..node.end_byte() - 1);
                    if let Some(path_uri) = self.get_analysis().get_valid_import(&path_text) {
                        return Ok(Some(GotoDefinitionResponse::Scalar(Location {
                            uri: path_uri.clone(),
                            range: Range::default(),
                        })));
                    }
                    Ok(None)
                }
                Rule::MacroNamespace => {
                    let macro_namespace = self
                        .get_document()
                        .get_ranged_text(node.start_byte()..node.end_byte());
                    if let Ok(symbols) =
                        self.get_analysis().find_symbol_definition(&macro_namespace)
                    {
                        let first_definition = symbols[0];
                        return Ok(Some(GotoDefinitionResponse::Scalar(Location {
                            uri: self.get_document().uri(),
                            range: first_definition.range,
                        })));
                    }
                    Ok(None)
                }
                _ => Ok(None),
            };
        }
        Ok(None)
    }
}
