// Copyright 2025-2026 Nokia
// Licensed under the BSD 3-Clause License.
// SPDX-License-Identifier: BSD-3-Clause

use std::sync::RwLock;

use tower_lsp_server::ls_types::{TextDocumentContentChangeEvent, Uri};

use crate::{
    semantic::SemanticModel,
    syntax::SyntaxTree,
    text::{PositionEncodingKind, SourceText},
};

/// A single open FreeMarker document: its source text, syntax tree, and a
/// lazily computed semantic model.
#[derive(Debug)]
pub struct Document {
    pub(crate) version: i32,
    source: SourceText,
    tree: SyntaxTree,
    semantic: RwLock<SemanticModel>,
}

impl Document {
    pub fn open(uri: &Uri, text: &str, version: i32) -> Self {
        Document {
            version,
            source: SourceText::new(uri, text),
            tree: SyntaxTree::parse(text),
            semantic: RwLock::new(SemanticModel::default()),
        }
    }

    pub fn source(&self) -> &SourceText {
        &self.source
    }

    pub fn tree(&self) -> &SyntaxTree {
        &self.tree
    }

    /// Returns the semantic model, building and caching it on first use.
    ///
    /// Building the model performs a short synchronous filesystem access for
    /// import canonicalization, which matches the previous behavior.
    pub fn semantic(&self) -> std::sync::RwLockReadGuard<'_, SemanticModel> {
        {
            let mut cache = self.semantic.write().expect("semantic cache poisoned");
            if !cache.built {
                // `build` takes the source and the tree rather than `self` so
                // that it can be called while the semantic cache is locked.
                *cache = SemanticModel::build(&self.source, &self.tree);
            }
        }
        self.semantic.read().expect("semantic cache poisoned")
    }

    /// Applies an incremental content change, re-parses the tree, and
    /// invalidates the cached semantic model. No analysis is computed eagerly.
    pub fn apply_change(&mut self, version: i32, change: &TextDocumentContentChangeEvent) {
        self.version = version;
        // TODO: what if the document's encoding is not UTF8?
        if let Ok(edit) = self
            .source
            .apply_content_change(change, PositionEncodingKind::UTF8)
        {
            self.tree.apply_edit(&self.source.to_string(), edit);
            if let Ok(cache) = self.semantic.get_mut() {
                *cache = SemanticModel::default();
            }
        }
    }
}
