# Public API → CLI Coverage Matrix

This document maps every public item in `compactp_{lexer, syntax,
diagnostics, parser, ast}` to the CLI subcommand(s) that exercise it.
The matrix is the contract pointed at by the WS2 spec: every public
item must be reachable from at least one CLI codepath, or carry a
doctest that compiles via `cargo test --doc` (which serves as the
consumer).

Baseline lines per crate (from `cargo public-api`, captured in
WS2 T2):

| Crate                  | Baseline lines |
| ---------------------- | -------------- |
| `compactp_lexer`       | 2              |
| `compactp_syntax`      | 232            |
| `compactp_diagnostics` | 91             |
| `compactp_parser`      | 27             |
| `compactp_ast`         | 3743           |

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
| `SyntaxKind::is_trivia`                           | (internal)                 | used inside `compactp_parser` (sink, parser); no CLI codepath touches it directly. **Orphan from the CLI’s perspective.** |
| `SyntaxKind::is_keyword`                          | (none)                     | **Orphan** — defined but never called inside the workspace.                                            |
| `SyntaxKind: Clone/Debug/Eq/Hash/Copy/From/Ord`   | `cst`, `stats`             | `{:?}` formatting (cst), `==` comparisons (stats), `From` conversions in parser sink                   |
| `enum CompactLanguage` + `Language` impl          | `cst`, `ast`, `stats`      | every `SyntaxNode`/`SyntaxToken` instantiation goes through `CompactLanguage::kind_from_raw`           |
| `type SyntaxNode`                                 | `cst`, `ast`, `stats`      | the rooted tree the CLI walks                                                                          |
| `type SyntaxToken`                                | `cst`, `ast`               | tokens are formatted in `cst` and read for identifiers in `ast`                                        |
| `type SyntaxElement`                              | (none)                     | **Orphan** — type alias, no consumer in this workspace.                                                |

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
| `pub mod compactp_parser::grammar`            | (none)                                 | **Orphan** — `grammar` is `pub mod` but every consumer (its own submodules, in-crate tests) accesses it via `crate::grammar`. T4 should make this `pub(crate)`. |
| `struct ParseOptions { max_errors, recover }` | `parse`, `diag`, `ast`                 | constructed in each subcommand’s `run`                                                 |
| `ParseOptions::default`                       | `cst`, `stats` (transitively)          | `parse()` calls `parse_with(source, ParseOptions::default())`; `cst` and `stats` both call `parse()`                  |
| `struct ParseResult { errors, green }`        | `parse`, `cst`, `ast`, `diag`, `stats` | returned by `parse` / `parse_with`; `.errors` and `.green` read by every subcommand    |
| `fn parse(&str) -> ParseResult`               | `cst`, `stats`                         | the no-options entry point                                                             |
| `fn parse_with(&str, ParseOptions) -> ParseResult` | `parse`, `diag`, `ast`            | the options-aware entry point                                                          |
| `fn parse_file(&Path) -> Result<ParseResult, io::Error>` | (none)                      | **Orphan** — CLI reads source via `resolve_inputs` and calls `parse_with`. No caller in the workspace. |

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
| `PrefixDecl`                                                 | `ast`         | reached via `Import::prefix()` (exposed but the `Import` variant is opaque in the CLI output) — **technically uncalled by the CLI** but on the same `Import` family that *is* matched. Flag for T4. |
| `ImportSpecifier`                                            | (none)        | **Orphan from the CLI** — `Import` is matched as an opaque variant; `ImportSpecifier` and the import-specifier list it belongs to are never destructured. |

#### `Stmt` / `Pat` / `Type` families and the entire `expr` module

The `ast` subcommand stops at `Item` and does **not** walk into
circuit bodies, statement lists, expression trees, or type annotations
beyond the type names exposed on the items themselves. Everything
below is *exported* by `compactp_ast` but **not reached by any CLI
subcommand**:

| Family                                                                                                                                  | Coverage status |
| --------------------------------------------------------------------------------------------------------------------------------------- | --------------- |
| `enum Stmt` + `Assert`/`Assign`/`Block`/`Const`/`Expr`/`For`/`If`/`MultiConst`/`Return` newtypes                                        | **Orphan from the CLI.** Reachable transitively via `CircuitDef::body() -> Block::stmts()`, but the CLI never calls `.body()` past `body.is_some()`. |
| `enum Pat` + `IdentPat`/`StructPat`/`StructPatField`/`TuplePat`/`TuplePatElt`                                                           | **Orphan from the CLI.** |
| `enum Type` + `BooleanType`/`BytesType`/`FieldType`/`OpaqueType`/`RecordType`/`TypeRef`/`TupleType`/`UintType`/`UnsignedIntegerType`/`VectorType` + `TypeSize` | **Orphan from the CLI.** `LedgerDecl::ty()`, `TypeDecl::aliased_type()`, `StructField::ty()` etc. return `Option<Type>`, but no CLI codepath inspects the returned `Type`. |
| `mod expr` — `enum Expr` + 22 expression newtypes (`ArrayExpr`, `BinaryExpr`, `BytesExpr`, `CallExpr`, `CastExpr`, `DefaultExpr`, `DiscloseExpr`, `FoldExpr`, `IndexExpr`, `LambdaExpr`, `LiteralExpr`, `MapExpr`, `MemberExpr`, `NameExpr`, `PadExpr`, `ParenExpr`, `SliceExpr`, `SpreadExpr`, `StructExpr`, `StructFieldInit`, `StructUpdate`, `TernaryExpr`, `UnaryExpr`) | **Orphan from the CLI.** No subcommand walks expressions. |
| `Param`, `ParamList`                                                                                                                    | **Orphan from the CLI.** Exposed on `CircuitDef::params()` / `CircuitDecl::params()` / `LambdaExpr::param_list()` but the CLI never calls `params()`. |
| `Block`                                                                                                                                 | **Orphan from the CLI.** Returned by `.body()` accessors; CLI only checks `body.is_some()`. |

