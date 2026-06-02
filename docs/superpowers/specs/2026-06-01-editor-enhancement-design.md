# Lisette Editor Enhancement Design

**Date:** 2026-06-01
**Author:** Contributor
**Status:** Approved

## 1. Problem Statement

The current Lisette VS Code extension (`editors/vscode/`) uses a basic TextMate grammar (`lisette.tmLanguage.json`) for syntax highlighting. While it covers keywords, strings, numbers, and some primitive types, it has significant gaps:

1. **No semantic awareness:** TextMate grammars are regex-based and cannot know whether an identifier is a type, function, variable, enum variant, or method. This means user-defined structs, enums, methods, and imported Go package symbols all receive generic coloring.
2. **Missing syntactic patterns:** Several Lisette constructs are either uncolored or incorrectly colored (e.g., `impl` blocks, `select`/`task`/`try`/`recover`, `match` arm bindings, field access, type annotations).
3. **Prelude / Go stdlib symbols:** Built-in types (`Option`, `Result`, `Slice`, etc.) and Go package types/functions are not distinguished from user-defined names.

## 2. Goals

- **Phase 1:** Improve static syntax highlighting by extending the TextMate grammar with more precise syntactic patterns.
- **Phase 2:** Implement LSP Semantic Tokens to provide accurate, context-aware highlighting for all identifiers (user-defined, prelude, Go stdlib).
- **Phase 3:** Polish the VS Code extension UX (theme integration docs, settings defaults).

## 3. Non-Goals

- Rewriting the LSP server architecture.
- Adding new LSP features unrelated to highlighting (e.g., code actions, inlay hints).
- Modifying the Tree-sitter grammar (`editors/tree-sitter-lisette/`).

## 4. Architecture

```
VS Code Extension
├── TextMate Grammar (Static)
│   └── lisette.tmLanguage.json  ← Phase 1
└── LSP Client (vscode-languageclient)
    └── Receives semantic tokens ← Phase 2

LSP Server (crates/lsp)
├── Existing handlers
└── NEW: Semantic Tokens Provider ← Phase 2
    └── AST traversal + definitions lookup
```

**Principle:** TextMate provides fast baseline highlighting; Semantic Tokens overlay precise, semantic information. This is the standard pattern used by Rust Analyzer, TypeScript, etc.

---

## 5. Phase 1: TextMate Grammar Enhancement

### 5.1 Current Gaps

| Construct | Current State | Target |
|-----------|-------------|--------|
| `impl` block | `impl` is `keyword.other` | `storage.type` + method definitions as `entity.name.function` |
| `select` / `task` / `try` / `recover` | Not highlighted | `keyword.control` |
| `match` arm bindings | Not highlighted | `variable` |
| `match` arm variants | Not highlighted | `entity.name.function` |
| Type annotation `: Type` | `Type` may be uncolored | `entity.name.type` |
| Field access `obj.field` | Incorrectly matched as `entity.name.namespace` + `entity.name.type` | `variable.other.property` for lowercase fields |
| `for` loop binding | Not highlighted | `variable` |
| `defer` block | `defer` is `keyword.other` | `keyword.control` |
| Pipeline `\|> func()` | `func` uncolored | `entity.name.function` |
| Generic arguments `Map<string, int>` | Partial | Primitive types inside generics should be `support.type.primitive` |
| `const` definitions | `const` keyword ok, identifier uncolored | `entity.name.constant` |
| `interface` method signatures | Method names uncolored | `entity.name.function` |
| `type alias` | `type` keyword ok, alias name uncolored | `entity.name.type` |

### 5.2 Implementation

Modify `editors/vscode/syntaxes/lisette.tmLanguage.json`:

