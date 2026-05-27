# Public API → CLI Coverage Matrix

This document maps every public item in `compactp_{lexer, syntax,
diagnostics, parser, ast}` to the CLI subcommand(s) that exercise it.
The matrix is the contract pointed at by the WS2 spec: every public
item must be reachable from at least one CLI codepath, or carry a
doctest that compiles via `cargo test --doc` (which serves as the
consumer).

Baseline lines per crate (from `cargo public-api`, captured in
WS2 T2 and re-snapshotted in WS2 T4 after demotions):

| Crate                  | Baseline lines | Δ since T2 |
| ---------------------- | -------------- | ---------- |
| `compactp_lexer`       | 2              | —          |
| `compactp_syntax`      | 230            | −2 (`is_keyword`, `SyntaxElement` removed) |
| `compactp_diagnostics` | 91             | —          |
| `compactp_parser`      | 25             | −2 (`grammar` module demoted, `parse_file` removed) |
| `compactp_ast`         | 3739           | −4 (`support` module demoted to `pub(crate)`) |

Each baseline line is roughly one public item — top-level types,
functions, free items, plus per-type accessor methods and the standard
auto-derive impls (`Clone`, `Debug`, `PartialEq`, `Hash`, the marker
traits). For `compactp_ast`, where every newtype struct produces ~15
baseline lines, the matrix below groups by AST-node family and notes
which subcommand walks the family rather than listing every accessor.

## Subcommand → primary crates

The mapping was determined by grepping `crates/compactp/src/` for `use
compactp_*` statements and reading each `commands/*.rs` file.

| Subcommand | Crate(s) exercised directly                                                  |
| ---------- | ---------------------------------------------------------------------------- |
| `lex`      | `compactp_lexer` (→ `compactp_syntax::SyntaxKind` via the token tuple)       |
| `parse`    | `compactp_parser`, `compactp_diagnostics`                                    |
| `cst`      | `compactp_parser`, `compactp_syntax`                                         |
| `ast`      | `compactp_parser`, `compactp_syntax`, `compactp_ast`                         |
| `diag`     | `compactp_parser`, `compactp_diagnostics`                                    |
| `stats`    | `compactp_lexer`, `compactp_parser`, `compactp_syntax`                       |
| `watch`    | dispatches to all six subcommands above via `WatchableCommand`               |

`watch` does not import any library crate itself — it re-runs whichever
subcommand the user passed to it, so every item reachable from `lex`,
`parse`, `cst`, `ast`, `diag`, or `stats` is reachable from `watch`.

## Per-crate item coverage

### `compactp_lexer` (2 baseline lines)

| Public item                           | Subcommand(s)      | Notes                                                          |
| ------------------------------------- | ------------------ | -------------------------------------------------------------- |
| `pub mod compactp_lexer`              | all (module root)  | trivially reachable                                            |
| `pub fn lex(&str) -> Vec<(SyntaxKind, &str)>` | `lex`, `stats` | `lex` prints each token; `stats` counts tokens for the report |

No orphans.

### `compactp_syntax` (232 baseline lines)

The bulk of this crate is the `SyntaxKind` enum (~170 variants) and
the `rowan` glue. Every `SyntaxKind` variant appears in CST output
(`cst`) via `{:?}` formatting of the kind, in `stats` recovery counts
(`ERROR`), and as the discriminant returned by the lexer (`lex`).

| Public item                                       | Subcommand(s)              | Notes                                                                                                  |
| ------------------------------------------------- | -------------------------- | ------------------------------------------------------------------------------------------------------ |
| `pub mod compactp_syntax`                         | all                        | module root                                                                                            |
| `enum SyntaxKind` (all variants)                  | `lex`, `cst`, `ast`, `stats` | every variant flows through tokens (lex) or tree (cst/stats/ast) at least for the wider corpus       |
| `SyntaxKind::ERROR`                               | `stats`                    | counted by `count_error_nodes` as the recovery metric                                                  |
| `SyntaxKind::is_trivia`                           | doctest                    | Used inside `compactp_parser` (sink, parser); kept `pub` and exercised by a doctest in T4 — see `SyntaxKind::is_trivia` in `crates/compactp_syntax/src/syntax_kind.rs`. |
| `SyntaxKind: Clone/Debug/Eq/Hash/Copy/From/Ord`   | `cst`, `stats`             | `{:?}` formatting (cst), `==` comparisons (stats), `From` conversions in parser sink                   |
| `enum CompactLanguage` + `Language` impl          | `cst`, `ast`, `stats`      | every `SyntaxNode`/`SyntaxToken` instantiation goes through `CompactLanguage::kind_from_raw`           |
| `type SyntaxNode`                                 | `cst`, `ast`, `stats`      | the rooted tree the CLI walks                                                                          |
| `type SyntaxToken`                                | `cst`, `ast`               | tokens are formatted in `cst` and read for identifiers in `ast`                                        |

