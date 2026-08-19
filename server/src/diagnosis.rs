// Copyright 2025-2026 Nokia
// Licensed under the BSD 3-Clause License.
// SPDX-License-Identifier: BSD-3-Clause

use std::str::FromStr;

use crate::{
    consts::{SEMANTICS, SYNTAX},
    grammar::Rule,
    href::{
        COMPARISION_EXPRESSION, DIRECTIVE_ASSIGN, DIRECTIVE_IMPORT, DIRECTIVE_LIST_BREAK,
        TOPLEVEL_VARIABLE,
    },
};
use tower_lsp_server::{
    jsonrpc,
    ls_types::{
        CodeDescription, Diagnostic, DiagnosticOptions, DiagnosticServerCapabilities,
        DiagnosticSeverity, DocumentDiagnosticParams, DocumentDiagnosticReport,
        DocumentDiagnosticReportResult, FullDocumentDiagnosticReport, NumberOrString,
        RelatedFullDocumentDiagnosticReport,
    },
};
use tree_sitter::Node;

use crate::{document::Document, features::DiagnosticFeature, text::SourceText, utils};

pub fn diagnostic_capability() -> DiagnosticServerCapabilities {
    DiagnosticServerCapabilities::Options(DiagnosticOptions {
        identifier: None,
        inter_file_dependencies: true,
        workspace_diagnostics: false,
        work_done_progress_options: Default::default(),
    })
}

pub struct Scenario {
    severity: DiagnosticSeverity,
    code: &'static str,
    source: &'static str,
    message: &'static str,
    href: &'static str,
}

impl Scenario {
    pub const UNDEFINED_MACRO: Scenario = Scenario {
        severity: DiagnosticSeverity::ERROR,
        code: "undefined_macro",
        source: SEMANTICS,
        message: "Macro definition not found.",
        href: DIRECTIVE_IMPORT,
    };

    const BACKSLASHED_IDENTIFIER: Scenario = Scenario {
        severity: DiagnosticSeverity::INFORMATION,
        code: "identifier_has_backslash",
        source: SYNTAX,
        message: "Identifiers containing reserved characters require escaping with a backslash (\\), which can significantly reduce readability. Consider refactoring to avoid such identifiers.",
        href: TOPLEVEL_VARIABLE,
    };

    const STRING_LVALUE: Scenario = Scenario {
        severity: DiagnosticSeverity::WARNING,
        code: "string_lvalue",
        source: SYNTAX,
        message: "While using a string literal as an L-value is syntactically valid for <#assign> and <#local>, this practice is generally discouraged due to potential ambiguity and reduced maintainability.",
        href: DIRECTIVE_ASSIGN,
    };

    const LEGACY_EQUAL_OPERATOR: Scenario = Scenario {
        severity: DiagnosticSeverity::WARNING,
        code: "legacy_equal_operator",
        source: SYNTAX,
        message: "For equality checks in comparisons, use '=='. The single '=' operator is deprecated for this purpose.",
        href: COMPARISION_EXPRESSION,
    };

    const SELF_CLOSING_TAG: Scenario = Scenario {
        severity: DiagnosticSeverity::WARNING,
        code: "self_closing_tag",
        source: SYNTAX,
        message: "For non-capture <#assign> directives, it is recommended to use '>' as the close tag. Using '/>' is undocumented and adds unnecessary characters.",
        href: DIRECTIVE_ASSIGN,
    };

    const DEPRECATED_LIST_BREAK: Scenario = Scenario {
        severity: DiagnosticSeverity::WARNING,
        code: "deprecated_list_break",
        source: SYNTAX,
        message: "<#break> is deprecated for most list-related use cases, as it can interfere with <#sep> and item?has_next. Instead, consider using sequence?take_while(predicate) to filter the sequence before iteration.",
        href: DIRECTIVE_LIST_BREAK,
    };

    const UNEXPECTED_BREAK_STMT: Scenario = Scenario {
        severity: DiagnosticSeverity::ERROR,
        code: "unexpected_break_stmt",
        source: SYNTAX,
        message: "The <#break> directive can only be used within <#list> or <#switch> blocks.",
        href: DIRECTIVE_LIST_BREAK,
    };
}