1. **Add `impl` rule:** Match `impl Type { ... fn method(...)` and color `impl` as `storage.type`, method names as `entity.name.function`.
2. **Move control-flow keywords:** `select`, `task`, `try`, `recover`, `defer` to a `keyword.control` group.
3. **Add type annotation rule:** Match `: \s*([A-Z][a-zA-Z0-9_]*)` and color the capture as `entity.name.type`.
4. **Fix field access:** Distinguish `pkg.Func()` (uppercase) from `obj.field` (lowercase). Lowercase after dot should be `variable.other.property`.
5. **Add `for` binding rule:** Match `for\s+([a-z_][a-zA-Z0-9_]*)` and color capture as `variable`.
6. **Add `match` arm rules:** Inside `match` blocks, distinguish `Variant(...)` (constructor) from `x` (binding).
7. **Add `const` / `type` definition rules:** Color the defined name appropriately.
8. **Add `interface` method rule:** Inside `interface` blocks, color method signatures.

### 5.3 Risks & Mitigations

- **Regex complexity:** TextMate grammars can become slow if over-engineered. We will keep patterns simple and avoid nested backtracking.
- **False positives:** A regex that colors any uppercase name after `:` might color enum constructors used in expressions. We will scope the rule to avoid matching inside strings/comments.

---

## 6. Phase 2: LSP Semantic Tokens Provider

### 6.1 LSP Capabilities

Register in `initialize`:
```rust
semantic_tokens_provider: Some(SemanticTokensServerCapabilities::SemanticTokensOptions(
    SemanticTokensOptions {
        legend: SemanticTokensLegend {
            token_types: vec![
                SemanticTokenType::TYPE,
                SemanticTokenType::ENUM,
                SemanticTokenType::ENUM_MEMBER,
                SemanticTokenType::STRUCT,
                SemanticTokenType::INTERFACE,
                SemanticTokenType::FUNCTION,
                SemanticTokenType::METHOD,
                SemanticTokenType::VARIABLE,
                SemanticTokenType::PARAMETER,
                SemanticTokenType::PROPERTY,
                SemanticTokenType::NAMESPACE,
                SemanticTokenType::KEYWORD,
                SemanticTokenType::COMMENT,
                SemanticTokenType::STRING,
                SemanticTokenType::NUMBER,
                SemanticTokenType::OPERATOR,
            ],
            token_modifiers: vec![
                SemanticTokenModifier::DECLARATION,
                SemanticTokenModifier::READONLY,
                SemanticTokenModifier::STATIC,
                SemanticTokenModifier::ASYNC,
                SemanticTokenModifier::MODIFICATION,
                SemanticTokenModifier::DOCUMENTATION,
            ],
        },
        full: Some(SemanticTokensFullOptions::Bool(true)),
        ..Default::default()
    },
)),
```

### 6.2 Token Mapping Strategy

Walk the AST (`syntax::ast::Expression`) and emit tokens based on expression kind + context:

| AST Node | Token Type | Modifiers |
|----------|------------|-----------|
| `Expression::Struct { name, .. }` | `struct` | `declaration` |
| `Expression::Enum { name, .. }` | `enum` | `declaration` |
| `Expression::ValueEnum { name, .. }` | `enum` | `declaration` |
| `Expression::Interface { name, .. }` | `interface` | `declaration` |
| `Expression::TypeAlias { name, .. }` | `type` | `declaration` |
| `Expression::Function { name, .. }` | `function` | `declaration` |
| `Expression::Impl { type_name, methods }` | `impl` methods → `method`, `declaration` | `declaration` |
| `Expression::Identifier { qualified, binding_id, .. }` | Resolve via `definitions` or `facts.bindings` | `variable`, `function`, `type`, etc. |
| `Expression::DotAccess { expression, member, .. }` | Resolve receiver type → `property` (field) or `method` | |
| `Expression::StructCall { name, .. }` | `function` or `enumMember` (constructor) | |
| `Expression::Match { arms }` | Arm patterns: `enumMember` for variants, `variable` for bindings | |
| `Expression::Const { identifier, .. }` | `variable` or `constant` | `readonly`, `static` |
| `Expression::Let { binding: pattern }` | Pattern identifiers → `variable` | `declaration` |
| `Expression::For { binding, .. }` | Binding identifier → `variable` | `declaration` |
| `Expression::Function { params }` | Param pattern identifiers → `parameter` | `declaration` |
| `Expression::Lambda { params }` | Param pattern identifiers → `parameter` | `declaration` |
| `Expression::Import { name, alias }` | Import name/alias → `namespace` | |