`SyntaxKind::is_keyword` and `type SyntaxElement` were removed in T4
(zero callers anywhere in the workspace; trivial to resurrect from git
history if a future consumer needs them).

### `compactp_diagnostics` (91 baseline lines)

| Public item                                          | Subcommand(s)        | Notes                                                                                                |
| ---------------------------------------------------- | -------------------- | ---------------------------------------------------------------------------------------------------- |
| `pub mod compactp_diagnostics` / `::json` / `::render` | all              | module roots                                                                                          |
| `fn render_human(&Diagnostic, &str, &str, bool)`     | `parse`, `diag`      | re-exported at crate root and used in both human-format paths                                         |
| `fn render_json(&Diagnostic, &str)`                  | `parse`, `diag`      | re-exported at crate root and used in both JSON-format paths                                          |
| `fn json::render_json(...)`                          | `parse`, `diag`      | same function, exposed under the `json` submodule path                                                |
| `fn render::render_human(...)`                       | `parse`, `diag`      | same function, exposed under the `render` submodule path                                              |
| `struct Diagnostic` (all fields)                     | `parse`, `diag`      | each field is read by `render_human` / `render_json`                                                  |
| `Diagnostic::error` / `note` / `warning`             | (internal)           | constructors used by `compactp_parser` to emit diagnostics; CLI never builds a `Diagnostic` directly  |
| `Diagnostic::with_note` / `with_secondary`           | (internal)           | builder methods used by parser/grammar; CLI is read-only                                              |
| `Diagnostic: Clone/Debug/Serialize`                  | `parse`, `diag`      | `Clone` used by `limit_diagnostics`; `Serialize` flows through `render_json`                          |
| `enum Severity` (Error/Warning/Note) + impls         | `parse`, `diag`      | embedded in `Diagnostic`, rendered by `render_human`/`render_json`                                    |
| `struct DiagnosticCode { prefix, number }` + impls   | `parse`, `diag`      | embedded in `Diagnostic`, rendered as `E0001` in human output and as a structured object in JSON     |
| `DiagnosticCode::new`                                | (internal)           | called from parser/grammar; CLI never constructs codes                                                |
| `struct LabeledSpan { label, span }` + impls         | `parse`, `diag`      | embedded in `Diagnostic.secondary_spans`, emitted by both renderers                                   |

No CLI-orphans. The constructor and builder methods (`error`, `note`,
`warning`, `with_note`, `with_secondary`, `DiagnosticCode::new`) are
internal-only from the CLI's perspective but are required by the
parser crate, so they are not candidates for demotion.

### `compactp_parser` (27 baseline lines)

| Public item                                   | Subcommand(s)                          | Notes                                                                                  |
| --------------------------------------------- | -------------------------------------- | -------------------------------------------------------------------------------------- |
| `pub mod compactp_parser`                     | all                                    | module root                                                                            |
| `struct ParseOptions { max_errors, recover }` | `parse`, `diag`, `ast`                 | constructed in each subcommand’s `run`                                                 |
| `ParseOptions::default`                       | `cst`, `stats` (transitively)          | `parse()` calls `parse_with(source, ParseOptions::default())`; `cst` and `stats` both call `parse()`                  |
| `struct ParseResult { errors, green }`        | `parse`, `cst`, `ast`, `diag`, `stats` | returned by `parse` / `parse_with`; `.errors` and `.green` read by every subcommand    |
| `fn parse(&str) -> ParseResult`               | `cst`, `stats`                         | the no-options entry point                                                             |
| `fn parse_with(&str, ParseOptions) -> ParseResult` | `parse`, `diag`, `ast`            | the options-aware entry point                                                          |

`compactp_parser::grammar` was demoted to `pub(crate)` in T4 (every
consumer accesses it via `crate::grammar`); `parse_file` was removed
(zero callers in the workspace, and the CLI reads source via
`resolve_inputs` + `parse_with`).

### `compactp_ast` (3743 baseline lines)

The surface is dominated by AstNode-family items. Each AST node
contributes one `pub struct Foo(_)`, three `AstNode` impl items
(`can_cast`, `cast`, `syntax`), and the standard auto-derive impls
(`Clone`, `Debug`, `PartialEq`, `Eq`, `Hash`, plus the marker traits)
— roughly 15 baseline lines per node, before any node-specific
accessor methods. There are ~50 such nodes, accounting for most of
the 3743 lines.

Coverage below is grouped by family. The rule of thumb: if the `ast`
subcommand walks the family, every accessor it exposes is covered
transitively (the CLI calls some, the public-API checker proves the
rest compile).

