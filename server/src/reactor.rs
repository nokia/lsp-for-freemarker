// Copyright 2025-2026 Nokia
// Licensed under the BSD 3-Clause License.
// SPDX-License-Identifier: BSD-3-Clause

use tower_lsp_server::ls_types::{TextDocumentContentChangeEvent, Uri};

use crate::{
    analysis::Analysis,
    doc::{PositionEncodingKind, TextDocument},
    parser::TextParser,
};

#[derive(Debug)]
pub struct Reactor {
    pub(crate) version: i32,
    doc: TextDocument,
    parser: TextParser,
    analysis: Analysis,
}

impl Reactor {
    pub fn new(uri: &Uri, text: &str, version: i32) -> Self {
        let doc = TextDocument::new(uri, text);
        let parser = TextParser::new(text);
        let analysis = Analysis::new(&doc, &parser);
        Reactor {
            version,
            doc,
            parser,
            analysis,
        }
    }

    pub fn get_document(&self) -> &TextDocument {
        &self.doc
    }

    pub fn get_parser(&self) -> &TextParser {
        &self.parser
    }

    pub fn get_analysis(&self) -> &Analysis {
        &self.analysis
    }

    pub fn apply_content_change(&mut self, version: i32, change: &TextDocumentContentChangeEvent) {
        // always?
        self.version = version;
        //TODO: what if the document's encoding is not UTF8?
        if let Ok(edit) = self
            .doc
            .apply_content_change(change, PositionEncodingKind::UTF8)
        {
            self.parser.apply_edit(&self.doc.to_string(), edit);
            self.analysis = Analysis::new(&self.doc, &self.parser);
        }
    }
}