**Resolution Logic:**
1. If `qualified` is present, look up in `snapshot.definitions()`.
2. If `binding_id` is present, look up in `snapshot.facts().bindings`.
3. If inside `DotAccess`, resolve receiver type via `type_name(&expression.get_type())` and look up field/method in `definitions`.
4. For `match` patterns, use pattern traversal to distinguish variant names from variable bindings.

### 6.3 Module Structure

New file: `crates/lsp/src/semantic_tokens.rs`
```rust
pub(crate) fn handle(
    file: &syntax::program::File,
    snapshot: &AnalysisSnapshot,
    line_index: &LineIndex,
) -> SemanticTokens {
    let mut builder = SemanticTokensBuilder::new();
    for item in &file.items {
        emit_expression_tokens(item, file, snapshot, &mut builder);
    }
    builder.build()
}
```

Wire into `crates/lsp/src/lib.rs`:
```rust
async fn semantic_tokens_full(
    &self,
    params: SemanticTokensParams,
) -> Result<Option<SemanticTokensResult>> {
    // ... fetch snapshot, file, line_index ...
    let tokens = semantic_tokens::handle(file, snapshot, line_index);
    Ok(Some(SemanticTokensResult::Tokens(tokens)))
}
```

### 6.4 VS Code Client Changes

Minimal. In `package.json`, ensure no `semanticHighlighting` is explicitly disabled. Optionally add:
```json
"configurationDefaults": {
  "[lisette]": {
    "editor.semanticHighlighting.enabled": true
  }
}
```

### 6.5 Risks & Mitigations

- **Performance:** Full AST traversal on every keystroke could be slow. Since `crates/lsp` already performs per-keystroke analysis (`did_change` → re-analyze), adding a token walk is acceptable. We can cache tokens per-file if needed.
- **Span accuracy:** Off-by-one errors in byte-offset → token mapping. We will use existing `LineIndex` utilities and write tests against known spans.
- **Go typedef spans:** Go stdlib definitions have dummy spans (`Span::dummy()`). We must skip these to avoid navigating to (0,0).

---

## 7. Phase 3: VS Code Extension Polish

1. **Theme documentation:** Add a section to `editors/vscode/README.md` showing how to customize semantic token colors for Lisette.
2. **Settings defaults:** In `package.json`, set `editor.semanticHighlighting.enabled` to `true` for `[lisette]`.
3. **Icon & marketplace:** Ensure the extension icon and marketplace metadata are up-to-date.

---

## 8. Testing Strategy

- **Phase 1:** Open various `.lis` files in VS Code and visually inspect. Add a `test-highlight.lis` fixture file to the repo.
- **Phase 2:** Write unit tests in `crates/lsp/tests/lsp.rs` that invoke `semantic_tokens_full` on a simple file and assert the returned token types/positions match expectations.
- **Integration:** Build the extension (`pnpm run compile`), run `lis lsp` in the background, open a Lisette project, and verify that user-defined types, methods, and prelude types are distinctly colored.

---

## 9. Success Criteria

- [ ] `impl` blocks, `select`/`task`/`try`/`recover` keywords are correctly colored.
- [ ] User-defined `struct`, `enum`, `interface`, `type alias` names receive distinct colors from variables.
- [ ] `match` arm variants and bindings are distinguishable.
- [ ] Prelude types (`Option`, `Result`, `Slice`, etc.) and Go stdlib symbols are colored as types/functions.
- [ ] No regressions in existing LSP features (hover, completion, goto definition, etc.).

---

## 10. Appendix: Existing LSP Capabilities

The LSP server already supports:
- `textDocumentSync`
- `documentFormattingProvider`
- `hoverProvider`
- `definitionProvider`
- `documentSymbolProvider`
- `referencesProvider`
- `renameProvider`
- `completionProvider`
- `signatureHelpProvider`

Phase 2 adds `semanticTokensProvider` to this list.
