# compactp Design Specification

## 1. Overview

compactp is a production-grade Rust frontend for the Compact language used by the Midnight Network. It provides a reusable crate stack for lexing, parsing, CST construction, typed AST access, and diagnostics, plus a prebuilt CLI for humans, CI, and downstream toolchains.

This is a parser-first tool. It does not generate JavaScript, TypeScript, ZKIR, proving keys, or runtime artifacts in v1.

The implementation targets the latest upstream Compact compiler syntax surface (language version 0.22.0), cross-checked against the official tree-sitter grammar, the upstream example/test corpus, and formal specification material.

### 1.1 Product Surfaces

Three equally first-class surfaces:

- **Rust library API** — crate stack consumable by external projects
- **CLI** — `compactp` binary for humans and CI
- **Machine-readable JSON** — stable contracts for downstream tooling

### 1.2 Naming

The project uses the `compactp` prefix (parser) to avoid collision with `compactc` (compiler) and other Compact ecosystem tooling. Crate names: `compactp_syntax`, `compactp_lexer`, `compactp_parser`, `compactp_ast`, `compactp_diagnostics`. The CLI binary is `compactp`.

## 2. Architecture

### 2.1 Crate Dependency Graph

```
compactp (binary)
├── compactp_ast
│   └── compactp_syntax
├── compactp_diagnostics
│   └── compactp_syntax
├── compactp_parser
│   ├── compactp_lexer
│   │   └── compactp_syntax
│   └── compactp_syntax
└── (clap, notify-debouncer-full, serde_json)
```

### 2.2 Build Order

Each crate is completed before its consumers:

1. **compactp_syntax** — `SyntaxKind` enum, rowan `Language` trait impl. Foundation for everything.
2. **compactp_lexer** — logos-based tokenizer producing `(SyntaxKind, &str)` pairs.
3. **compactp_parser** — Recursive descent + Pratt parser with marker-based events, builds rowan `GreenNode`.
4. **compactp_ast** (parallel with diagnostics) — Typed AST wrappers over `SyntaxNode`.
5. **compactp_diagnostics** (parallel with ast) — Diagnostic model, human/JSON renderers.
6. **compactp** — CLI binary pulling everything together.

### 2.3 Key Dependency Insight

`compactp_ast` depends on `compactp_syntax`, not `compactp_parser`. The AST layer works over rowan `SyntaxNode` values regardless of how they were produced. This keeps AST wrappers testable independently and means the parser could be swapped without touching AST code.

## 3. Parser Pipeline

### 3.1 Architecture Choice: Marker-Based Event Pipeline

The parser uses a marker-based event pipeline (the pattern used by rust-analyzer):

```
Source (&str)
  → Lexer (logos)
  → Token Source (Vec<(SyntaxKind, &str)>)
  → Parser (recursive descent + Pratt, with markers)
  → Events (Vec<Event>)
  → Sink (event → rowan conversion)
  → GreenNode (rowan CST)
```

This architecture was chosen over direct tree building and plain event pipelines because:

- Markers with `precede()` are essential for Pratt expression parsing — wrapping already-parsed left-hand sides in new binary expression nodes.
- Abandoned markers enable cheap speculative parsing.
- The event stream decouples the parser from tree construction, making it testable in isolation.
- Recovery is cleaner — emit error events without managing builder state.

### 3.2 Core Internal Types

#### Event

```rust
enum Event {
    StartNode { kind: SyntaxKind, forward_parent: Option<u32> },
    Token { kind: SyntaxKind, n_raw_tokens: u8 },
    FinishNode,
    Error { message: String },
    Placeholder, // tombstone for abandoned markers
}
```

`forward_parent` enables Pratt parsing: when a marker uses `precede()`, the new parent is inserted before the existing node via a forward pointer. The sink resolves these into proper nesting.

#### Marker

```rust
struct Marker {
    pos: u32,       // index into events vec
    bomb: DropBomb, // panics if not completed or abandoned
}

impl Marker {
    fn complete(self, p: &mut Parser, kind: SyntaxKind) -> CompletedMarker;
    fn abandon(self, p: &mut Parser);
}

impl CompletedMarker {
    fn precede(self, p: &mut Parser) -> Marker;
}
```

