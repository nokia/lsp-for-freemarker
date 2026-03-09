// Copyright 2025-2026 Nokia
// Licensed under the BSD 3-Clause License.
// SPDX-License-Identifier: BSD-3-Clause

//! https://gist.github.com/rojas-diego/04d9c4e3fff5f8374f29b9b738d541ef

use std::{ops::Range, path::PathBuf};

use ropey::{Rope, RopeSlice};
use thiserror::Error;
use tower_lsp_server::ls_types::{Position, TextDocumentContentChangeEvent, Uri};
use tree_sitter::{InputEdit, Point};

#[derive(Debug)]
pub struct TextDocument {
    uri: Uri,
    pub rope: Rope,
}

#[derive(Error, Debug)]
pub enum DocumentError {
    #[error("position {0}:{1} is out of bounds")]
    PositionOutOfBounds(u32, u32),
    #[error("line index {0} is out of bounds")]
    LineIndexOutOfBounds(usize),
}

#[derive(Clone, Debug, Copy)]
/// We redeclare this enum here because the `lsp_types` crate exports a Cow
/// type that is unconvenient to deal with.
pub enum PositionEncodingKind {
    UTF8,
    #[allow(dead_code)]
    UTF16,
    #[allow(dead_code)]
    UTF32,
}

impl std::fmt::Display for TextDocument {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.rope.fmt(f)
    }
}

impl TextDocument {
    /// Creates a new document from the given text and language id. It creates
    /// a rope, parser and syntax tree from the text.
    pub fn new(uri: &Uri, text: &str) -> Self {
        TextDocument {
            uri: uri.clone(),
            rope: Rope::from_str(text),
        }
    }

    pub fn uri(&self) -> Uri {
        self.uri.clone()
    }

    pub fn canonical_uri(&self) -> PathBuf {
        let filepath = self.uri.to_file_path().unwrap();
        filepath.canonicalize().unwrap()
    }

    pub fn dir(&self) -> PathBuf {
        let filepath = self.uri.to_file_path().unwrap();
        let parent = filepath.parent().unwrap();
        parent.to_path_buf()
    }

    pub fn line_count(&self) -> usize {
        self.rope.len_lines()
    }

    pub fn enumerate_lines<F>(&self, mut func: F)
    where
        F: FnMut(usize, &str),
    {
        for (index, line_rope) in self.rope.lines().enumerate() {
            let line = line_rope.to_string().trim_end().to_string();
            func(index, &line)
        }
    }

    pub fn get_ranged_text(&self, range: Range<usize>) -> String {
        let source = self.rope.to_string();
        source[range.start..range.end].to_owned()
    }

    pub fn get_line_text(&self, index: usize) -> String {
        let line = self.rope.line(index);
        line.to_string()
    }

    pub fn get_prev_char_at(&self, position: &Position) -> Option<char> {
        if let Some(line) = self.rope.get_line(position.line as usize)
            && position.character > 0
        {
            return line.get_char(position.character as usize - 1);
        }
        None
    }

    pub fn line_len(&self, id: usize) -> Result<usize, DocumentError> {
        match self.rope.get_line(id) {
            Some(line) => Ok(line.len_chars()),
            None => Err(DocumentError::LineIndexOutOfBounds(id)),
        }
    }

