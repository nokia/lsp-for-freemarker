// Copyright 2025-2026 Nokia
// Licensed under the BSD 3-Clause License.
// SPDX-License-Identifier: BSD-3-Clause

use tree_sitter::{InputEdit, Node, Parser, Point, Tree};

#[derive(Default, Debug)]
pub struct TextParser {
    //parser: Parser,
    ast: Option<Tree>,
}

impl TextParser {
    /// Creates a new document from the given text and language id. It creates
    /// a rope, parser and syntax tree from the text.
    pub fn new(text: &str) -> Self {
        let mut parser = Parser::new();
        let language = tree_sitter_freemarker::LANGUAGE;
        parser
            .set_language(&language.into())
            .expect("set parser language should always succeed");
        let ast = parser.parse(text, None);
        TextParser { ast }
    }

    pub fn get_ast(&self) -> Option<Tree> {
        self.ast.clone()
    }

    pub fn get_node_at_point(&self, point: Point) -> Option<Node<'_>> {
        if let Some(tree) = self.ast.as_ref() {
            return tree
                .root_node()
                .named_descendant_for_point_range(point, point);
        }
        None
    }

    pub fn apply_edit(&mut self, text: &str, input_edit: Option<InputEdit>) {
        //TODO: what if the document's encoding is not UTF8?
        let old_tree = self.ast.as_mut().unwrap();
        let mut parser = Parser::new();
        let language = tree_sitter_freemarker::LANGUAGE;
        parser
            .set_language(&language.into())
            .expect("set parser language should always succeed");
        self.ast = parser.parse(
            text,
            match input_edit {
                Some(edit) => {
                    old_tree.edit(&edit);
                    Some(old_tree)
                }
                _ => None,
            },
        );
    }
}