#### Parser (internal)

```rust
struct Parser<'src> {
    tokens: Vec<(SyntaxKind, &'src str)>,
    pos: usize,
    events: Vec<Event>,
    expected: Vec<SyntaxKind>,
}
```

Key methods: `current()`, `nth(n)`, `at(kind)`, `eat(kind)`, `expect(kind)`, `bump(kind)`, `start()`, `error(msg)`.

Trivia skipping happens transparently in `current()` and `nth()` — parser logic never sees whitespace or comments.

### 3.3 Expression Precedence

The Pratt parser uses these binding power levels (low to high):

| BP | Level | Operators | Associativity |
|----|-------|-----------|---------------|
| 1 | Conditional | `? :` | Right |
| 2 | Logical OR | `\|\|` | Left |
| 3 | Logical AND | `&&` | Left |
| 4 | Equality | `== !=` | Left |
| 5 | Relational | `< <= > >=` | None (no chaining) |
| 6 | Cast | `as` | Left |
| 7 | Additive | `+ -` | Left |
| 8 | Multiplicative | `*` | Left |
| 9 | Unary prefix | `!` | — |
| 10 | Postfix | `. [] ()` | Left |

### 3.4 Error Recovery

**Recovery sets:** Each grammar rule defines a set of tokens that can follow it. On unexpected input, the parser wraps tokens up to the next recovery-set member in an `ERROR` node.

**Synchronization points:** Top-level keywords (`circuit`, `ledger`, `export`, etc.) and closing delimiters (`}`, `)`, `]`) are synchronization points the parser never skips past.

**Missing tokens:** For missing semicolons, commas, and closing delimiters — emit a diagnostic but don't insert phantom tokens. The CST simply lacks the token.

**Error budgeting:** After 256 errors (configurable), the parser stops recovery and wraps remaining tokens in ERROR nodes.

### 3.5 Trivia Attachment

Leading trivia (whitespace/comments before a token) attaches to the following token. Trailing trivia on the same line stays with the preceding token. This gives predictable behavior — comments above a function attach to that function.

## 4. SyntaxKind Enum

A flat `#[repr(u16)]` enum with ~120-140 variants. Every variant is either a token (leaf) or a node (interior).

### 4.1 Token Kinds

**Trivia:** `WHITESPACE`, `LINE_COMMENT`, `BLOCK_COMMENT`

**Literals:** `INT_LIT`, `HEX_LIT`, `OCT_LIT`, `BIN_LIT`, `STRING_LIT`, `VERSION_LIT`, `TRUE_KW`, `FALSE_KW`

**Keywords:** Each keyword gets its own `*_KW` variant — `PRAGMA_KW`, `INCLUDE_KW`, `IMPORT_KW`, `FROM_KW`, `PREFIX_KW`, `EXPORT_KW`, `MODULE_KW`, `LEDGER_KW`, `CONSTRUCTOR_KW`, `CIRCUIT_KW`, `WITNESS_KW`, `CONTRACT_KW`, `STRUCT_KW`, `ENUM_KW`, `TYPE_KW`, `CONST_KW`, `RETURN_KW`, `IF_KW`, `ELSE_KW`, `FOR_KW`, `OF_KW`, `ASSERT_KW`, `AS_KW`, `PURE_KW`, `SEALED_KW`, `NEW_KW`, `MAP_KW`, `FOLD_KW`, `DEFAULT_KW`, `DISCLOSE_KW`, `PAD_KW`, `SLICE_KW`

**Builtin type keywords:** `BOOLEAN_KW`, `FIELD_KW`, `UINT_KW`, `BYTES_KW`, `OPAQUE_KW`, `VECTOR_KW`

