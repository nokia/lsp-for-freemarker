// Copyright 2025-2026 Nokia
// Licensed under the BSD 3-Clause License.
// SPDX-License-Identifier: BSD-3-Clause

use tower_lsp_server::{
    jsonrpc::Result as JsonRpcResult,
    ls_types::{
        DocumentFormattingOptions, DocumentFormattingParams, OneOf, Position, Range, TextEdit,
    },
};
use tree_sitter::Point;

use crate::{reactor::Reactor, server::FormatFeature, window_log_info};

#[derive(Clone, Copy)]
struct FormatState {
    preset: Option<usize>,
    indent: usize,
    has_directive: bool,
}

fn reset_state(mut state: FormatState) -> FormatState {
    // MUST NOT reset the preset
    state.indent = 0;
    state.has_directive = false;
    state
}

fn update_state(
    reactor: &Reactor,
    index: usize,
    line: &str,
    mut state: FormatState,
) -> FormatState {
    let trimed_line = line.trim_start();
    if trimed_line.starts_with("</#") || trimed_line.starts_with("<#") {
        let col = line.len() - trimed_line.len();
        let node = reactor
            .get_parser()
            .get_node_at_point(Point {
                row: index,
                column: col,
            })
            .unwrap();
        if node.kind() == "comment" {
            // under comment section
            state.has_directive = false;
        } else {
            state.has_directive = true;
            // compute indent
            let mut node_cursor = node;
            while let Some(parent) = node_cursor.parent() {
                let kind = node_cursor.kind();
                if kind.ends_with("_clause") {
                    // node kind with "_clause" requires indent increasing
                    state.indent += 1;
                }
                node_cursor = parent;
            }
            // save the preset indentation for the top level directives
            if state.indent == 0
                && let Some(parent) = node.parent()
            {
                let _parentkind = parent.kind();
                let end_row = parent.end_position().row;
                let start_row = parent.start_position().row;
                if end_row > start_row {
                    if state.preset.is_none() {
                        state.preset = Some(col);
                    }
                } else if col > 0 {
                    state.preset = Some(col);
                } else {
                    state.preset = None;
                }
            }
        }
    }
    state
}

pub fn formatting_capability() -> OneOf<bool, DocumentFormattingOptions> {
    OneOf::Left(true)
}

impl FormatFeature for Reactor {
    async fn on_formatting(
        &self,
        params: DocumentFormattingParams,
    ) -> JsonRpcResult<Option<Vec<TextEdit>>> {
        let uri = params.text_document.uri;
        window_log_info!(format!("on_formatting: {}", uri.to_string()));
        let mut state = FormatState {
            preset: None,
            indent: 0,
            has_directive: false,
        };
        let mut formatted = String::from("");
        let mut last_length = 0;
        let line_count = self.get_document().line_count();
        self.get_document().enumerate_lines(|index, line| {
            last_length = line.len();
            state = update_state(self, index, line, state);
            let preset = state.preset.unwrap_or_default();
            if state.has_directive {
                // todo: make indent step become a configuration
                // currently use 4 whitespaces as the indent step by default
                formatted += &(" ".repeat(preset + state.indent * 4) + line.trim());
            } else {
                formatted += line;
            }
            if index < line_count - 1 {
                formatted += "\n";
            }
            state = reset_state(state);
        });
        let range = Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: line_count as u32,
                character: last_length as u32,
            },
        };
        Ok(Some(vec![TextEdit::new(range, formatted)]))
    }
}
