// Copyright 2025-2026 Nokia
// Licensed under the BSD 3-Clause License.
// SPDX-License-Identifier: BSD-3-Clause

use tree_sitter::{InputEdit, Node, Parser, Point, Tree};

/// The syntax tree of a document. The underlying [`Parser`] is retained so that
/// edits can be re-parsed incrementally, reusing the previous tree.
pub struct SyntaxTree {
    parser: Parser,
    ast: Tree,
}

impl std::fmt::Debug for SyntaxTree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SyntaxTree").finish_non_exhaustive()
    }
}

impl SyntaxTree {
    /// Parses the given text into an initial syntax tree.
    pub fn parse(text: &str) -> Self {
        let mut parser = Self::parser_for_freemarker();
        let ast = parser
            .parse(text, None)
            .expect("the language is set, so parsing always yields a tree");
        SyntaxTree { parser, ast }
    }

    fn parser_for_freemarker() -> Parser {
        let mut parser = Parser::new();
        let language = tree_sitter_freemarker::LANGUAGE;
        parser
            .set_language(&language.into())
            .expect("set parser language should always succeed");
        parser
    }

    pub fn root_node(&self) -> Node<'_> {
        self.ast.root_node()
    }

    pub fn node_at(&self, point: Point) -> Option<Node<'_>> {
        self.ast
            .root_node()
            .named_descendant_for_point_range(point, point)
    }

    pub fn apply_edit(&mut self, text: &str, input_edit: Option<InputEdit>) {
        // TODO: what if the document's encoding is not UTF8?
        // Apply the incremental edit to the existing tree, then reuse it for an
        // incremental re-parse. A `None` edit means the whole document changed,
        // in which case we fall back to a full parse.
        let incremental = input_edit.is_some();
        if let Some(edit) = &input_edit {
            self.ast.edit(edit);
        }
        let old_tree = if incremental { Some(&self.ast) } else { None };
        self.ast = self
            .parser
            .parse(text, old_tree)
            .expect("the language is set, so re-parsing always yields a tree");
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use crate::grammar::Rule;

    use super::SyntaxTree;

    #[test]
    fn node_kinds_match_rule_enum() {
        let text = r#"<#assign greeting = "Hello">
<#if user.age >= 18>Adult<#else>Minor</#if>
<#list items as item>${item?upper_case}</#list>
<#import "lib.ftl" as lib>
<@lib.macro param=1 />
"#;
        let tree = SyntaxTree::parse(text);
        assert!(
            !tree.root_node().has_error(),
            "sample FTL should parse cleanly"
        );

        // Walk the tree, collecting every named node kind that maps to a Rule.
        let mut matched = std::collections::HashSet::new();
        let mut stack = vec![tree.root_node()];
        while let Some(node) = stack.pop() {
            if node.is_named() && Rule::from_str(node.kind()).is_ok() {
                matched.insert(node.kind().to_string());
            }
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i as u32) {
                    stack.push(child);
                }
            }
        }

        // Spot-check the structural node kinds the server's analysis relies on.
        // (Note: a bare `>=` inside `<#if ...>` terminates the tag at `>`, so
        // comparison operators are intentionally not asserted here.)
        for expected in [
            "assign_stmt",
            "if_stmt",
            "list_stmt",
            "import_stmt",
            "macro_call",
            "interpolation",
        ] {
            assert!(
                matched.contains(expected),
                "grammar should produce a node kind '{expected}' that maps to Rule; matched: {matched:?}"
            );
        }
    }
}
