# Corpus

A collection of FreeMarker Template Language (FTL) text cases that exercise the
language features of the `lsp-for-freemarker` server. These files are meant to
be opened in an editor running the language server, so you can see the
diagnostics, go-to-definition jumps, and code actions live.

Each file is annotated with `<#-- ... -->` comments explaining what the snippet
demonstrates.

## Layout

- `diagnostic/` — snippets that trigger syntax and semantic diagnostics
  (import errors, undefined macros, undocumented constructs, …).
- `goto/` — snippets showing go-to-definition: jumping to an imported template
  or to a macro definition. `goto/lib/common.ftl` is a helper that the other
  files import.
- `action/` — snippets with quick-fixable warnings (self-closing tags, legacy
  equality operators, …).

## Usage

Open this folder (or the repo root) in an editor with the Freemarker extension
and the server running. The `.ftl` files under `corpus/` are picked up
automatically (the extension watches `**/*.ftl`).