**Operators:** `EQ`, `PLUS_EQ`, `MINUS_EQ`, `EQ_EQ`, `BANG_EQ`, `LT`, `LT_EQ`, `GT`, `GT_EQ`, `AMP_AMP`, `PIPE_PIPE`, `PLUS`, `MINUS`, `STAR`, `SLASH`, `BANG`, `QUESTION`, `FAT_ARROW`, `DOT`, `DOT_DOT`, `DOT_DOT_DOT`

Note on `SLASH`: The upstream compiler lexer produces `/` as a binop token (after ruling out `//` and `/*` comments). It is not observed in any example expressions and may not be accepted by the upstream parser in expression contexts. The token is included because the lexer must handle it — if encountered in source, it will be lexed as `SLASH` rather than `ERROR`. The Pratt parser does not assign it a binding power; if it appears in an expression position, it will be captured as an error.

**Delimiters:** `L_PAREN`, `R_PAREN`, `L_BRACE`, `R_BRACE`, `L_BRACKET`, `R_BRACKET`, `COMMA`, `SEMICOLON`, `COLON`, `HASH`

**Special:** `IDENT`, `ERROR`, `EOF`

Design notes:
- `LT`/`GT` serve double duty as comparison operators and generic angle brackets — the parser disambiguates contextually.
- Each keyword is its own variant to enable precise pattern matching without string comparisons.
- `CIRCUIT_DEF` vs `CIRCUIT_DECL` at the node level distinguishes bodies from interface declarations.
- Boolean literals are keyword tokens, not identifiers, matching upstream compiler behavior.
- `TYPE_KW` is included as a reserved keyword token. No `TYPE_DECL` node kind is defined in v1 because neither the upstream compiler parser, examples, nor tree-sitter grammar show type alias declarations as accepted syntax. The `type` keyword is reserved for future use. If upstream adds type alias syntax, a `TYPE_DECL` node kind and corresponding AST type will be added.
- Unary minus (`-expr`) is not part of the Compact expression surface. Compact operates on Field elements (unsigned) and no examples or compiler grammar rules show unary negation. `MINUS` appears only in binary subtraction (`expr - expr`) and compound assignment (`-=`). The Pratt parser does not assign a prefix binding power to `MINUS`.
- `HASH` is used in generic parameter declarations (`# tvar-name`) per the tree-sitter grammar's `generic_param` rule.

### 4.2 Node Kinds

**Top-level:** `SOURCE_FILE`, `PRAGMA`, `INCLUDE`, `IMPORT`, `IMPORT_SPECIFIER`, `IMPORT_SPECIFIER_LIST`, `EXPORT_LIST`, `MODULE_DEF`, `LEDGER_DECL`, `CONSTRUCTOR_DEF`, `CIRCUIT_DEF`, `CIRCUIT_DECL`, `WITNESS_DECL`, `CONTRACT_DECL`, `CONTRACT_CIRCUIT`, `STRUCT_DEF`, `STRUCT_FIELD`, `ENUM_DEF`, `ENUM_VARIANT`

**Types:** `TYPE_REF`, `BOOLEAN_TYPE`, `FIELD_TYPE`, `UINT_TYPE`, `BYTES_TYPE`, `OPAQUE_TYPE`, `VECTOR_TYPE`, `TUPLE_TYPE`, `GENERIC_ARG_LIST`, `GENERIC_ARG`, `GENERIC_PARAM_LIST`, `GENERIC_PARAM`, `TYPE_SIZE`

**Patterns:** `IDENT_PAT`, `TUPLE_PAT`, `TUPLE_PAT_ELT`, `STRUCT_PAT`, `STRUCT_PAT_FIELD`, `TYPED_PAT`

**Statements:** `BLOCK`, `ASSIGN_STMT`, `EXPR_STMT`, `RETURN_STMT`, `IF_STMT`, `FOR_STMT`, `ASSERT_STMT`, `CONST_STMT`, `MULTI_CONST_STMT`

