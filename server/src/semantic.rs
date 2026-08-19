// Copyright 2025-2026 Nokia
// Licensed under the BSD 3-Clause License.
// SPDX-License-Identifier: BSD-3-Clause

//! The semantic model of a document: symbols, imports, and the diagnostics
//! that require cross-referencing (import validation, duplicated symbols,
//! undefined macro calls). It is built once per parse and shared by the
//! goto/hover/completion features and the diagnostics pass.

use std::{collections::HashMap, path::PathBuf, str::FromStr};

use tower_lsp_server::ls_types::{
    CodeDescription, Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity, Location,
    NumberOrString, Range, Uri,
};
use tree_sitter::Node;

use crate::{
    consts::SEMANTICS, diagnosis::Scenario, grammar::Rule, href::DIRECTIVE_IMPORT,
    syntax::SyntaxTree, text::SourceText, utils,
};

#[derive(Clone, Copy, Debug)]
pub struct Symbol {
    pub(crate) rule: Rule,
    pub(crate) start_byte: usize,
    pub(crate) end_byte: usize,
    pub(crate) range: Range,
}

/// State accumulated while walking the tree once to build the model.
#[derive(Default)]
struct BuildContext {
    /// Import paths already seen, for the "import path duplicated" warning.
    import_map: HashMap<String, Vec<Symbol>>,
    /// Macro call sites by namespace name, for the "undefined macro" error.
    macro_call_map: HashMap<String, Vec<Symbol>>,
}

/// The semantic analysis of a document.
///
/// `built` is a cache flag owned by [`crate::document::Document`]; a default
/// model means "not built yet".
#[derive(Default, Debug)]
pub struct SemanticModel {
    pub(crate) built: bool,
    symbol_map: HashMap<String, Vec<Symbol>>,
    import_uri_map: HashMap<String, Uri>,
    diagnostics: Vec<Diagnostic>,
}

impl SemanticModel {
    /// Builds the model by walking the syntax tree once.
    pub fn build(source: &SourceText, tree: &SyntaxTree) -> Self {
        let mut model = SemanticModel {
            built: true,
            ..Default::default()
        };
        let mut ctx = BuildContext::default();
        Self::collect(&tree.root_node(), source, &mut model, &mut ctx);
        Self::post_process(source, &mut model, &ctx);
        model
    }

    fn collect(node: &Node, source: &SourceText, model: &mut Self, ctx: &mut BuildContext) {
        if let Ok(rule) = Rule::from_str(node.kind()) {
            match rule {
                Rule::ImportStmt => collect_import_statement(node, source, ctx, model),
                Rule::MacroStmt => collect_macro_statement(node, source, model),
                Rule::MacroNamespace => {
                    let node_text = source.get_ranged_text(node.start_byte()..node.end_byte());
                    let macro_call = Symbol {
                        rule,
                        start_byte: node.start_byte(),
                        end_byte: node.end_byte(),
                        range: utils::parser_node_to_document_range(node),
                    };
                    ctx.macro_call_map
                        .entry(node_text)
                        .and_modify(|calls| calls.push(macro_call))
                        .or_insert(vec![macro_call]);
                }
                _ => {}
            }
        }
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i as u32) {
                Self::collect(&child, source, model, ctx);
            }
        }
    }

    fn post_process(source: &SourceText, model: &mut Self, ctx: &BuildContext) {
        // duplicated symbols
        for symbols in model.symbol_map.values() {
            if symbols.len() > 1 {
                let first = symbols[0];
                for redefinition in symbols.iter().skip(1) {
                    model.diagnostics.push(Diagnostic {
                        range: redefinition.range,
                        severity: Some(DiagnosticSeverity::ERROR),
                        code: Some(NumberOrString::String("duplicated_symbol".to_owned())),
                        source: Some(SEMANTICS.to_owned()),
                        message: "redefinition of symbol".to_owned(),
                        related_information: Some(vec![DiagnosticRelatedInformation {
                            location: Location {
                                uri: source.uri(),
                                range: first.range,
                            },
                            message: "first defined here".to_owned(),
                        }]),
                        ..Default::default()
                    });
                }
            }
        }
        // undefined macro calls
        for (call_name, call_symbols) in &ctx.macro_call_map {
            if model.find_symbol_definition(call_name).is_none() {
                for sym in call_symbols {
                    model.diagnostics.push(Diagnostic {
                        range: sym.range,
                        ..Scenario::UNDEFINED_MACRO.into()
                    });
                }
            }
        }
    }

    pub fn foreach_symbol<F>(&self, mut func: F)
    where
        F: FnMut(&str, &Vec<Symbol>),
    {
        for (name, symbols) in &self.symbol_map {
            func(name, symbols)
        }
    }

    pub fn find_symbol_definition(&self, name: &str) -> Option<&Vec<Symbol>> {
        self.symbol_map.get(name)
    }

    pub fn get_valid_import(&self, path: &str) -> Option<&Uri> {
        self.import_uri_map.get(path)
    }

    /// Cross-referencing diagnostics (imports, duplicated symbols, undefined
    /// macro calls).
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    fn add_symbol(&mut self, name: &str, symbol: Symbol) {
        self.symbol_map
            .entry(name.to_owned())
            .and_modify(|e| e.push(symbol))
            .or_insert(vec![symbol]);
    }
}