#### `support` module

| Public item                                            | Subcommand(s) | Notes                                          |
| ------------------------------------------------------ | ------------- | ---------------------------------------------- |
| `support::child_node`, `support::child_token`, `support::children_nodes` | (none) | **Orphan from the CLI.** Used internally by generated `AstNode` impls inside `compactp_ast` itself (paths without the `compactp_ast::` prefix). Either keep `pub` and document them as a stable extension API, or demote to `pub(crate)` in T4. |

## Orphans (items not exercised by any CLI subcommand)

Summary table consolidating the "Orphan" entries above. These are the
inputs to WS2 Task 4.

| Item                                                | Crate                  | Notes                                                                                                                |
| --------------------------------------------------- | ---------------------- | -------------------------------------------------------------------------------------------------------------------- |
| `SyntaxKind::is_keyword`                            | `compactp_syntax`      | No caller anywhere in the workspace. Demote, delete, or doctest.                                                     |
| `SyntaxKind::is_trivia`                             | `compactp_syntax`      | Used by `compactp_parser` internally; not by CLI. Keep `pub` (parser is a consumer), but no CLI codepath exercises it. |
| `type SyntaxElement`                                | `compactp_syntax`      | Type alias with no callers. Demote or doctest.                                                                       |
| `compactp_parser::grammar` module                   | `compactp_parser`      | `pub mod` but every consumer uses `crate::grammar`. Demote to `pub(crate)`.                                          |
| `parse_file(&Path) -> Result<ParseResult, io::Error>` | `compactp_parser`    | CLI uses `resolve_inputs` + `parse_with`. No callers in the workspace.                                               |
| `ImportSpecifier` + accessors                       | `compactp_ast`         | Exposed but never destructured. `Import` is opaque in `ast` output.                                                  |
| `PrefixDecl` + accessors                            | `compactp_ast`         | Exposed via `Import::prefix()`; CLI does not inspect.                                                                |
| Entire `enum Stmt` family                           | `compactp_ast`         | CLI does not walk circuit bodies. ~9 newtypes + 1 enum + their accessors.                                            |
| Entire `enum Pat` family                            | `compactp_ast`         | CLI does not walk patterns. ~4 newtypes + 1 enum.                                                                    |
| Entire `enum Type` family                           | `compactp_ast`         | CLI calls `Type` accessors that *return* `Type` but never matches on the variants. ~10 newtypes + 1 enum + `TypeSize`. |
| Entire `compactp_ast::expr` module                  | `compactp_ast`         | 22 expression newtypes + the `Expr` enum. CLI never walks expressions.                                               |
| `Param`, `ParamList`                                | `compactp_ast`         | Exposed on every circuit-like accessor but CLI never calls `params()`.                                               |
| `Block`                                             | `compactp_ast`         | Returned by `.body()`; CLI only checks `.is_some()`.                                                                 |
| `support::child_node`, `child_token`, `children_nodes` | `compactp_ast`      | Internal helpers; no external consumer. Demote to `pub(crate)` or doctest.                                           |

### Orphan-count summary

| Crate                  | CLI-orphan items (approx.)                                                                                |
| ---------------------- | --------------------------------------------------------------------------------------------------------- |
| `compactp_lexer`       | 0                                                                                                         |
| `compactp_syntax`      | 3 (`is_keyword`, `is_trivia` from CLI’s POV, `SyntaxElement`)                                             |
| `compactp_diagnostics` | 0 (constructors used by parser, not by CLI — kept)                                                        |
| `compactp_parser`      | 2 (`grammar` mod, `parse_file`)                                                                            |
| `compactp_ast`         | ~50 nodes across `Stmt`, `Pat`, `Type`, `expr`, plus `Block`, `Param`, `ParamList`, `ImportSpecifier`, `PrefixDecl`, and the 3 `support` helpers |

Task 4 will decide, for each orphan, whether to:
1. demote to `pub(crate)`,
2. add a doctest that exercises it from the public API,
3. extend the CLI (most likely for the AST orphans — e.g. an `--depth full` flag on `ast`), or
4. delete outright.
