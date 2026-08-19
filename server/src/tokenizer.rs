// Copyright 2025-2026 Nokia
// Licensed under the BSD 3-Clause License.
// SPDX-License-Identifier: BSD-3-Clause

use std::{ops::BitOr, str::FromStr};

use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use tower_lsp_server::{
    jsonrpc,
    ls_types::{
        SemanticToken, SemanticTokenModifier, SemanticTokenType, SemanticTokens,
        SemanticTokensFullOptions, SemanticTokensLegend, SemanticTokensOptions,
        SemanticTokensParams, SemanticTokensResult, SemanticTokensServerCapabilities,
        WorkDoneProgressOptions,
    },
};

use crate::{document::Document, features::SemanticTokenFeature, grammar::Rule, text::SourceText};
use tree_sitter::{Node, Point, Range};

// NOTICE: We use "semantic-token-provider" to provide code highlighting, see below link
// https://code.visualstudio.com/api/language-extensions/semantic-highlight-guide#semantic-token-provider
#[repr(u32)]
#[derive(Debug, EnumIter, PartialEq, Clone, Copy)]
enum TokenType {
    Boolean,
    Call,
    Comment,
    Decorator, // normally, it will set "fontStyle" to "italic"
    Function,
    Keyword,
    Macro,
    Namespace,
    Number,
    Operator,
    Parameter,
    String,
    Variable,
}

impl From<TokenType> for SemanticTokenType {
    fn from(val: TokenType) -> Self {
        // (see also https://code.visualstudio.com/api/language-extensions/semantic-highlight-guide#standard-token-types-and-modifiers)
        match val {
            TokenType::Boolean => SemanticTokenType::VARIABLE,
            TokenType::Call => SemanticTokenType::INTERFACE,
            TokenType::Comment => SemanticTokenType::COMMENT,
            TokenType::Decorator => SemanticTokenType::DECORATOR,
            TokenType::Function => SemanticTokenType::FUNCTION,
            TokenType::Keyword => SemanticTokenType::KEYWORD,
            TokenType::Macro => SemanticTokenType::MACRO,
            TokenType::Namespace => SemanticTokenType::NAMESPACE,
            TokenType::Number => SemanticTokenType::NUMBER,
            TokenType::Operator => SemanticTokenType::OPERATOR,
            TokenType::Parameter => SemanticTokenType::PARAMETER,
            TokenType::String => SemanticTokenType::STRING,
            TokenType::Variable => SemanticTokenType::VARIABLE,
        }
    }
}

#[repr(u8)]
#[derive(Debug, EnumIter, PartialEq, Clone, Copy)]
enum Modifier {
    Deprecated, // normally deprecated text will be strike-through
    Readonly,   // normally mutable variables will have lighter color than read-only ones.
}

