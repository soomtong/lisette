# AGENTS.md

## Project overview

Lisette is a little language inspired by Rust that compiles to Go. It has a Rust-based compiler toolchain organized as a Cargo workspace. The binary is `lis` (built from `crates/cli`).

**Key property:** Rust code compiles the compiler itself. Lisette source code (`.lis`) is only found in:
- `tests/spec/` — test cases (inline strings, not files)
- `crates/stdlib/typedefs/` — type definition stubs for Go stdlib packages
- `crates/stdlib/prelude.d.lis` — prelude definitions
- `prelude/` — Go runtime support library (standard Go, not Lisette)
- `bindgen/` — Go tool that generates Lisette typedef stubs from Go source

## Prerequisites

- Rust 1.94 (see `rust-toolchain.toml`): `rustup default 1.94`
- Go 1.25.10 (see `go-version`) for bindgen, prelude, and E2E tests
- `just` command runner: `brew install just` / `cargo install just`

## Build, test, and lint commands

All commands are driven by `just`. Aliases are defined in the `justfile`:

```bash
just b          # cargo build --release
just bd         # cargo build (debug)
just t          # cargo test -p tests --test suite && cargo test -p lisette-lsp --test lsp
just tu         # cargo test --workspace --lib --bins (fast unit tests only)
just c          # format-check + test + test-unit + lint (full CI check)
just l          # cargo clippy --all-targets -- -D warnings
just lf         # cargo clippy --fix --allow-dirty --allow-staged
just f          # cargo fmt
just ta         # cargo insta accept --all   (accept new snapshots)
just tr         # cargo insta review    (interactive snapshot review)
just te         # e2e smoke test: cargo build -p lisette && cargo test -p tests --test e2e_smoke
```

Incremental test targets:
```bash
just test-infer        # only inference tests: cargo test -p tests --test suite infer_tests
just test-watch        # watch mode: cargo watch -x "test -p tests --test suite"
just test-e2e-suite    # full e2e suite: cargo test -p tests --test e2e_suite -- --nocapture
just test-e2e-smoke    # smoke + e2e_learn tests
just test-refresh-snapshots  # cargo insta test --force-update-snapshots
```

Coverage:
```bash
just cov  # cargo llvm-cov -p tests -p lisette-lsp --test suite --test lsp --html --open
```

Note: `cargo test` at workspace root does NOT run the test suite. Use `just t`.

## Architecture: compiler pipeline

The compiler has 5 phases, each in a separate crate:

```
Source (.lis)
  → syntax (lex → parse → desugar)
  → semantics (module graph → registration → inference → passes → lints)
  → emit (Go code generation)
  → CLI writes to target/*.go
```

### Crate dependency graph

```
cli ──→ semantics ──→ syntax
 │        │  │           │
 │        │  └─ diagnostics
 │        │       └─ syntax
 │        └─ stdlib, deps
 ├── emit ──→ syntax, diagnostics
 ├── format ──→ syntax
 ├── lsp ──→ semantics, syntax, emit, diagnostics, format
 └── deps
```

### Phase details

**`syntax` crate** (`crates/syntax/`):
- `lex/` — hand-written lexer (not generated). Produces tokens and trivia (comments).
- `parse/` — recursive descent + Pratt parser. Produces `Expression` AST nodes.
- `desugar/` — transforms syntactic sugar (pipeline `|>`, etc.) into core AST.
- `ast.rs` — the core `Expression` enum (~90 variants).
- `types.rs` — the `Type` enum (used by both syntax and semantics; serializable via `serde` feature).
- `program/` — module-level constructs: `Definition`, `ModuleInfo`, file/module management.

**`semantics` crate** (`crates/semantics/`):
- `analyze.rs` — top-level orchestrator: builds module graph, resolves imports, caches, runs checker.
- `module_graph/` — Kahn's algorithm for topological sort with cycle detection.
- `checker/` — type-checking core:
  - `registration/` — registers definitions (types, functions, methods, builtins) in the type environment.
  - `infer/` — Hindley-Milner unification-based type inference (no bidirectional typing).
  - `scopes.rs` — lexical scope tracking.
  - `type_env.rs` — type variable management, unification state.
- `passes/` — post-inference checks run in parallel via `rayon`:
  - `checks/` — exhaustive pattern checking (Maranget algorithm), irrefutability, visibility, const naming, etc.
  - `lints/` — dead code, unused variables, casing, self-comparisons, redundant patterns, etc.
  - Lint data flows: `fact_producers/` extract facts → `lints/from_facts.rs` consumes facts → `ast_walk/` does visitor-based checks.
- `cache/` — incremental compilation cache (module hashing, prelude cache, Go stdlib cache).
- `prelude.rs` — registers built-in types (`Option`, `Result`, `Slice`, `Map`, `Channel`, etc.) and their methods.

**`emit` crate** (`crates/emit/`):
- Produces Go source files from semantic output.
- Handles ABI transitions (Lisette types ↔ Go types), enum layouts, pattern matching compilation.
- Subfolders mirror language constructs: `definitions/`, `expressions/`, `control_flow/`, `patterns/`, `calls/`.
- `patterns/decision_tree.rs` + `tree_emitter.rs` — compiles pattern matching into switch/if-else chains.