**Expressions:** `TERNARY_EXPR`, `BINARY_EXPR`, `UNARY_EXPR`, `CAST_EXPR`, `CALL_EXPR`, `MEMBER_EXPR`, `INDEX_EXPR`, `PAREN_EXPR`, `EXPR_SEQ`, `ARRAY_EXPR`, `BYTES_EXPR`, `SPREAD_EXPR`, `STRUCT_EXPR`, `STRUCT_FIELD_INIT`, `STRUCT_UPDATE`, `DEFAULT_EXPR`, `MAP_EXPR`, `FOLD_EXPR`, `DISCLOSE_EXPR`, `PAD_EXPR`, `SLICE_EXPR`, `LAMBDA_EXPR`, `PARAM_LIST`, `PARAM`, `RANGE_EXPR`, `PREFIX_DECL`

**Version expressions:** `VERSION_EXPR`, `VERSION_AND_EXPR`, `VERSION_OR_EXPR`, `VERSION_UNARY_EXPR`, `VERSION_PAREN_EXPR`

**Recovery:** `ERROR` — wraps malformed syntax, always structurally visible in the CST.

Note on `MULTI_CONST_STMT`: LS.md Section 7.5 documents comma-separated multi-const constructs observed in corpus files (e.g., `const a = 1, b = 2;`). This is captured as a distinct node kind to preserve the source form even though it may be semantically equivalent to multiple single-const statements.

## 5. AST Layer

### 5.1 Wrapper Pattern

Each AST type is a zero-cost newtype over `SyntaxNode`:

```rust
trait AstNode {
    fn can_cast(kind: SyntaxKind) -> bool;
    fn cast(node: SyntaxNode) -> Option<Self>;
    fn syntax(&self) -> &SyntaxNode;
}

struct CircuitDef(SyntaxNode);

impl CircuitDef {
    fn export_kw(&self) -> Option<SyntaxToken>;
    fn pure_kw(&self) -> Option<SyntaxToken>;
    fn name(&self) -> Option<SyntaxToken>;
    fn generic_params(&self) -> Option<GenericParamList>;
    fn params(&self) -> impl Iterator<Item = Param>;
    fn return_type(&self) -> Option<Type>;
    fn body(&self) -> Option<Block>;
    fn is_exported(&self) -> bool;
    fn is_pure(&self) -> bool;
}
```

No allocation, no cloning — just views into the existing CST.

### 5.2 Sum Types

Grammar positions that accept multiple node kinds use Rust enums:

```rust
enum Expr {
    Ternary(TernaryExpr),
    Binary(BinaryExpr),
    Unary(UnaryExpr),
    Cast(CastExpr),
    Call(CallExpr),
    // ...
}
```

`Type`, `Stmt`, `Pat`, and `Expr` are all sum-type enums, giving callers exhaustive matching.

### 5.3 AST Type Inventory

**Top-level:** SourceFile, Pragma, Include, Import, ImportSpecifier, ExportList, ModuleDef, LedgerDecl, ConstructorDef, CircuitDef, CircuitDecl, WitnessDecl, ContractDecl, ContractCircuit, StructDef, EnumDef

**Types:** Type (enum), TypeRef, BooleanType, FieldType, UintType, BytesType, OpaqueType, VectorType, TupleType, GenericArgList, GenericParamList

**Statements:** Stmt (enum), Block, AssignStmt, ConstStmt, MultiConstStmt, ExprStmt, ReturnStmt, IfStmt, ForStmt, AssertStmt

**Expressions:** Expr (enum), TernaryExpr, BinaryExpr, UnaryExpr, CastExpr, CallExpr, MemberExpr, IndexExpr, ArrayExpr, BytesExpr, SpreadExpr, StructExpr, DefaultExpr, MapExpr, FoldExpr, DiscloseExpr, PadExpr, SliceExpr, LambdaExpr, ParenExpr

**Patterns:** Pat (enum), IdentPat, TuplePat, StructPat

## 6. Diagnostics

### 6.1 Model

```rust
struct Diagnostic {
    severity: Severity,           // Error, Warning, Note
    code: DiagnosticCode,         // e.g. E0001
    message: String,
    primary_span: TextRange,
    secondary_spans: Vec<LabeledSpan>,
    notes: Vec<String>,
}

enum Severity { Error, Warning, Note }
```

