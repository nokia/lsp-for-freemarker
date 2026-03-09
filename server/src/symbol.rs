// Copyright 2025-2026 Nokia
// Licensed under the BSD 3-Clause License.
// SPDX-License-Identifier: BSD-3-Clause

use std::{path::PathBuf, str::FromStr};

use tower_lsp_server::ls_types::{
    CodeDescription, Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity, Location,
    NumberOrString, Range, Uri,
};
use tree_sitter::Node;
use tree_sitter_freemarker::href::DIRECTIVE_IMPORT;
use tree_sitter_freemarker::{SEMANTICS, grammar::Rule};

use crate::diagnosis::Scenario;
use crate::{
    analysis::{Analysis, AnalysisContext, Symbol, SymbolAnalysis},
    doc::TextDocument,
    utils,
};

struct ImportWarning(&'static str, &'static str);

impl ImportWarning {
    const PATH_DUPLICATED: Self = ImportWarning("path_duplicated", "import path is dupicated");

    pub fn build(
        &self,
        range: Range,
        related_information: Option<Vec<DiagnosticRelatedInformation>>,
    ) -> Diagnostic {
        Diagnostic {
            range,
            severity: Some(DiagnosticSeverity::WARNING),
            code: Some(NumberOrString::String(self.0.to_owned())),
            code_description: Some(CodeDescription {
                href: DIRECTIVE_IMPORT.parse().unwrap(),
            }),
            source: Some(SEMANTICS.to_owned()),
            message: self.1.to_owned(),
            related_information,
            ..Default::default()
        }
    }
}

struct ImportError(&'static str, &'static str);

impl ImportError {
    const PATH_UNCANONICAL: Self = ImportError("path_uncanonical", "import path is uncanonical");
    const PATH_NOT_FILE: Self = ImportError("path_not_file", "import path is not a file");
    const PATH_NOT_EXISTS: Self = ImportError("path_not_exists", "import path is not exists");
    const PATH_REF_SELF: Self = ImportError("path_refer_itself", "import path refers to itself");

    pub fn build(
        &self,
        range: Range,
        related_information: Option<Vec<DiagnosticRelatedInformation>>,
    ) -> Diagnostic {
        Diagnostic {
            range,
            severity: Some(DiagnosticSeverity::ERROR),
            code: Some(NumberOrString::String(self.0.to_owned())),
            code_description: Some(CodeDescription {
                href: DIRECTIVE_IMPORT.parse().unwrap(),
            }),
            source: Some(SEMANTICS.to_owned()),
            message: self.1.to_owned(),
            related_information,
            ..Default::default()
        }
    }
}

fn analyze_import_statement(
    import_node: &Node,
    doc: &TextDocument,
    ctx: &mut AnalysisContext,
    analysis: &mut Analysis,
) {
    // "import as" alias
    let alias_node = import_node
        .child_by_field_name(Rule::ImportAlias.to_string())
        .unwrap();
    let alias_range = utils::parser_node_to_document_range(&alias_node);
    let import_alias = doc.get_ranged_text(alias_node.start_byte()..alias_node.end_byte());
    analysis.add_symbol(
        &import_alias,
        Symbol {
            rule: Rule::ImportAlias,
            start_byte: alias_node.start_byte(),
            end_byte: alias_node.end_byte(),
            range: alias_range,
        },
    );

    // import path
    let path_node = import_node
        .child_by_field_name(Rule::ImportPath.to_string())
        .unwrap();
    let path_range = utils::parser_node_to_document_range(&path_node);
    // the tree-sitter parser had ensured the import_path is '"' quoted, so it is safe to slice like this [1..len()-1]
    let import_path_str = doc.get_ranged_text(path_node.start_byte() + 1..path_node.end_byte() - 1);
    let import_path_buf = PathBuf::from(&import_path_str);
    let canonicalize_import = match import_path_buf.is_absolute() {
        true => import_path_buf.canonicalize(),
        false => doc.dir().join(import_path_buf).canonicalize(),
    };

    match canonicalize_import {
        Ok(canonicalize_import_path) => {
            if !canonicalize_import_path.is_file() {
                // import must be a file
                analysis.add_diagnostic(ImportError::PATH_NOT_FILE.build(path_range, None));
            } else if !canonicalize_import_path.exists() {
                // import must exists
                analysis.add_diagnostic(ImportError::PATH_NOT_EXISTS.build(path_range, None));
            } else if doc.canonical_uri() == canonicalize_import_path {
                // don't import yourself
                analysis.add_diagnostic(ImportError::PATH_REF_SELF.build(path_range, None));
            }
            //
            let canonicalize_import_str = canonicalize_import_path.to_str().unwrap();
            ctx.import_map
                .entry(canonicalize_import_str.to_string())
                .and_modify(|symbols| {
                    let first_definition = symbols[0];
                    // import path duplicated
                    analysis.add_diagnostic(ImportWarning::PATH_DUPLICATED.build(
                        path_range,
                        Some(vec![DiagnosticRelatedInformation {
                            location: Location {
                                uri: doc.uri(),
                                range: first_definition.range,
                            },
                            message: "first imported here".to_owned(),
                        }]),
                    ));
                })
                .or_insert_with(|| {
                    analysis.record_valid_import(
                        &import_path_str, // record original text as key
                        Uri::from_file_path(&canonicalize_import_path).unwrap(),
                    );
                    vec![Symbol {
                        rule: Rule::ImportPath,
                        start_byte: path_node.start_byte(),
                        end_byte: path_node.end_byte(),
                        range: path_range,
                    }]
                });
        }
        Err(_) => {
            analysis.add_diagnostic(ImportError::PATH_UNCANONICAL.build(path_range, None));
        }
    }
}