impl From<Scenario> for Diagnostic {
    fn from(s: Scenario) -> Self {
        Diagnostic {
            severity: Some(s.severity),
            code: Some(NumberOrString::String(s.code.to_owned())),
            code_description: Some(CodeDescription {
                href: s
                    .href
                    .parse()
                    .expect("static scenario href must be a valid url"),
            }),
            source: Some(s.source.to_owned()),
            message: s.message.to_owned(),
            ..Default::default()
        }
    }
}

/// Computes the syntax-level diagnostics by walking the tree once. Semantic
/// (cross-referencing) diagnostics come from the [`crate::semantic::SemanticModel`].
pub(crate) fn syntax_diagnostics(doc: &Document) -> Vec<Diagnostic> {
    let source = doc.source();
    let tree = doc.tree();
    fn collect(
        source: &SourceText,
        node: &Node,
        scope: &mut Vec<Rule>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let node_kind = node.kind();
        let range = utils::parser_node_to_document_range(node);
        // TODO: maybe use tree-sitter query in the future
        if node.is_missing() {
            // TODO : maybe use query in the future
            diagnostics.push(Diagnostic {
                range,
                severity: Some(DiagnosticSeverity::ERROR),
                source: Some(SYNTAX.to_owned()),
                message: format!("Missing {} here", node_kind),
                ..Default::default()
            });
        }

        if node.is_error() {
            let node_text = source.get_ranged_text(node.start_byte()..node.end_byte());
            diagnostics.push(Diagnostic {
                range,
                severity: Some(DiagnosticSeverity::ERROR),
                source: Some(SYNTAX.to_owned()),
                message: format!("ERROR: Unexpected '{}'.\n", node_text),
                ..Default::default()
            });
        }

        if let Ok(rule) = Rule::from_str(node_kind) {
            match rule {
                Rule::Identifier => {
                    let node_text = source.get_ranged_text(node.start_byte()..node.end_byte());
                    if node_text.contains("\\") {
                        diagnostics.push(Diagnostic {
                            range,
                            ..Scenario::BACKSLASHED_IDENTIFIER.into()
                        });
                    }
                }
                Rule::StringLvalue => {
                    diagnostics.push(Diagnostic {
                        range,
                        ..Scenario::STRING_LVALUE.into()
                    });
                }
                Rule::LegacyEqualOperator => {
                    diagnostics.push(Diagnostic {
                        range,
                        ..Scenario::LEGACY_EQUAL_OPERATOR.into()
                    });
                }
                Rule::SelfClosingTag => {
                    diagnostics.push(Diagnostic {
                        range,
                        ..Scenario::SELF_CLOSING_TAG.into()
                    });
                }
                Rule::ListBegin | Rule::SwitchBegin => {
                    scope.push(rule);
                }
                Rule::ListClose | Rule::SwitchClose => {
                    scope.pop();
                }
                Rule::BreakStmt => match scope.last() {
                    Some(scope_rule) => {
                        if *scope_rule == Rule::ListBegin {
                            diagnostics.push(Diagnostic {
                                range,
                                ..Scenario::DEPRECATED_LIST_BREAK.into()
                            })
                        }
                    }
                    None => diagnostics.push(Diagnostic {
                        range,
                        ..Scenario::UNEXPECTED_BREAK_STMT.into()
                    }),
                },
                _ => {}
            }
        }
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i as u32) {
                collect(source, &child, scope, diagnostics);
            }
        }
    }

    let mut diagnostics = Vec::new();
    let mut scope = Vec::new();
    collect(source, &tree.root_node(), &mut scope, &mut diagnostics);
    diagnostics
}

impl DiagnosticFeature for Document {
    async fn on_diagnostic(
        &self,
        _: DocumentDiagnosticParams,
    ) -> jsonrpc::Result<DocumentDiagnosticReportResult> {
        // TODO: Unchanged support
        let mut items = syntax_diagnostics(self);
        items.extend(self.semantic().diagnostics().iter().cloned());
        Ok(DocumentDiagnosticReportResult::Report(
            DocumentDiagnosticReport::Full(RelatedFullDocumentDiagnosticReport {
                full_document_diagnostic_report: FullDocumentDiagnosticReport {
                    result_id: None,
                    items,
                },
                ..Default::default()
            }),
        ))
    }
}