impl From<Modifier> for SemanticTokenModifier {
    fn from(val: Modifier) -> Self {
        match val {
            Modifier::Deprecated => SemanticTokenModifier::DEPRECATED,
            Modifier::Readonly => SemanticTokenModifier::READONLY,
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy, Default)]
struct Modifiers(u32);

impl BitOr for Modifiers {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

const READONLY: Modifiers = Modifiers(1 << (Modifier::Readonly as u8));
const DEPRECATED: Modifiers = Modifiers(1 << (Modifier::Deprecated as u8));

struct Token(TokenType, Range, Option<Modifiers>);

fn tokenize_from(node: &Node<'_>) -> Option<Token> {
    let range = node.range();
    let kind = node.kind();
    match Rule::from_str(kind) {
        Ok(rule) => match rule {
            Rule::Comment => Some(Token(TokenType::Comment, range, None)),
            Rule::FunctionBegin | Rule::FunctionClose => {
                Some(Token(TokenType::Keyword, range, None))
            }
            Rule::FunctionName | Rule::BuiltinName | Rule::MacroName => {
                Some(Token(TokenType::Call, range, None))
            }
            Rule::KeywordAs
            | Rule::AssignBegin
            | Rule::AssignClose
            | Rule::LocalBegin
            | Rule::LocalClose
            | Rule::FtlBegin
            | Rule::IfBegin
            | Rule::ElseBegin
            | Rule::ElseifBegin
            | Rule::IfClose
            | Rule::ImportBegin
            | Rule::CloseTag
            | Rule::ListBegin
            | Rule::ListClose
            | Rule::ItemsBegin
            | Rule::ItemsClose
            | Rule::SepBegin
            | Rule::SepClose
            | Rule::SwitchBegin
            | Rule::SwitchClose
            | Rule::BreakStmt
            | Rule::ContinueStmt
            | Rule::OnBegin
            | Rule::CaseBegin
            | Rule::DefaultBegin
            | Rule::ReturnBegin => Some(Token(TokenType::Keyword, range, None)),
            Rule::SelfClosingTag => Some(Token(TokenType::Keyword, range, Some(DEPRECATED))),
            Rule::MacroBegin
            | Rule::MacroCloseTag
            | Rule::MacroClose
            | Rule::MacroCallBegin
            | Rule::MacroCallEnd
            | Rule::InterpolationStart => Some(Token(TokenType::Macro, range, None)),
            Rule::ImportAlias | Rule::MacroNamespace => {
                Some(Token(TokenType::Namespace, range, None))
            }
            Rule::Number => Some(Token(TokenType::Number, range, None)),
            Rule::EqualOperator
            | Rule::AssignOperator
            | Rule::BinaryOperator
            | Rule::MissingValueOperator
            | Rule::NotOperator
            | Rule::MinusOperator
            | Rule::PlusOperator
            | Rule::GreaterThanOperator
            | Rule::GreaterThanEqualOperator => Some(Token(TokenType::Operator, range, None)),
            Rule::LegacyEqualOperator => Some(Token(TokenType::Operator, range, Some(DEPRECATED))),
            Rule::ParameterName => Some(Token(TokenType::Parameter, range, None)),
            Rule::Variable | Rule::Identifier | Rule::MacroPath => {
                Some(Token(TokenType::Variable, range, None))
            }
            Rule::StringLiteral | Rule::ImportPath | Rule::StringLvalue => {
                Some(Token(TokenType::String, range, None))
            }
            Rule::BooleanTrue | Rule::BooleanFalse => {
                Some(Token(TokenType::Boolean, range, Some(READONLY)))
            }
            _ => {
                // reaching here means that we don't have any corresponding standard token types for this tree-sitter node kind
                // if this tree-sitter node kind need to be hightlighted, there is 2 options:
                // A) map this node kind into a standard token
                // B) use custom token type mechanism (which brings complexity, NOT preferred)
                // See aslo https://code.visualstudio.com/api/language-extensions/semantic-highlight-guide#custom-token-types-and-modifiers
                None
            }
        },
        Err(_unknown) => None,
    }
}

pub fn semantic_token_capability() -> SemanticTokensServerCapabilities {
    // NOTICE: We use "semantic-token-provider" to provide syntax highlighting, see below link
    // https://code.visualstudio.com/api/language-extensions/semantic-highlight-guide#semantic-token-provider
    SemanticTokensServerCapabilities::SemanticTokensOptions(SemanticTokensOptions {
        work_done_progress_options: WorkDoneProgressOptions {
            work_done_progress: Some(false),
        },
        legend: SemanticTokensLegend {
            // NOTE: #[repr(u32)] makes TokenType ranged from 0, so that which value exactly matches the sequence index of below token_types array
            token_types: TokenType::iter().map(|t| t.into()).collect(),
            token_modifiers: Modifier::iter().map(|m| m.into()).collect(),
        },
        range: None,
        full: Some(SemanticTokensFullOptions::Bool(true)),
    })
}

fn encode_semantic_token(
    prev_start: &Point,
    token_type: TokenType,
    start: &Point,
    length: usize,
    modifiers: Option<Modifiers>,
) -> SemanticToken {
    // toxic encoding rule, see also:
    // (https://github.com/microsoft/vscode-extension-samples/blob/5ae1f7787122812dcc84e37427ca90af5ee09f14/semantic-tokens-sample/vscode.proposed.d.ts#L71)
    let delta_line = (start.row - prev_start.row) as u32;
    SemanticToken {
        delta_line,
        delta_start: match delta_line == 0 {
            // `deltaStart`: token start character, relative to the previous token (relative to 0 or the previous token's start if they are on the same line)
            true => start.column - prev_start.column,
            false => start.column,
        } as u32,
        length: length as u32,
        token_type: token_type as u32, // #[repr(u32)] makes token_type ranged from 0
        token_modifiers_bitset: match modifiers {
            Some(m) => m.0,
            None => 0,
        },
    }
}

/// Computes the semantic tokens for a document by walking the syntax tree once.
pub(crate) fn semantic_tokens(doc: &Document) -> Vec<SemanticToken> {
    let source = doc.source();
    let tree = doc.tree();
    fn collect(
        source: &SourceText,
        node: &Node,
        prev_start: &mut Point,
        tokens: &mut Vec<SemanticToken>,
    ) {
        if node.is_error() || node.is_missing() {
            // not sure if it is proper
            return;
        }
        if let Some(Token(token_type, range, modifiers)) = tokenize_from(node) {
            if range.end_point.row == range.start_point.row {
                // single-line token
                tokens.push(encode_semantic_token(
                    prev_start,
                    token_type,
                    &range.start_point,
                    range.end_byte - range.start_byte,
                    modifiers,
                ));
                *prev_start = range.start_point;
            } else {
                // multi-line token is not allowed, so split it into multiple inline tokens
                // token of 1st line
                let first_start = range.start_point;
                let first_line_len = source.line_len(first_start.row).unwrap();
                tokens.push(encode_semantic_token(
                    prev_start,
                    token_type,
                    &first_start,
                    first_line_len,
                    modifiers,
                ));
                *prev_start = first_start;
                // tokens from 2nd to last-1 line
                let mut next_row = first_start.row + 1;
                while next_row < range.end_point.row {
                    let next_start = Point {
                        row: next_row,
                        column: 0,
                    };
                    let next_line_len = source.line_len(next_row).unwrap();
                    tokens.push(encode_semantic_token(
                        prev_start,
                        token_type,
                        &next_start,
                        next_line_len,
                        modifiers,
                    ));
                    next_row += 1;
                    *prev_start = next_start;
                }
                // token of last line
                let last_start = Point {
                    row: range.end_point.row,
                    column: 0,
                };
                tokens.push(encode_semantic_token(
                    prev_start,
                    token_type,
                    &last_start,
                    range.end_point.column,
                    modifiers,
                ));
                *prev_start = last_start;
            }
        }
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i as u32) {
                collect(source, &child, prev_start, tokens);
            }
        }
    }

    let mut tokens = Vec::new();
    let mut prev_start = Point { row: 0, column: 0 };
    collect(source, &tree.root_node(), &mut prev_start, &mut tokens);
    tokens
}

impl SemanticTokenFeature for Document {
    async fn on_semantic_tokens_full(
        &self,
        _: SemanticTokensParams,
    ) -> jsonrpc::Result<Option<SemanticTokensResult>> {
        Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: Some(self.version.to_string()),
            data: semantic_tokens(self),
        })))
    }
}
