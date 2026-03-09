// Copyright 2025-2026 Nokia
// Licensed under the BSD 3-Clause License.
// SPDX-License-Identifier: BSD-3-Clause

use std::collections::{HashMap, HashSet};

use thiserror::Error;
use tower_lsp_server::ls_types::{
    Diagnostic, FoldingRange, Range, RelatedFullDocumentDiagnosticReport, SemanticToken, Uri,
};
use tree_sitter::{Node, Point};
use tree_sitter_freemarker::grammar::Rule;

use crate::{doc::TextDocument, parser::TextParser};

#[derive(Clone, Copy, Debug)]
pub struct Symbol {
    pub(crate) rule: Rule,
    pub(crate) start_byte: usize,
    pub(crate) end_byte: usize,
    pub(crate) range: Range,
}

#[derive(Default)]
pub struct AnalysisContext {
    pub prev_start: Point,
    pub ranges_set: HashSet<usize>,
    pub scope: Vec<Rule>,
    pub import_map: HashMap<String, Vec<Symbol>>,
    pub macro_call_map: HashMap<String, Vec<Symbol>>,
}

#[derive(Error, Debug)]
pub enum AnalysisError {
    #[error("symbol {0} is undefined")]
    Undefined(String),
}

#[derive(Clone, Default, Debug)]
pub struct Analysis {
    semantic_tokens: Vec<SemanticToken>,
    full_diagnostic: RelatedFullDocumentDiagnosticReport,
    folding_range: Vec<FoldingRange>,
    symbol_map: HashMap<String, Vec<Symbol>>,
    import_uri_map: HashMap<String, Uri>,
}

// TODO: wrap parser methods and document methods
impl Analysis {
    pub fn new(doc: &TextDocument, parser: &TextParser) -> Self {
        let mut analysis = Analysis {
            ..Default::default()
        };
        let mut ctx = AnalysisContext {
            ..Default::default()
        };
        let ast = parser.get_ast().unwrap();
        analysis.syntatic_analysis(&ast.root_node(), doc, &mut ctx);
        analysis.post_syntatic_analysis(doc, &mut ctx);
        analysis
    }

    fn syntatic_analysis(&mut self, node: &Node, doc: &TextDocument, ctx: &mut AnalysisContext) {
        // semantic highlight
        self.analyze_semantic_highlight(node, doc, ctx);
        // folding range
        self.analyze_folding_ranges(node, ctx);
        // symbols
        self.analyze_syntatic_symbols(node, doc, ctx);
        // diagnostics
        self.analyze_diagnostic_report(node, doc, ctx);
        // Perform a DFS traversing
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.syntatic_analysis(&child, doc, ctx)
            }
        }
    }

    pub fn add_symbol(&mut self, name: &str, symbol: Symbol) {
        self.symbol_map
            .entry(name.to_owned())
            .and_modify(|e| e.push(symbol))
            .or_insert(vec![symbol]);
    }

    pub fn foreach_symbol<F>(&self, mut func: F)
    where
        F: FnMut(&str, &Vec<Symbol>),
    {
        for (name, symbols) in &self.symbol_map {
            func(name, symbols)
        }
    }

    pub fn find_symbol_definition(&self, name: &str) -> Result<&Vec<Symbol>, AnalysisError> {
        match self.symbol_map.get(name) {
            Some(symbols) => Ok(symbols),
            None => Err(AnalysisError::Undefined(name.to_owned())),
        }
    }

    pub fn record_valid_import(&mut self, path: &str, uri: Uri) {
        self.import_uri_map.insert(path.to_owned(), uri);
    }

    pub fn get_valid_import(&self, path: &str) -> Option<&Uri> {
        self.import_uri_map.get(path)
    }

    pub fn add_diagnostic(&mut self, item: Diagnostic) {
        self.full_diagnostic
            .full_document_diagnostic_report
            .items
            .push(item);
    }

    pub fn add_diagnostics(&mut self, items: Vec<Diagnostic>) {
        self.full_diagnostic
            .full_document_diagnostic_report
            .items
            .extend(items);
    }

    pub fn add_folding_range(&mut self, range: FoldingRange) {
        self.folding_range.push(range);
    }

    pub fn add_semantic_tokens(&mut self, tokens: Vec<SemanticToken>) {
        self.semantic_tokens.extend(tokens);
    }

    // For LSP responses
    pub fn get_analyzed_full_diagnostics(&self) -> RelatedFullDocumentDiagnosticReport {
        self.full_diagnostic.clone()
    }

    pub fn get_analyzed_folding_ranges(&self) -> Vec<FoldingRange> {
        self.folding_range.clone()
    }

    pub fn get_analyzed_semantic_tokens(&self) -> Vec<SemanticToken> {
        self.semantic_tokens.clone()
    }
}

pub trait FoldingAnalysis {
    fn analyze_folding_ranges(&mut self, node: &Node, ctx: &mut AnalysisContext);
}

pub trait HighlightAnalysis {
    fn analyze_semantic_highlight(
        &mut self,
        node: &Node,
        doc: &TextDocument,
        ctx: &mut AnalysisContext,
    );
}

pub trait SymbolAnalysis {
    fn analyze_syntatic_symbols(
        &mut self,
        node: &Node,
        doc: &TextDocument,
        ctx: &mut AnalysisContext,
    );

    fn post_syntatic_analysis(&mut self, doc: &TextDocument, ctx: &mut AnalysisContext);
}

pub trait DiagnosticAnalysis {
    fn analyze_diagnostic_report(
        &mut self,
        node: &Node,
        doc: &TextDocument,
        ctx: &mut AnalysisContext,
    );
}