struct ImportWarning(&'static str, &'static str);

impl ImportWarning {
    const PATH_DUPLICATED: Self = ImportWarning("path_duplicated", "import path is dupicated");

    fn build(
        &self,
        range: Range,
        related_information: Option<Vec<DiagnosticRelatedInformation>>,
    ) -> Diagnostic {
        Diagnostic {
            range,
            severity: Some(DiagnosticSeverity::WARNING),
            code: Some(NumberOrString::String(self.0.to_owned())),
            code_description: Some(CodeDescription {
                href: DIRECTIVE_IMPORT
                    .parse()
                    .expect("DIRECTIVE_IMPORT is a static valid url"),
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

    fn build(
        &self,
        range: Range,
        related_information: Option<Vec<DiagnosticRelatedInformation>>,
    ) -> Diagnostic {
        Diagnostic {
            range,
            severity: Some(DiagnosticSeverity::ERROR),
            code: Some(NumberOrString::String(self.0.to_owned())),
            code_description: Some(CodeDescription {
                href: DIRECTIVE_IMPORT
                    .parse()
                    .expect("DIRECTIVE_IMPORT is a static valid url"),
            }),
            source: Some(SEMANTICS.to_owned()),
            message: self.1.to_owned(),
            related_information,
            ..Default::default()
        }
    }
}

fn collect_import_statement(
    import_node: &Node,
    source: &SourceText,
    ctx: &mut BuildContext,
    model: &mut SemanticModel,
) {
    // "import as" alias
    let Some(alias_node) = import_node.child_by_field_name(Rule::ImportAlias.to_string()) else {
        return;
    };
    let alias_range = utils::parser_node_to_document_range(&alias_node);
    let import_alias = source.get_ranged_text(alias_node.start_byte()..alias_node.end_byte());
    model.add_symbol(
        &import_alias,
        Symbol {
            rule: Rule::ImportAlias,
            start_byte: alias_node.start_byte(),
            end_byte: alias_node.end_byte(),
            range: alias_range,
        },
    );

    // import path
    let Some(path_node) = import_node.child_by_field_name(Rule::ImportPath.to_string()) else {
        return;
    };
    let path_range = utils::parser_node_to_document_range(&path_node);
    // the tree-sitter parser had ensured the import_path is '"' quoted, so it is safe to slice like this [1..len()-1]
    let import_path_str =
        source.get_ranged_text(path_node.start_byte() + 1..path_node.end_byte() - 1);
    let import_path_buf = PathBuf::from(&import_path_str);
    // A `None` result (e.g. the uri is not a file or the path cannot be
    // canonicalized) is reported as an uncanonical import below.
    let canonicalize_import = match import_path_buf.is_absolute() {
        true => import_path_buf.canonicalize().ok(),
        false => source
            .dir()
            .ok()
            .and_then(|dir| dir.join(import_path_buf).canonicalize().ok()),
    };

    match canonicalize_import {
        Some(canonicalize_import_path) => {
            if !canonicalize_import_path.is_file() {
                // import must be a file
                model
                    .diagnostics
                    .push(ImportError::PATH_NOT_FILE.build(path_range, None));
            } else if !canonicalize_import_path.exists() {
                // import must exists
                model
                    .diagnostics
                    .push(ImportError::PATH_NOT_EXISTS.build(path_range, None));
            } else if source
                .canonical_uri()
                .is_ok_and(|canonical_uri| canonical_uri == canonicalize_import_path)
            {
                // don't import yourself
                model
                    .diagnostics
                    .push(ImportError::PATH_REF_SELF.build(path_range, None));
            }
            let canonicalize_import_str = canonicalize_import_path.to_string_lossy();
            ctx.import_map
                .entry(canonicalize_import_str.to_string())
                .and_modify(|symbols| {
                    let first_definition = symbols[0];
                    // import path duplicated
                    model.diagnostics.push(ImportWarning::PATH_DUPLICATED.build(
                        path_range,
                        Some(vec![DiagnosticRelatedInformation {
                            location: Location {
                                uri: source.uri(),
                                range: first_definition.range,
                            },
                            message: "first imported here".to_owned(),
                        }]),
                    ));
                })
                .or_insert_with(|| {
                    if let Some(uri) = Uri::from_file_path(&canonicalize_import_path) {
                        model.import_uri_map.insert(import_path_str.clone(), uri);
                    }
                    vec![Symbol {
                        rule: Rule::ImportPath,
                        start_byte: path_node.start_byte(),
                        end_byte: path_node.end_byte(),
                        range: path_range,
                    }]
                });
        }
        None => {
            model
                .diagnostics
                .push(ImportError::PATH_UNCANONICAL.build(path_range, None));
        }
    }
}

fn collect_macro_statement(macro_node: &Node, source: &SourceText, model: &mut SemanticModel) {
    // "import as" alias
    let Some(name_node) = macro_node.child_by_field_name(Rule::MacroName.to_string()) else {
        return;
    };
    let name_range = utils::parser_node_to_document_range(&name_node);
    let name_text = source.get_ranged_text(name_node.start_byte()..name_node.end_byte());
    model.add_symbol(
        &name_text,
        Symbol {
            rule: Rule::MacroName,
            start_byte: name_node.start_byte(),
            end_byte: name_node.end_byte(),
            range: name_range,
        },
    );
}
