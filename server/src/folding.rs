// Copyright 2025-2026 Nokia
// Licensed under the BSD 3-Clause License.
// SPDX-License-Identifier: BSD-3-Clause

use std::{collections::HashSet, str::FromStr};

use tower_lsp_server::{
    jsonrpc,
    ls_types::{FoldingRange, FoldingRangeParams, FoldingRangeProviderCapability},
};
use tree_sitter::Node;

use crate::{document::Document, features::FoldingFeature, grammar::Rule};

pub fn folding_capability() -> FoldingRangeProviderCapability {
    FoldingRangeProviderCapability::Simple(true)
}

/// Computes folding ranges by walking the syntax tree once.
pub(crate) fn folding_ranges(doc: &Document) -> Vec<FoldingRange> {
    fn collect(node: &Node, seen: &mut HashSet<usize>, ranges: &mut Vec<FoldingRange>) {
        if node.is_error() || node.is_missing() {
            // not sure if it is proper
            return;
        }
        if let Ok(
            Rule::Comment
            | Rule::AssignClause
            | Rule::CaseClause
            | Rule::DefaultClause
            | Rule::ElseClause
            | Rule::ElseifClause
            | Rule::FunctionClause
            | Rule::IfClause
            | Rule::ListClause
            | Rule::LocalClause
            | Rule::MacroClause
            | Rule::OnClause
            | Rule::SwitchClause,
        ) = Rule::from_str(node.kind())
        {
            let id = node.id();
            if seen.insert(id) {
                let start_line = node.start_position().row as u32;
                // Saturate: a node ending on line 0 (e.g. a comment on the
                // first line) would otherwise underflow the subtraction.
                let end_line = node.end_position().row.saturating_sub(1) as u32;
                // Only fold constructs spanning more than one line; anything
                // shorter would produce a degenerate range.
                if start_line < end_line {
                    ranges.push(FoldingRange {
                        start_line,
                        end_line,
                        ..Default::default()
                    });
                }
            }
        }
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i as u32) {
                collect(&child, seen, ranges);
            }
        }
    }

    let mut ranges = Vec::new();
    let mut seen = HashSet::new();
    collect(&doc.tree().root_node(), &mut seen, &mut ranges);
    ranges
}

impl FoldingFeature for Document {
    async fn on_folding_range(
        &self,
        _: FoldingRangeParams,
    ) -> jsonrpc::Result<Option<Vec<FoldingRange>>> {
        Ok(Some(folding_ranges(self)))
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use tower_lsp_server::ls_types::Uri;

    use super::folding_ranges;
    use crate::document::Document;

    /// Regression test: a folding construct that ends on line 0 (a comment on
    /// the first line of a file) used to underflow `end_line - 1` and panic in
    /// debug builds. See `corpus/goto/lib/common.ftl`.
    #[test]
    fn comment_on_first_line_does_not_underflow() {
        let text = "<#-- The helper template imported below. -->\n\
                    <#macro greet(name)>Hello, ${name}!</#macro>\n";
        let uri = Uri::from_str("file:///tmp/common.ftl").expect("valid test uri");
        let document = Document::open(&uri, text, 1);
        // Must not panic, and any emitted ranges must be well-formed.
        for range in folding_ranges(&document) {
            assert!(
                range.start_line <= range.end_line,
                "fold range must not be inverted: {range:?}"
            );
        }
    }
}