    /// Apply a change to the document.
    pub fn apply_content_change(
        &mut self,
        change: &TextDocumentContentChangeEvent,
        position_encoding: PositionEncodingKind,
    ) -> Result<Option<InputEdit>, DocumentError> {
        match change.range {
            Some(range) => {
                assert!(
                    range.start.line < range.end.line
                        || (range.start.line == range.end.line
                            && range.start.character <= range.end.character)
                );

                let same_line = range.start.line == range.end.line;
                let same_character = range.start.character == range.end.character;

                let change_start_line_cu_idx = range.start.character as usize;
                let change_end_line_cu_idx = range.end.character as usize;

                // 1. Get the line at which the change starts.
                let change_start_line_idx = range.start.line as usize;
                let change_start_line = match self.rope.get_line(change_start_line_idx) {
                    Some(line) => line,
                    None => {
                        return Err(DocumentError::PositionOutOfBounds(
                            range.start.line,
                            range.start.character,
                        ));
                    }
                };

                // 2. Get the line at which the change ends. (Small optimization
                // where we first check whether start and end line are the
                // same O(log N) lookup. We repeat this all throughout this
                // function).
                let change_end_line_idx = range.end.line as usize;
                let change_end_line = match same_line {
                    true => change_start_line,
                    false => match self.rope.get_line(change_end_line_idx) {
                        Some(line) => line,
                        None => {
                            return Err(DocumentError::PositionOutOfBounds(
                                range.end.line,
                                range.end.character,
                            ));
                        }
                    },
                };

                fn compute_char_idx(
                    position_encoding: PositionEncodingKind,
                    position: &Position,
                    slice: &RopeSlice,
                ) -> Result<usize, DocumentError> {
                    match position_encoding {
                        PositionEncodingKind::UTF8 => {
                            slice.try_byte_to_char(position.character as usize)
                        }
                        PositionEncodingKind::UTF16 => {
                            slice.try_utf16_cu_to_char(position.character as usize)
                        }
                        PositionEncodingKind::UTF32 => Ok(position.character as usize),
                    }
                    .map_err(|_| {
                        DocumentError::PositionOutOfBounds(position.line, position.character)
                    })
                }

                // 3. Compute the character offset into the start/end line where
                // the change starts/ends.
                let change_start_line_char_idx =
                    compute_char_idx(position_encoding, &range.start, &change_start_line)?;
                let change_end_line_char_idx = match same_line && same_character {
                    true => change_start_line_char_idx,
                    false => compute_char_idx(position_encoding, &range.end, &change_end_line)?,
                };

                // 4. Compute the character and byte offset into the document
                // where the change starts/ends.
                let change_start_doc_char_idx =
                    self.rope.line_to_char(change_start_line_idx) + change_start_line_char_idx;
                let change_end_doc_char_idx = match same_line && same_character {
                    true => change_start_doc_char_idx,
                    false => self.rope.line_to_char(change_end_line_idx) + change_end_line_char_idx,
                };
                let change_start_doc_byte_idx = self.rope.char_to_byte(change_start_doc_char_idx);
                let change_end_doc_byte_idx = match same_line && same_character {
                    true => change_start_doc_byte_idx,
                    false => self.rope.char_to_byte(change_end_doc_char_idx),
                };

                // 5. Compute the byte offset into the start/end line where the
                // change starts/ends. Required for tree-sitter.
                let change_start_line_byte_idx = match position_encoding {
                    PositionEncodingKind::UTF8 => change_start_line_cu_idx,
                    PositionEncodingKind::UTF16 => {
                        change_start_line.char_to_utf16_cu(change_start_line_char_idx)
                    }
                    PositionEncodingKind::UTF32 => change_start_line_char_idx,
                };
                let change_end_line_byte_idx = match same_line && same_character {
                    true => change_start_line_byte_idx,
                    false => match position_encoding {
                        PositionEncodingKind::UTF8 => change_end_line_cu_idx,
                        PositionEncodingKind::UTF16 => {
                            change_end_line.char_to_utf16_cu(change_end_line_char_idx)
                        }
                        PositionEncodingKind::UTF32 => change_end_line_char_idx,
                    },
                };

                self.rope
                    .remove(change_start_doc_char_idx..change_end_doc_char_idx);
                self.rope.insert(change_start_doc_char_idx, &change.text);

                // 6. Compute the byte index into the new end line where the
                // change ends. Required for tree-sitter.
                let change_new_end_line_idx = self
                    .rope
                    .byte_to_line(change_start_doc_byte_idx + change.text.len());
                let change_new_end_line_byte_idx = change_start_doc_byte_idx + change.text.len();

                // 7. Construct the tree-sitter edit. We stay mindful that
                // tree-sitter Point::column is a byte offset.
                let edit = InputEdit {
                    start_byte: change_start_doc_byte_idx,
                    old_end_byte: change_end_doc_byte_idx,
                    new_end_byte: change_start_doc_byte_idx + change.text.len(),
                    start_position: Point {
                        row: change_start_line_idx,
                        column: change_start_line_byte_idx,
                    },
                    old_end_position: Point {
                        row: change_end_line_idx,
                        column: change_end_line_byte_idx,
                    },
                    new_end_position: Point {
                        row: change_new_end_line_idx,
                        column: change_new_end_line_byte_idx,
                    },
                };

                //tree.edit(&edit);
                //self.tree = self.parser.parse(self.rope.to_string(), Some(tree));
                return Ok(Some(edit));
            }
            None => {
                self.rope = Rope::from_str(&change.text);
                //self.tree = self.parser.parse(&change.text, None);
            }
        }
        // update version
        Ok(None)
    }
}
