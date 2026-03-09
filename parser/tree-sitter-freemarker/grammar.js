/**
 * Copyright 2025-2026 Nokia
 * Licensed under the BSD 3-Clause License.
 * SPDX-License-Identifier: BSD-3-Clause
 */

/**
 * @file The tree-sitter grammar DSL for freemarker template language
 * @author Lu Kai <kai.f.lu@nokia.com>
 * @license BSD-3-Clause
 * @see https://tree-sitter.github.io/tree-sitter/creating-parsers/2-the-grammar-dsl.html
 */

/// <reference types="tree-sitter-cli/dsl" />
// @ts-nocheck

const keyword_assign = 'assign';
const keyword_break = 'break';
const keyword_case = 'case';
const keyword_default = 'default';
const keyword_else = 'else';
const keyword_elseif = 'elseif';
const keyword_ftl = 'ftl';
const keyword_function = 'function';
const keyword_if = 'if';
const keyword_import = 'import';
const keyword_list = 'list';
const keyword_local = 'local';
const keyword_macro = 'macro';
const keyword_on = 'on';
const keyword_return = 'return';
const keyword_sep = 'sep';
const keyword_switch = 'switch';

module.exports = grammar({
  name: 'freemarker',

  // Define tokens that the external scanner will handle
  // The order here must match the enum in scanner.c
  externals: $ => [
    $.close_tag,                              // 0: Special '>' for closing directives
    $.greater_than_operator,                  // 1: Regular '>' operator
    $._open_paren,                            // 2: '('
    $._close_paren,                           // 3: ')'
    $.greater_than_equal_operator,            // 4: Regular '>=' operator
    $.deprecated_equal_operator,              // 5: '=', retard syntax alert
    $.equal_operator,                         // 6: '=='
    $.comment,                                // 7: comment
  ],

  precedences: $ => [
    [
      'member',
      'call',
      'unary_default',
      'unary_void',
      'binary_times',
      'binary_plus',
      'binary_compare',
      'binary_relation',
      'binary_equality',
      'bitwise_and',
      'bitwise_xor',
      'bitwise_or',
      'logical_and',
      'logical_or',
    ],
    ['assign', $._primary_expression],
    ['member', 'call', $._primary_expression],
    ['declaration', 'literal'],
    [$._primary_expression, 'object'],
    [$.import_stmt],
    [$.hash_variable, $.variable],
  ],

  conflicts: $ => [
    [$.subscript_expression, $._evaluate_expression],
    [$.subscript_expression, $.macro_call],
  ],

  rules: {
    source_file: $ => repeat($._definition),

    _definition: $ => choice(
      $._ftl_spec,
      $.text,
      $.macro_call,
    ),

    _ftl_spec: $ => choice(
      $.comment,
      $.directive,
    ),

    text: $ => seq(
      choice(
        $.interpolation,
        $._rawstring
      )
    ),

    _rawstring: $ => choice(
      // match string that does not contain '<' or '$'
      /[^<$]+/,
      // match '<' when next char is not '#'
      seq('<', /[^#]/),
      // match '</' when next char is not '#'
      seq('</', token.immediate(/[^#]/)),
      // match '$' when next char is not '{'
      seq('$', token.immediate(/[^{]/))
    ),

    interpolation: $ => seq(
      prec.right(seq(
        alias('$', $.interpolation_prepend),
        '{',
        $._evaluate_expression,
        '}'))
    ),

    directive: $ => choice(
      $.assign_stmt,
      $.ftl_stmt,
      $.function_stmt,
      $.if_stmt,
      $.import_stmt,
      $.list_stmt,
      $.local_stmt,
      $.macro_stmt,
      $.return_stmt,
      $.switch_stmt,
      $.break_stmt,
    ),

    // TODO: support '[#]' syntax
    break_stmt: _ => `<#${keyword_break}>`,

    /********** STATEMENT_BEGIN: "assign" **************/
    assign_stmt: $ => seq(
      BeginAlias(keyword_assign, $.assign_begin),
      choice(
        $.assign_inline,
        seq($.assign_clause, CloseAlias(keyword_assign, $.assign_close))
      )
    ),

    assign_inline: $ => seq(
      repeat($.assign_expression),
      choice(
        $.close_tag,
        alias('/>', $.undocumented_close_tag) // retard syntax alert
      )
    ),

    assign_clause: $ => seq(
      field('into', $.variable), // todo: support "<#assign x in y>"
      $.close_tag,
      field('from', repeat($._definition)),
    ),
    /********** STATEMENT_END: "assign" **************/

    /********** STATEMENT_BEGIN: "ftl" **************/
    ftl_stmt: $ => seq(
      BeginAlias(keyword_ftl, $.ftl_begin),
      repeat($.ftl_parameter),
      $.close_tag,
    ),

    ftl_parameter: $ => seq(
      field('name', $.variable),
      alias('=', $.assign_operator),
      field('value', choice(
        $.boolean_true,
        $.boolean_false,
        $.number,
        $.string_literal,
        // todo: hash object, see https://freemarker.apache.org/docs/ref_directive_ftl.html
      ))
    ),
    /********** STATEMENT_END: "ftl" **************/

    /********** STATEMENT_BEGIN: "if" ***********/
    if_stmt: $ => seq(
      BeginAlias(keyword_if, $.if_begin),
      $.if_clause,
      repeat(seq(
        BeginAlias(keyword_elseif, $.elseif_begin),
        $.elseif_clause
      )),
      optional(seq(
        BeginAlias(keyword_else, $.else_begin, false),
        alias(optional(repeat($._definition)), $.else_clause)
      )),
      CloseAlias(keyword_if, $.if_close)
    ),

    if_clause: $ => seq(
      field('condition', $._evaluate_expression),
      $.close_tag,
      optional(repeat($._definition)),
    ),

    elseif_clause: $ => seq(
      field('condition', $._evaluate_expression),
      $.close_tag,
      optional(repeat($._definition)),
    ),

    /********** STATEMENT_END: "if" ***********/

    /********** STATEMENT_BEGIN: "import" ***********/
    import_stmt: $ => seq(
      BeginAlias(keyword_import, $.import_begin),
      FieldAlias(alias($.string_literal, $.import_path)),
      $.keyword_as,
      FieldAlias(alias($.identifier, $.import_alias)),
      $.close_tag,
    ),
    /********** STATEMENT_END: "import" **************/

    /********** STATEMENT_BEGIN: "function" ***********/
    function_stmt: $ => seq(
      BeginAlias(keyword_function, $.function_begin),
      $.function_clause,
      CloseAlias(keyword_function, $.function_close)
    ),

    function_clause: $ => seq(
      field('name', alias($.identifier, $.function_name)),
      field('parameter', optional(repeat(choice(alias($.identifier, $.parameter_name), $.assign_expression)))),
      $.close_tag,
      field('body', optional(repeat($._definition))),
    ),

    // TODO: currently <#return> can appear outside <#function>, which is improper
    return_stmt: $ => seq(
      BeginAlias(keyword_return, $.return_begin),
      field('value', $._evaluate_expression),
      $.close_tag,
    ),
    /********** STATEMENT_END: "function" **************/

    /********** STATEMENT_BEGIN: "list" ***********/
    /*
    TODO:
      + <#items> directive
    DEPRECATED:
      + <#continue>, see https://freemarker.apache.org/docs/ref_directive_list.html#ref_list_continue
    */
    list_stmt: $ => seq(
      BeginAlias(keyword_list, $.list_begin),
      $.list_clause,
      optional(seq(
        BeginAlias(keyword_else, $.else_begin, false),
        alias(optional(repeat($._definition)), $.else_clause)
      )),
      CloseAlias(keyword_list, $.list_close),
    ),

    list_clause: $ => seq(
      field('collection', choice($.variable, $.hash_variable, $.array)),
      $.keyword_as,
      field('iterator', $._list_iterator),
      $.close_tag,
      field('body', optional(repeat($._definition))),
      field('sep', optional($.sep_directive)),
    ),

    _list_iterator: $ => choice(
      $.identifier,
      $._keyval_pair,
    ),

    _keyval_pair: $ => seq(
      $.identifier,
      ',',
      $.identifier
    ),

    sep_directive: $ => seq(
      BeginAlias(keyword_sep, $.sep_begin, false),
      field('seperator', $.text),
      optional(
        seq(
          CloseAlias(keyword_sep, $.sep_close), // optional </#sep> might conflict with <#else>?
          optional(repeat($._definition)))
      )),
    /********** STATEMENT_END: "list" **************/

    /********** STATEMENT_BEGIN: "local" **************/
    local_stmt: $ => seq(
      BeginAlias(keyword_local, $.local_begin),
      choice(
        $.local_inline,
        seq(
          $.local_clause,
          CloseAlias(keyword_local, $.local_close)
        )
      )
    ),

    local_inline: $ => seq(
      repeat($.assign_expression),
      $.close_tag,
    ),

    local_clause: $ => seq(
      field('into', $.variable),
      $.close_tag,
      field('from', repeat($._definition)),
    ),
    /********** STATEMENT_END: "local" **************/

    /********** STATEMENT_BEGIN: "macro" ***********/
    macro_stmt: $ => seq(
      BeginAlias(keyword_macro, $.macro_begin),
      FieldAlias(alias($.identifier, $.macro_name)),
      $.macro_clause,
      CloseAlias(keyword_macro, $.macro_close)
    ),

    macro_clause: $ => seq(
      field('parameter', optional(repeat(choice($.identifier, $.assign_expression)))),
      // TODO: support "..." syntax
      alias($.close_tag, $.macro_close_tag),
      field('body', repeat($._definition)),
    ),

    macro_call: $ => seq(
      alias('<@', $.macro_call_begin),
      alias($.identifier, $.macro_namespace),
      alias(repeat(seq('.', $.identifier)), $.macro_specs),
      field('parameter', repeat(choice($._primary_expression, $.assign_expression))),
      alias('/>', $.macro_call_end)
    ),
    /********** STATEMENT_END: "macro" **************/

    /********** STATEMENT_BEGIN: "switch" ***********/
    switch_stmt: $ => seq(
      BeginAlias(keyword_switch, $.switch_begin),
      $.switch_clause,
      CloseAlias(keyword_switch, $.switch_close)
    ),

    switch_clause: $ => seq(
      field('value', choice($.variable, $.hash_variable, $.string_literal, $.number)),
      $.close_tag,
      repeat(choice(
        seq(BeginAlias(keyword_on, $.on_begin), $.on_clause),
        seq(BeginAlias(keyword_case, $.case_begin), $.case_clause)
      )),
      optional(seq(
        BeginAlias(keyword_default, $.default_begin, false),
        alias(
          repeat($._definition),
          $.default_clause)
      )),
    ),

    on_clause: $ => seq(
      field('condition', commaSep(choice($.string_literal, $.number))),
      $.close_tag,
      optional(repeat($._definition)),
    ),

    case_clause: $ => seq(
      field('condition', choice($.string_literal, $.number)),
      $.close_tag,
      optional(repeat($._definition)),
    ),
    /********** STATEMENT_END: "switch" **************/

    /********** basic tokens rules **************/
    number: _ => {
      const hexLiteral = seq(
        choice('0x', '0X'),
        /[\da-fA-F](_?[\da-fA-F])*/,
      );

      const decimalDigits = /\d(_?\d)*/;
      const signedInteger = seq(optional(choice('-', '+')), decimalDigits);
      const exponentPart = seq(choice('e', 'E'), signedInteger);

      const binaryLiteral = seq(choice('0b', '0B'), /[0-1](_?[0-1])*/);

      const octalLiteral = seq(choice('0o', '0O'), /[0-7](_?[0-7])*/);

      const bigintLiteral = seq(choice(hexLiteral, binaryLiteral, octalLiteral, decimalDigits), 'n');

      const decimalIntegerLiteral = choice(
        '0',
        seq(optional('0'), /[1-9]/, optional(seq(optional('_'), decimalDigits))),
      );

      const decimalLiteral = choice(
        seq(decimalIntegerLiteral, '.', optional(decimalDigits), optional(exponentPart)),
        seq('.', decimalDigits, optional(exponentPart)),
        seq(decimalIntegerLiteral, exponentPart),
        decimalDigits,
      );

      return token(choice(
        hexLiteral,
        decimalLiteral,
        binaryLiteral,
        octalLiteral,
        bigintLiteral,
      ));
    },

    boolean_true: _ => 'true',
    boolean_false: _ => 'false',
    keyword_as: _ => 'as',

    // unescaped string that can be single quoted
    _unescaped_string1: _ => token.immediate(prec(1, /[^'\\\r\n]+/)),
    // unescaped string that can be single quoted
    _unescaped_string2: _ => token.immediate(prec(1, /[^"\\\r\n]+/)),
    // escaped string
    _escape_sequence: _ => token.immediate(seq(
      '\\',
      choice(
        /[^xu0-7]/,
        /[0-7]{1,3}/,
        /x[0-9a-fA-F]{2}/,
        /u[0-9a-fA-F]{4}/,
        /u\{[0-9a-fA-F]+\}/,
        /[\r?][\n\u2028\u2029]/,
      ),
    )),

    string_literal: $ => choice(
      seq(
        '\'',
        repeat(choice(
          $._unescaped_string1,
          $._escape_sequence,
        ))
        ,
        '\'',
      ),
      seq(
        '"',
        repeat(choice(
          $._unescaped_string2,
          $._escape_sequence,
        )),
        '"',
      ),
    ),

    identifier: _ => {
      const alpha = /[^\x00-\x1F\s\p{Zs}0-9:;`"'@#.,|^&<=>+\-*/\\%?!~()\[\]{}\uFEFF\u2060\u200B\u2028\u2029]|\\u[0-9a-fA-F]{4}|\\u\{[0-9a-fA-F]+\}/;

      const alphanumeric = /[^\x00-\x1F\s\p{Zs}:;`"'@#.,|^&<=>+*/%?!~()\[\]{}\uFEFF\u2060\u200B\u2028\u2029]|\\u[0-9a-fA-F]{4}|\\u\{[0-9a-fA-F]+\}/;
      return token(seq(alpha, repeat(alphanumeric)));
    },

    variable: $ => field('name', $.identifier),

    _primary_expression: $ => choice(
      $.parenthesized_expression,
      $.subscript_expression,
      $.member_expression,
      $.variable,
      $.number,
      $.string_literal,
      $.boolean_true,
      $.boolean_false,
      $.array,
      $.object,
      $.call_expression,
    ),

    hash_variable: $ => prec.right('member', choice(
      $.member_expression,
      $.subscript_expression,
    )),

    member_expression: $ => prec.left('member', seq(
      field('object', $._primary_expression),
      choice(
        seq('.', $.identifier),
        $.builtin_call
      ))
    ),

    subscript_expression: $ => prec.right('member', seq(
      field('object', $._primary_expression),
      '[', $._evaluate_expression, ']',
    )),

    _lvalue_expression: $ => choice(
      $.variable,
      alias($.string_literal, $.ambiguous_string_literal) // retard syntax alert
    ),

    assign_expression: $ => prec.right(seq(
      field('left', $._lvalue_expression),
      alias('=', $.assign_operator),
      field('right', $._evaluate_expression),
    )),

    unary_expression: $ => prec.left('unary_void', seq(
      // TODO: handle '-' as prefix, like negative numbers
      alias('!', $.negation_operator),
      field('argument', $._evaluate_expression),
    )),

    default_expression: $ => prec.left('unary_default', seq(
      field('argument', choice($.variable, $.hash_variable, $.parenthesized_expression, $.subscript_expression)),
      field('operator', alias('??', $.default_operator)),
    )),

    binary_expression: $ => choice(
      ...[
        ['&&', 'logical_and'],
        ['||', 'logical_or'],
        ['&', 'bitwise_and'],
        ['^', 'bitwise_xor'],
        ['|', 'bitwise_or'],
        ['+', 'binary_plus'],
        ['-', 'binary_plus'],
        ['*', 'binary_times'],
        ['/', 'binary_times'],
        ['%', 'binary_times'],
        [$.deprecated_equal_operator, 'binary_equality'],
        [$.equal_operator, 'binary_equality'],
        ['!=', 'binary_equality'],
        ['!==', 'binary_equality'],
        ['gt', 'binary_relation'],
        ['lt', 'binary_relation'],
        ['gte', 'binary_relation'],
        ['lte', 'binary_relation'],
        [$.greater_than_operator, 'binary_relation'],
        ['<', 'binary_relation'],
        [$.greater_than_equal_operator, 'binary_relation'],
        ['<=', 'binary_relation'],
        ['in', 'binary_relation'],
      ].map(([operator, precedence]) =>
        prec.left(precedence, seq(
          field('left', operator === 'in' ? choice($.hash_variable, $.variable) : $._evaluate_expression),
          field('operator', typeof operator === 'string' ? alias(operator, $.binary_operator) : operator),
          field('right', $._evaluate_expression),
        )),
      ),
    ),

    builtin_for_string: $ => prec('member', choice(
      ...[
        ['boolean', false],
        ['blank_to_null', false],
        ['cap_first', false],
        ['c', false],
        ['cn', false],
        ['c_lower_case', false],
        ['c_upper_case', false],
        ['capitalize', false],
        ['chop_linebreak', false],
        ['contains', true],
        ['date', false],
        ['time', false],
        ['datetime', false],
        ['empty_to_null', false],
        ['ends_with', true],
        ['ensure_ends_with', true],
        ['ensure_starts_with', true],
        ['esc', false],
        ['groups', true],
        //'html (deprecated)', true],
        ['index_of', true],
        ['j_string', false],
        ['js_string', false],
        ['json_string', false],
        ['keep_after', true],
        ['keep_after_last', true],
        ['keep_before', true],
        ['keep_before_last', true],
        ['last_index_of', true],
        ['left_pad', true],
        ['length', false],
        ['lower_case', false],
        ['matches', true],
        ['no_esc', false],
        ['number', false],
        ['replace', true],
        ['right_pad', true],
        ['remove_beginning', true],
        ['remove_ending', true],
        //'rtf (deprecated)', true],
        ['split', true],
        ['starts_with', true],
        ['string', false],
        //'substring (deprecated]) true],,
        ['trim', false],
        ['trim_to_null', true],
        ['truncate', true],
        ['uncap_first', false],
        ['upper_case', false],
        ['url', false],
        ['url_path', true],
        ['word_list', false],
        //'xhtml (deprecated)', true],
        //'xml (deprecated)', true],
        //'Common flags', true],
      ].map(([name, has_params]) =>
        has_params ?
          seq('?', alias(name, $.builtin_name), $._call_arguments) :
          seq('?', alias(name, $.builtin_name)))
    )),

    builtin_for_number: $ => prec('member', choice(
      ...[
        ['abs', false],
        //'c (for numbers)', true],
        //'cn (for numbers)', true],
        ['is_infinite', false],
        ['is_nan', false],
        ['lower_abc', false],
        ['round', false],
        ['floor', false],
        ['ceiling', false],
        //'string (when used with a numerical value)', true],
        ['upper_abc', false],
      ].map(([name, has_params]) =>
        has_params ?
          seq('?', alias(name, $.builtin_name), $._call_arguments) :
          seq('?', alias(name, $.builtin_name)))
    )),

    builtin_for_boolean: $ => prec('member', choice(
      //c (for boolean value)
      //cn (for boolean value)
      //string (when used with a boolean value)
      ...[
        ['then', true],
      ].map(([name, has_params]) =>
        has_params ?
          seq('?', alias(name, $.builtin_name), $._call_arguments) :
          seq('?', alias(name, $.builtin_name)))
    )),

    builtin_for_sequence: $ => prec('member', choice(
      ...[
        ['chunk', true],
        ['drop_while', true],
        ['filter', true],
        ['first', false],
        ['join', true],
        ['last', false],
        ['map', true],
        ['min', false],
        ['max', false],
        ['reverse', false],
        ['seq_contains', true],
        ['seq_index_of', true],
        ['seq_last_index_of', true],
        ['size', false],
        ['sort', false],
        ['sort_by', true],
        ['take_while', true],
      ].map(([name, has_params]) =>
        has_params ?
          seq('?', alias(name, $.builtin_name), $._call_arguments) :
          seq('?', alias(name, $.builtin_name)))
    )),

    builtin_for_expert: $ => prec('member', choice(
      ...[
        ['absolute_template_name', true],
        ['api', false],
        ['has_api', false],
        ['byte', false],
        ['double', false],
        ['float', false],
        ['int', false],
        ['long', false],
        ['short', false],
        ['eval', false],
        ['eval_json', false],
        ['has_content', false],
        ['interpret', false],
        ['is_string', false],
        ['is_number', false],
        ['is_boolean', false],
        ['is_date', false],
        ['is_date_like', false],
        ['is_date_only', false],
        ['is_time', false],
        ['is_datetime', false],
        ['is_unknown_date_like', false],
        ['is_method', false],
        ['is_transform', false],
        ['is_macro', false],
        ['is_hash', false],
        ['is_hash_ex', false],
        ['is_sequence', false],
        ['is_collection', false],
        ['is_collection_ex', false],
        ['is_enumerable', false],
        ['is_indexable', false],
        ['is_directive', false],
        ['is_node', false],
        ['is_markup_output', false],
        ['markup_string', false],
        ['namespace', false],
        ['new', true],
        ['number_to_date', false],
        ['number_to_time', false],
        ['number_to_datetime', false],
        ['sequence', false],
        ['with_args', true],
        ['with_args_last', true],
      ].map(([name, has_params]) =>
        has_params ?
          seq('?', alias(name, $.builtin_name), $._call_arguments) :
          seq('?', alias(name, $.builtin_name)))
    )),

    builtin_for_hash: $ => prec('member', choice(
      ...[
        ['keys', false],
        ['values', false],
      ].map(([name, has_params]) =>
        has_params ?
          seq('?', alias(name, $.builtin_name), $._call_arguments) :
          seq('?', alias(name, $.builtin_name)))
    )),

    builtin_call: $ => prec('member', choice(
      $.builtin_for_string,
      $.builtin_for_number,
      $.builtin_for_boolean,
      $.builtin_for_sequence,
      $.builtin_for_expert,
      $.builtin_for_hash,
    )),

    call_expression: $ =>
      prec('call', seq(
        alias($.variable, $.function_name),
        field('argument', $._call_arguments)
      )),

    _call_arguments: $ => seq(
      '(',
      commaSep(optional($._evaluate_expression)),
      ')',
    ),

    _evaluate_expression: $ => choice(
      $.binary_expression,
      $._primary_expression,
      $.unary_expression,
      $.default_expression,
    ),

    parenthesized_expression: $ => prec.right(seq(
      $._open_paren, // From external scanner
      $._evaluate_expression,
      $._close_paren // From external scanner
    )),

    array: $ => seq(
      '[',
      commaSep(optional($._evaluate_expression)),
      ']',
    ),

    _property_name: $ => choice(
      alias(
        choice($.identifier, $.hash_variable),
        $.property_identifier,
      ),
      $.string_literal,
      $.number
    ),

    pair: $ => seq(
      field('key', $._property_name),
      ':',
      field('value', $._evaluate_expression),
    ),

    object: $ => prec('object', seq(
      '{',
      commaSep(optional($.pair)),
      '}',
    )),

    /********** end basic tokens rules **************/
  }
});

/**
 * Creates a rule to match one or more of the rules separated by a comma
 *
 * @param {Rule} rule
 *
 * @returns {SeqRule}
 */
function commaSep1(rule) {
  return seq(rule, repeat(seq(',', rule)));
}

/**
 * Creates a rule to optionally match one or more of the rules separated by a comma
 *
 * @param {Rule} rule
 *
 * @returns {ChoiceRule}
 */
function commaSep(rule) {
  return optional(commaSep1(rule));
}

/**
 * Creates an alias rule for the begin part of the freemarker keyword
 * e.g. "<#foo" || "<#foo>"
 *
 * @param {String} keyword
 * @param {Rule} rule_alias
 * @param {boolean} any_attributes whether begin part has any attributes 
 *
 * @returns {Rule} rule_alias
 * 
 * @note This function relies on the tree-sitter ABI version!
 */
function BeginAlias(keyword, rule_alias, any_attributes = true) {
  var text = `<#${keyword}`;
  if (!any_attributes) {
    text += '>';
  }
  return alias(text, rule_alias)
}

/**
 * Creates an alias rule for the close part of the freemarker keyword, e.g. "</#foo>"
 *
 * @param {String} keyword
 * @param {Rule} rule_alias
 *
 * @returns {Rule}
 * 
 * @note This function relies on the tree-sitter ABI version!
 */
function CloseAlias(keyword, rule_alias) {
  return alias(`</#${keyword}>`, rule_alias)
}

/**
 * Creates an alias rule for the close part of the freemarker keyword, e.g. "</#foo>"
 *
 * @param {Rule} alias_rule alias rule
 *
 * @returns {Rule}
 * 
 * @note This function relies on the tree-sitter ABI version!
 */
function FieldAlias(alias_rule) {
  return field(alias_rule.value, alias_rule)
}