Diagnostics are collected during parsing but stored separately from the CST. `ParseResult` contains both the `GreenNode` (always present, even with errors) and `Vec<Diagnostic>`. A tree with diagnostics is still valid and traversable.

### 6.2 Rendering

**Human output:** ANSI color support (`--color auto|always|never`), source snippets with path/line/column, rustc-style formatting.

**JSON output:** Structured diagnostics with severity, code, message, spans. Part of the stable JSON contract.

## 7. CLI

### 7.1 Commands

| Command | Purpose |
|---------|---------|
| `lex` | Tokenize and print token stream |
| `parse` | Parse and report diagnostics |
| `cst` | Dump lossless concrete syntax tree |
| `ast` | Dump typed AST |
| `diag` | Emit diagnostics only |
| `stats` | Token/node counts, parse time, file size |
| `watch` | Watch files/dirs and rerun on change |

### 7.2 Global Options

- `--format human|json|jsonl`
- `--pretty`
- `--color auto|always|never`
- `--timing`
- `--stdin-filename <path>`
- `--max-diagnostics <n>`
- `--no-recover`
- `--stop-after lex|parse|cst|ast` — halt pipeline at the specified stage and output results so far
- `--language-version <version>`

### 7.3 Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Syntax errors |
| 2 | IO/file error |
| 3 | Invalid CLI usage |
| 4 | Internal failure |

### 7.4 Input Handling

- Files: one or more paths
- Directories: recursive, filtering to `*.compact`
- Stdin: when `-` is passed or piped input detected
- Mixed: deterministic ordering (argument order for files, alphabetical for directories)
- Symlinks: followed with cycle detection
- `--stdin-filename` sets display path for stdin in diagnostics

### 7.5 Watch Mode

**Human mode:** Clears terminal, shows changed file, pass/fail, concise diagnostics. Debounces rapid saves (200ms default).

**Machine mode:** JSONL events with schema version, event kind, timestamp, affected files, parse result, diagnostics.

### 7.6 JSON Output Contract

Every JSON payload includes envelope metadata:

```json
{
  "tool_version": "0.1.0",
  "schema_version": 1,
  "language_version": "0.22.0",
  "input": "counter.compact",
  "timing_ms": 1.23,
  "data": { }
}
```

Schema version bumps on breaking changes. This is a public contract from day one.

### 7.7 Deferred CLI Options

`--fail-on-warn` is deferred until warnings are implemented. The flag will be added when the diagnostic model produces warning-severity diagnostics (currently only errors and notes are emitted by the parser).

## 8. Public Rust API

```rust
// compactp_lexer
fn lex(source: &str) -> Vec<(SyntaxKind, &str)>;

// compactp_parser
struct ParseResult {
    green: GreenNode,
    diagnostics: Vec<Diagnostic>,
}

fn parse(source: &str) -> ParseResult;                    // sugar for parse_with(source, ParseOptions::default())
fn parse_with(source: &str, opts: ParseOptions) -> ParseResult;
fn parse_file(path: &Path) -> Result<ParseResult, IoError>;

struct ParseOptions {
    recover: bool,         // default: true
    max_errors: usize,     // default: 256
}

// compactp_ast
fn source_file(node: &SyntaxNode) -> Option<SourceFile>;

// compactp_diagnostics
fn render_human(diag: &Diagnostic, source: &str, colored: bool) -> String;
fn render_json(diag: &Diagnostic) -> serde_json::Value;
```

Design requirements: no hidden global mutable state, option structs instead of boolean soup, explicit error types, syntax errors in parse results (not hard errors), thread-safe data structures.

## 9. Project Layout

```
compact-ast/
├── Cargo.toml                    # workspace root
├── Cargo.lock
├── LICENSE
├── .gitignore
├── crates/
│   ├── compactp_syntax/
│   ├── compactp_lexer/
│   ├── compactp_parser/
│   │   └── src/grammar/          # declarations, expressions, statements,
│   │                             # types, patterns, imports, version
│   ├── compactp_ast/
│   ├── compactp_diagnostics/
│   └── compactp/                 # CLI binary
│       └── src/commands/         # lex, parse, cst, ast, diag, stats, watch
├── tests/
│   ├── corpus/                   # all 489 upstream .compact files
│   │   └── LICENSE-APACHE-2.0    # upstream license preserved
│   └── fixtures/                 # curated per parser concern
└── docs/
```

