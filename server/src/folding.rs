// Copyright 2025-2026 Nokia
// Licensed under the BSD 3-Clause License.
// SPDX-License-Identifier: BSD-3-Clause

use std::str::FromStr;

use tower_lsp_server::ls_types::{FoldingRange, FoldingRangeProviderCapability};
use tree_sitter::Node;
use tree_sitter_freemarker::grammar::Rule;

use crate::{
    analysis::{Analysis, AnalysisContext, FoldingAnalysis},
    reactor::Reactor,
    server::FoldingFeature,
};

pub fn folding_capability() -> FoldingRangeProviderCapability {
    FoldingRangeProviderCapability::Simple(true)
}

impl FoldingAnalysis for Analysis {
    fn analyze_folding_ranges(&mut self, node: &Node, ctx: &mut AnalysisContext) {
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
            | Rule::FunctionClause
            | Rule::IfClause
            | Rule::ListClause
            | Rule::LocalClause
            | Rule::MacroClause
            | Rule::OnClause
            | Rule::SwitchClause,
        ) = Rule::from_str(node.kind())
        {
            // node kind with "_clause" requires indent increasing
            let id = node.id();
            if !ctx.ranges_set.contains(&id) {
                ctx.ranges_set.insert(id);
                self.add_folding_range(FoldingRange {
                    start_line: node.start_position().row as u32,
                    end_line: node.end_position().row as u32 - 1,
                    ..Default::default()
                });
            }
        }
    }
}

impl FoldingFeature for Reactor {
    async fn on_folding_range(
        &self,
        _: tower_lsp_server::ls_types::FoldingRangeParams,
    ) -> tower_lsp_server::jsonrpc::Result<Option<Vec<FoldingRange>>> {
        Ok(Some(self.get_analysis().get_analyzed_folding_ranges()))
    }
}