**`diagnostics` crate** (`crates/diagnostics/`):
- `LisetteDiagnostic` wraps `miette::Diagnostic` with source spans.
- `LocalSink` — thread-local diagnostic collector (no `Arc<Mutex<>>`; each thread pushes to its own sink).
- Error rendering uses `miette` for fancy terminal output.

**`format` crate** (`crates/format/`):
- Pretty-printer: parse → format → string. Assumes valid input (no error recovery).
- Uses a custom line-based layout engine (`lindig.rs`). `INDENT_WIDTH = 2`, `MAX_LINE_WIDTH = 80`.

**`cli` crate** (`crates/cli/`):
- Binary name: `lis`. Entry point: `src/main.rs` → `Command::parse` → handler dispatch.
- Handlers in `src/handlers/`: `build.rs`, `check.rs`, `run.rs`, `format.rs`, `new.rs`, `add.rs`, `sync.rs`, `doc.rs`, `lsp.rs`, `learn.rs`, `bindgen.rs`, `completions.rs`.
- `pipeline.rs` — orchestrates `syntax::build_ast` → `semantics::analyze` → `emit::Emitter::emit`.
- `go_cli.rs` — invokes `go build`, `go run`, `go mod tidy`, etc.
- `agents_md.rs` — embeds `agents_template.md` (the `lis learn` content) at compile time.
- `lock.rs` — `target/` file-locking via `fs2`.

**`lsp` crate** (`crates/lsp/`):
- Standard LSP server using `tower-lsp`.
- Analysis performed on per-keystroke snapshots, then responses served from analysis results.

**`stdlib` crate** (`crates/stdlib/`):
- Embeds type definition files (`.d.lis`) for Go standard library packages.
- `build.rs` compiles the typedefs into the binary at build time.
- `prelude.d.lis` — defines the Lisette prelude types (`Option`, `Result`, `Slice`, `Map`, `Ref`, `Channel`, `Partial`, etc.).

**`deps` crate** (`crates/deps/`):
- `TypedefLocator` — resolves import paths to typedef file contents.
- `project_manifest.rs` — parses `lisette.toml`.

## Key conventions and gotchas

### Testing patterns

**Snapshot testing** is the primary test strategy. The project uses `insta` with custom macros defined in `tests/_harness/macros.rs`:

- `assert_parse_snapshot!` — lexes + parses source, snapshots the AST
- `assert_format_snapshot!` — formats source, snapshots the formatted output
- `assert_desugar_snapshot!` — lexes + parses + desugars, snapshots AST
- `assert_lex_snapshot!` — snapshots tokens + trivia
- `assert_infer_error_snapshot!` — asserts inference produces a specific error
- `assert_lex_error_snapshot!`, `assert_parse_error_snapshot!`, `assert_desugar_error_snapshot!` — error snapshot variants

**Test file conventions:**
- Each test is a `#[test] fn` in `tests/spec/<phase>/mod.rs` (or `.rs` for single-file modules) with inline source strings
- Snapshot files live in `tests/spec/<phase>/snapshots/<test_name>.snap`
- System tests (UI errors) live in `tests/ui/` and use `miette` rendering in snapshots
- To accept new/updated snapshots: `just ta` (i.e. `cargo insta accept --all`)
- To review interactively: `just tr` (i.e. `cargo insta review`)

**Adding a new test:**
1. Add a `#[test] fn` with inline source in the appropriate `spec/<phase>/mod.rs` or its submodule
2. Run `just t` — it will create a `.snap.new` file
3. Run `just ta` to accept, or `just tr` to review

**Inference test harness** (`tests/_harness/infer.rs`):
- `infer(source)` — parses and runs full inference + lint pipeline
- `infer_module(module_name, MockFileSystem)` — multi-file module inference
- `infer_with_go_typedefs(source, typedefs)` — with Go interop typedefs

**Multi-module tests** use `MockFileSystem` to simulate a project with multiple files.

**E2E tests** compile Lisette → Go, build the Go output, and check Go artifacts for expected strings. Found in `tests/e2e_smoke.rs`, `tests/e2e_learn.rs`, and `tests/e2e_suite.rs`.

### Snapshot macros and insta settings

All snapshot macros use consistent insta settings:
```rust
insta::with_settings!({
    prepend_module_to_snapshot => false,
    omit_expression => true,
}, {
    insta::assert_debug_snapshot!(...);
});
```

**Critical:** Prefer `assert_debug_snapshot!` for AST snapshots — it uses the `Debug` trait which produces structured output. Use `assert_snapshot!` for text/error output.

### The `EcoString` type

Used pervasively for string interning. Imported from `ecow` crate (copy-on-write string). Re-exported as `syntax::EcoString`. Always use `.into()` not `EcoString::from()` to convert `&str` → `EcoString`.

### The `FxHashMap` / `FxHashSet` convention