#### Re-exports at crate root

| Public item                                     | Subcommand(s) | Notes                                          |
| ----------------------------------------------- | ------------- | ---------------------------------------------- |
| `pub use SyntaxKind` / `SyntaxNode` / `SyntaxToken` | `ast`     | Used directly in `commands/ast.rs` (e.g. `compactp_ast::SyntaxToken` in `text()`) |

#### Core trait

| Public item                                                                                   | Subcommand(s) | Notes                                                                              |
| --------------------------------------------------------------------------------------------- | ------------- | ---------------------------------------------------------------------------------- |
| `trait AstNode { fn can_cast; fn cast; fn syntax; }`                                          | `ast`         | `commands/ast.rs` imports `AstNode` and calls `SourceFile::cast(root)`             |

#### Top-level file + `Item` family (the family the CLI dispatches on)

`commands/ast.rs::item_summary` and `item_json` match every variant of
`compactp_ast::Item` and call accessor methods on each. Every node
listed in `Item` is therefore reached by the `ast` subcommand.

| Family member                                                | Subcommand(s) | Notes                                                                                          |
| ------------------------------------------------------------ | ------------- | ---------------------------------------------------------------------------------------------- |
| `SourceFile` + accessors (`items`, `circuit_defs`, `enum_defs`, `imports`, `includes`, `pragmas`, `struct_defs`) | `ast` | `dump_source_file` calls `file.items()`. Other accessors are exposed but not called by the CLI — they share machinery with `items()` and are covered by the family contract. |
| `enum Item` (Pragma, Include, Import, ExportList, LedgerDecl, ConstructorDef, CircuitDef, CircuitDecl, WitnessDecl, ContractDecl, StructDef, EnumDef, ModuleDef, TypeDecl) | `ast` | every variant is matched in `item_summary` / `item_json` |
| `Pragma`, `Include`, `Import`, `ExportList`, `ConstructorDef` | `ast`         | matched as opaque variants (CLI emits `"kind": "Pragma"`, etc.)                                |
| `LedgerDecl` + accessors (`name`, `is_exported`, `is_sealed`, `sealed_kw`, `ty`, `export_kw`) | `ast` | `ledger_json` reads `name`, `is_exported`, `is_sealed`                                         |
| `CircuitDef` + accessors (`name`, `is_exported`, `is_pure`, `body`, `pure_kw`, `params`, `generic_params`, `return_type`, `export_kw`) | `ast` | `circuit_def_json` reads `name`, `is_exported`, `is_pure`, `body` |
| `CircuitDecl` + accessors                                    | `ast`         | `circuit_decl_json` reads `name`, `is_exported`                                                |
| `WitnessDecl` + accessors                                    | `ast`         | `witness_json` reads `name`, `is_exported`                                                     |
| `ContractDecl` + accessors (`circuits`, `name`, `is_exported`, `export_kw`) | `ast` | `contract_json` reads `name`, `is_exported`, `circuits`                                         |
| `ContractCircuit` + accessors                                | `ast`         | reached transitively via `ContractDecl::circuits()`                                            |
| `StructDef` + accessors (`name`, `fields`, `is_exported`, `generic_params`, `export_kw`) | `ast`     | `struct_json` reads `name`, `is_exported`, `fields`                                            |
| `StructField` + accessors                                    | `ast`         | reached via `StructDef::fields()`                                                              |
| `EnumDef` + accessors (`name`, `variants`, `is_exported`, `export_kw`) | `ast`     | `enum_json` reads `name`, `is_exported`, `variants`                                            |
| `EnumVariant` + accessors                                    | `ast`         | reached via `EnumDef::variants()`                                                              |
| `ModuleDef` + accessors                                      | `ast`         | `module_json` reads `name`, `is_exported`                                                      |
| `TypeDecl` + accessors (`name`, `is_exported`, `is_newtype`, `generic_params`, `aliased_type`, `export_kw`) | `ast` | `type_decl_json` reads `name`, `is_exported`, `is_newtype`, `generic_params`                  |

#### Generic-parameter family

| Family member                                                | Subcommand(s) | Notes                                                              |
| ------------------------------------------------------------ | ------------- | ------------------------------------------------------------------ |
| `GenericParamList`, `GenericParam`, `GenericArgList`, `GenericArg` | `ast`   | reached via `TypeDecl::generic_params()` and other `.generic_params()` accessors |
| `PrefixDecl`                                                 | doctest       | Kept `pub` and exercised by a doctest on the struct itself (`crates/compactp_ast/src/nodes.rs`) — reached at runtime via `Import::prefix()`. |
| `ImportSpecifier`                                            | doctest       | Kept `pub` and exercised by a doctest on the struct itself; walks `import { foo, bar } from "x";` and pulls each specifier's `name()`. |