fn analyze_macro_statement(
    macro_node: &Node,
    doc: &TextDocument,
    _: &mut AnalysisContext,
    analysis: &mut Analysis,
) {
    // "import as" alias
    let name_node = macro_node
        .child_by_field_name(Rule::MacroName.to_string())
        .unwrap();
    let name_range = utils::parser_node_to_document_range(&name_node);
    let name_text = doc.get_ranged_text(name_node.start_byte()..name_node.end_byte());
    analysis.add_symbol(
        &name_text,
        Symbol {
            rule: Rule::MacroName,
            start_byte: name_node.start_byte(),
            end_byte: name_node.end_byte(),
            range: name_range,
        },
    );
}

impl SymbolAnalysis for Analysis {
    fn analyze_syntatic_symbols(
        &mut self,
        node: &Node,
        doc: &TextDocument,
        ctx: &mut AnalysisContext,
    ) {
        let rule = Rule::from_str(node.kind());
        if rule.is_err() {
            return;
        }
        match rule.unwrap() {
            Rule::ImportStmt => {
                analyze_import_statement(node, doc, ctx, self);
            }
            Rule::MacroStmt => {
                analyze_macro_statement(node, doc, ctx, self);
            }
            _ => {}
        }
    }

    fn post_syntatic_analysis(&mut self, doc: &TextDocument, ctx: &mut AnalysisContext) {
        // check duplicated symbols
        let mut duplicated_symbols = vec![];
        self.foreach_symbol(|_, symbols| {
            if symbols.len() > 1 {
                let first_definition = symbols[0];
                for redefinition in symbols.iter().skip(1) {
                    duplicated_symbols.push(Diagnostic {
                        range: redefinition.range,
                        severity: Some(DiagnosticSeverity::ERROR),
                        code: Some(NumberOrString::String("duplicated_symbol".to_owned())),
                        source: Some(SEMANTICS.to_owned()),
                        message: "redefinition of symbol".to_owned(),
                        related_information: Some(vec![DiagnosticRelatedInformation {
                            location: Location {
                                uri: doc.uri(),
                                range: first_definition.range,
                            },
                            message: "first defined here".to_owned(),
                        }]),
                        ..Default::default()
                    });
                }
            }
        });
        self.add_diagnostics(duplicated_symbols);
        // check undefined macro calls
        ctx.macro_call_map
            .iter()
            .for_each(|(call_name, call_symbols)| {
                if self.find_symbol_definition(call_name).is_err() {
                    call_symbols.iter().for_each(|sym| {
                        self.add_diagnostic(Diagnostic {
                            range: sym.range,
                            ..Scenario::UNDEFINED_MACRO.into()
                        })
                    })
                }
            });
    }
}