The project uses `rustc_hash::FxHashMap` and `rustc_hash::FxHashSet` everywhere (not `std::collections::HashMap`). Import pattern:
```rust
use rustc_hash::FxHashMap as HashMap;
use rustc_hash::FxHashSet as HashSet;
```

### Thread-local diagnostic collection

`LocalSink` is `!Sync` + `!Send` — it's thread-local. Each worker thread creates its own sink. Do NOT try to share sinks across threads. Instead, collect from each thread after parallel work completes.

### Parallelism with rayon

Post-inference semantic passes run in parallel via `rayon`. The `passes/` module orchestrates this. Inference itself is NOT parallel — it's sequential per module, respecting topological order.

### Size assertions

`crates/syntax/src/lib.rs` contains compile-time size assertions for key types:
```rust
const _: () = assert!(size_of::<ast::Expression>() == 344);
const _: () = assert!(size_of::<types::Type>() == 48);
const _: () = assert!(size_of::<ast::Pattern>() == 120);
const _: () = assert!(size_of::<ast::Span>() == 12);
```
If you add/remove fields from these types, the assertions must be updated.

### Module IDs

- Lisette modules: plain identifiers like `"main"`, `"store"`, `"models"`
- Go stdlib imports: prefixed with `"go:"` — e.g., `"go:fmt"`, `"go:net/http"`
- Prelude: the special `"prelude"` module
- Entry module: semantic constant `ENTRY_MODULE_ID = 0`

### File extensions

- `.lis` — Lisette source file
- `.d.lis` — Lisette type definition file (header-only, no bodies)
- `.snap` — Insta test snapshot

### Project manifest

Projects have a `lisette.toml` at their root with a `[project]` section containing `name` and `go_module`. The `deps::project_manifest` module handles parsing.

### Go version synchronization

Five files must agree on Go version. The `go-version` file is the source of truth:
- `go-version`
- `prelude/go.mod`
- `bindgen/go.mod`
- `bindgen/internal/cli/metadata/go-version`
- `crates/cli/go-version`

The `lefthook.yaml` pre-commit hook enforces this.

### Pre-commit hooks

Managed by `lefthook.yaml`:
- `commit-msg` — conventional commits format: `type: description` (lowercase after colon), ≤72 chars
- `pre-commit` — `just check` + bindgen Go checks + Go version sync verification

### The `check` alias significance

`just c` = `just check` = `format-check + test + test-unit + lint`. This is THE full CI check. Run it before pushing. The CI workflow also runs this.

### Release process

Managed by `release-plz` with `release-plz.toml`. The `release.yml` workflow is auto-generated by `cargo-dist`. Release notes generated by `git-cliff` (`cliff.toml`). Tags use the pattern `lisette-v{version}`.

### CLI and the `lis learn` command

The `lis learn` command creates `AGENTS.md` and `lisette.toml` in a new project's root. The template is embedded at compile time via `agents_template.md` in `crates/cli/`. When updating the embedded template, also consider whether the repository-level `AGENTS.md` should reflect similar changes.

### Bindgen

`bindgen/` is a Go program (`main.go`) that reads Go source (stdlib or third-party), analyzes it, and generates `.d.lis` typedef stubs. Output goes to `crates/stdlib/typedefs/`. The `justfile` has targets for regenerating these:

```bash
just generate-stdlib-typedefs <go_version>    # regenerate stdlib typedefs
just check-stdlib-drift                        # check if typedefs are stale
```

### Constraints

- Maximum source file: 10 MiB (`MAX_SOURCE_BYTES` in `crates/syntax/src/lib.rs`)
- Rust 2024 edition, minimum Rust 1.94
- Tests in `tests/` use `edition = "2024"` (inherited from workspace)

## Common patterns

### Error handling

- `semantics` returns `Result<T, LisetteDiagnostic>` from most operations
- Diagnostics use `LocalSink::push()` for non-fatal errors — accumulation over abort
- Parse errors are collected, not short-circuited; the parser recovers and continues

### Visitor pattern

The `ast_walk` lint module uses a visitor pattern. Adding a new lint that walks the AST: add a method to `visitor.rs` and a corresponding lint implementation in `checks.rs`.

### Pattern analysis (exhaustiveness checking)

The Maranget algorithm implementation lives in `semantics/src/passes/checks/pattern_analysis/`. It determines whether match expressions are exhaustive. Uses witness generation to provide counter-examples for non-exhaustive matches.

### Interface satisfaction

Lisette interfaces are structural (like Go). `checker/infer/interface.rs` handles interface satisfaction checking.

### Cache

The cache system in `semantics/src/cache/` uses content hashing. Cached artifacts include:
- Prelude (parsed once, reused)
- Go stdlib typedefs (per Go version, per target arch)
- Per-module compiled outputs (identified by source hash + dependency hashes)

`CompilePhase` enum controls if compilation stops at `Check` (type check only) or proceeds to `Emit` (full Go code generation).

### The `target/` directory

Generated Go files go in `target/` under the project root. `target/` is gitignored by the compiler's `.gitignore` template. The CLI acquires a file lock on `target/` to prevent concurrent builds.