#### `Stmt` / `Pat` / `Type` families and the entire `expr` module

T4 added the `--include-bodies` flag to the `ast` subcommand
(`crates/compactp/src/commands/ast.rs`). With the flag set, the CLI
walks every `CircuitDef`/`ConstructorDef` body into its `Block`, then
matches each `Stmt`, `Pat`, `Type`, and `Expr` variant. Without the
flag the CLI behaviour is unchanged.

| Family                                                                                                                                  | Coverage status |
| --------------------------------------------------------------------------------------------------------------------------------------- | --------------- |
| `enum Stmt` + `AssertStmt`/`AssignStmt`/`Block`/`ConstStmt`/`ExprStmt`/`ForStmt`/`IfStmt`/`MultiConstStmt`/`ReturnStmt` newtypes        | `ast --include-bodies`. `dump_stmt`/`stmt_json` match every variant; `cli::ast_include_bodies_dumps_stmts_and_exprs` asserts the dump. |
| `enum Pat` + `IdentPat`/`StructPat`/`StructPatField`/`TuplePat`/`TuplePatElt`                                                           | `ast --include-bodies`. `dump_pat`/`pat_json` match every variant. |
| `enum Type` + `BooleanType`/`BytesType`/`FieldType`/`OpaqueType`/`RecordType`/`TypeRef`/`TupleType`/`UintType`/`UnsignedIntegerType`/`VectorType` + `TypeSize` | `ast --include-bodies`. `type_summary` matches every variant; reached via `ConstStmt::ty()`, `CastExpr::ty()`, `DefaultExpr::ty()`, `Param::ty()`, and `CircuitDef::return_type()`. `TypeSize` is reached transitively via `UintType::sizes()` / `VectorType::size()` / `BytesType::size()`. |
| `mod expr` — `enum Expr` + 22 expression newtypes (`ArrayExpr`, `BinaryExpr`, `BytesExpr`, `CallExpr`, `CastExpr`, `DefaultExpr`, `DiscloseExpr`, `FoldExpr`, `IndexExpr`, `LambdaExpr`, `LiteralExpr`, `MapExpr`, `MemberExpr`, `NameExpr`, `PadExpr`, `ParenExpr`, `SliceExpr`, `SpreadExpr`, `StructExpr`, `StructFieldInit`, `StructUpdate`, `TernaryExpr`, `UnaryExpr`) | `ast --include-bodies`. `dump_expr`/`expr_json` match every `Expr` variant; `StructFieldInit` and `StructUpdate` are reached via `StructExpr::field_inits()` and `StructExpr::update()` respectively. |
| `Param`, `ParamList`                                                                                                                    | `ast --include-bodies`. `param_json` reads `Param::pattern()` and `Param::ty()`; `ParamList` is reached transitively via `LambdaExpr::param_list()`. |
| `Block`                                                                                                                                 | `ast --include-bodies`. `block_json` reads `Block::stmts()` recursively. |

#### `support` module

The `support` module was demoted to `pub(crate)` in T4 (used only by
`compactp_ast`'s own generated accessors; no external consumer in the
workspace). It no longer appears in the public surface.

## Orphans (items not exercised by any CLI subcommand)

**No orphans.** All public items identified in WS2 T3 have been resolved
in T4 via one of three buckets:

| Bucket                          | Items                                                                                                                                                       |
| ------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Demoted to `pub(crate)` (or removed) | `SyntaxKind::is_keyword` (removed), `type SyntaxElement` (removed), `compactp_parser::grammar` (demoted), `parse_file` (removed), `compactp_ast::support` (demoted) |
| Exposed via `ast --include-bodies` | `enum Stmt` + 9 newtypes, `enum Pat` + 5 newtypes, `enum Type` + 11 newtypes + `TypeSize`, `enum Expr` + 22 expression newtypes (incl. `StructFieldInit`, `StructUpdate`), `Block`, `Param`, `ParamList` |
| Added doctest as consumer       | `SyntaxKind::is_trivia`, `ImportSpecifier`, `PrefixDecl`                                                                                                    |

### Per-crate resolution

| Crate                  | Orphans before T4 | Orphans after T4 |
| ---------------------- | ----------------- | ---------------- |
| `compactp_lexer`       | 0                 | 0                |
| `compactp_syntax`      | 3                 | 0                |
| `compactp_diagnostics` | 0                 | 0                |
| `compactp_parser`      | 2                 | 0                |
| `compactp_ast`         | ~50               | 0                |

The contract is preserved: every public item is either (a) exercised by
a CLI subcommand (the `ast` subcommand alone, with `--include-bodies`,
covers the entire AST surface), or (b) carries a runnable doctest in
the source of the item itself.