### 9.1 .gitignore

```
FUNC_SPEC.md
LS.md
docs/superpowers/
.superpowers/
references/
```

## 10. Dependencies

All versions verified via `cargo search`.

| Crate | Version | Used By | Purpose |
|-------|---------|---------|---------|
| rowan | 0.16 | syntax, parser, ast | Lossless syntax tree |
| logos | 0.16 | lexer | Tokenizer generation |
| serde | 1.0 | diagnostics, cli | Serialization |
| serde_json | 1.0 | diagnostics, cli | JSON output |
| clap | 4.6 | cli | CLI argument parsing |
| notify-debouncer-full | 0.7 | cli (watch) | File watching |
| drop_bomb | *verify* | parser | Marker must-use enforcement |

**Dev dependencies:** insta 1.46 (snapshots), expect-test (inline snapshots), criterion (benchmarks), assert_cmd (CLI tests). Versions marked *verify* will be confirmed at implementation time.

**Edition:** 2024, latest stable MSRV.

**License:** MIT.

## 11. Testing Strategy

### 11.1 Test Categories

1. **Lexer snapshot tests** — every token kind, keyword classification, trivia, numeric literal forms, string escapes, identifiers with `$`
2. **CST snapshot tests** — one test per grammar construct in `tests/fixtures/`, capturing exact tree shape including trivia
3. **AST accessor tests** — unit tests verifying typed wrapper accessors
4. **Recovery tests** — intentionally broken source files; verify no panics, ERROR node placement, meaningful diagnostics, intact surrounding tree
5. **Diagnostic rendering tests** — snapshots for human and JSON output
6. **JSON contract tests** — parse known inputs, snapshot serialized JSON including envelope metadata
7. **Corpus parse tests** — bulk parse all 489 files; no panics, expected error/success per file
8. **CLI integration tests** — run `compactp` as subprocess with `assert_cmd`; verify exit codes, output format, file/stdin/directory handling

### 11.2 Test Corpus

- `tests/corpus/` — all 489 upstream `.compact` files mirroring upstream directory structure. Apache-2.0 license headers preserved.
- `tests/fixtures/` — curated subset organized by parser concern (declarations, expressions, imports, recovery, types, etc.) with snapshot expectations.

### 11.3 Benchmarks

Criterion benchmarks for: single-file lex, single-file parse, full-corpus parse, CST-to-AST cast, diagnostic rendering.

## 12. Source Compatibility

### 12.1 Source Hierarchy

The parser follows this source-of-truth order:

1. Upstream compiler source (references/compact/compiler)
2. Upstream accepted examples and tests
3. Official tree-sitter grammar
4. Public grammar/documentation page

### 12.2 Known Divergences

- **Tree-sitter identifier rule vs real examples:** Tree-sitter uses a simplified identifier regex lacking `$`. Parser follows upstream compiler behavior (`$` and `_` allowed).
- **Tree-sitter import grammar vs examples:** Tree-sitter models simpler imports. Parser supports the richer `import { ... } from ... prefix ...` form.
- **Numeric literals:** Tree-sitter shows only simple naturals. Parser supports hex (`0x`), octal (`0o`), and binary (`0b`) per upstream compiler.
- **Bytes/spread/slice surface:** Not fully modeled in tree-sitter. Parser treats these as required coverage.
- **Assert syntax:** Parser uses call-like `assert(condition, "message")` per examples, not bare `assert expr str`.
- **Public docs version (0.21.0) vs compiler version (0.22.0):** Parser targets 0.22.0.

### 12.3 Version-Gated Parsing

The architecture supports future version-gated parsing via `ParseOptions` if syntax versions diverge enough to require branching behavior.
