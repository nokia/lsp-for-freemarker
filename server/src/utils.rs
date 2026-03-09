// Copyright 2025-2026 Nokia
// Licensed under the BSD 3-Clause License.
// SPDX-License-Identifier: BSD-3-Clause

use tower_lsp_server::ls_types::{LanguageString, Position, Range};
use tree_sitter::{Node, Point};

pub fn parser_node_to_document_range(node: &Node) -> Range {
    let start = node.start_position();
    let end = node.end_position();
    Range {
        start: Position {
            line: start.row as u32,
            character: start.column as u32,
        },
        end: Position {
            line: end.row as u32,
            character: end.column as u32,
        },
    }
}

pub fn lsp_position_to_parser_point(position: &Position) -> Point {
    Point {
        row: position.line as usize,
        column: position.character as usize,
    }
}

pub fn ftl_to_rust(ftl_text: &str) -> LanguageString {
    // for highlighting in hover
    let line_trimmed = ftl_text.trim();
    let mut result = String::from(line_trimmed);
    if result.starts_with("<#import") || result.starts_with("<#macro") {
        result = result.replacen("<#", "", 1);
    }
    if result.ends_with('>') {
        result.pop();
    }
    LanguageString {
        language: "rust".to_owned(),
        value: result,
    }
}
