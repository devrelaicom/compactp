# LS_DIFF — Language Surface Delta

This document enumerates Compact language surface changes across
pinned reference versions, as recorded in `SOURCE_VERSIONS.md`. It is
the working document for grammar-gap work (WS1 Phase 2 and beyond).

## Baseline: 2026-05-26 — initial pinned reference

This is the first recorded pin in the project. Prior to this commit
the parser was developed against an effectively-equivalent snapshot of
upstream Compact (`tests/corpus/` was already byte-identical to
`LFDT-Minokawa/compact@878045a9`, tag `compactc-v0.31.0`). No
retrospective diff is meaningful; the parser's behavior against this
baseline is captured by:

- `tests/corpus/` — 489 `.compact` files copied from upstream
  `examples/`.
- `tests/corpus_known_failures.txt` — 71 manifest entries categorized
  as: 21 grammar-gap, 28 upstream-bug-repro, 22 intentional-strictness.
- `LS.md` §18 — three intentional-strictness deviations documented.

### Grammar-gap themes at baseline

The 21 grammar-gap entries cluster into a small number of language
features the parser does not yet accept. WS1 Phase 2 will close these.
Counts sum to 21. Cross-references below use corpus-relative paths
(rooted at `tests/corpus/`); the same paths and annotations appear in
`tests/corpus_known_failures.txt`.

#### IIFE arrow-function pattern `(() => {...})()` — 7 entries

An immediately-invoked arrow function used as an expression to compute
a constant or to run setup logic at definition time. The parser does
not yet recognize the `(() => { ... })()` call form.

- `assert/example_one.compact`
- `multiconst/multiconst.compact`
- `return/examples.compact`
- `wpp/constructor_test.compact`
- `wpp/export_circuit.compact`
- `wpp/module_wpp.compact`
- `wpp/pm_16723.compact` (tracked-as pm-16723)

#### Spread operator `...` in array/Bytes literals — 5 entries

Array and `Bytes` literals using the JavaScript-style spread operator
to splice another iterable into the literal. Includes spread inside
struct initializers and spread followed by an `as` cast.

- `bytes/test_basic_bytes.compact`
- `casts/advanced_casts.compact`
- `modules/selective_examples.compact` (spread with `as` cast)
- `types/examples.compact` (spread inside struct init)
- `vectors/spread_part_one.compact`

#### Composable contracts — 3 entries

Forms exercising the composable-contracts feature: `export circuit`
declared without an explicit return type, contracts nested inside
constructors, and the `contract A { ... }` nested-definition form.

- `composable/cases/contract-in-circuit/main.compact` (export circuit
  without return type)
- `composable/cases/contract-in-constructor/main.compact` (nested
  contract in constructor)
- `composable/cases/export-in-definition/main.compact` (nested
  `contract A { ... }` form)

#### One-offs — 6 entries

Each of the following exercises a distinct grammar gap that does not
share a theme with any other manifest entry. Each is its own bucket
of size one.

- `commas/more_commas.compact` — trailing comma after circuit
  declaration.
- `errors/noimport.compact` — `fold` expression form.
- `proposal.compact` — type alias form `type Ledger = { ... }`.
- `std_lib/mint.compact` — two-word type `Unsigned Integer[64]`.
- `vectors/slice_part_one.compact` — vector indexing
  `vector[index - 1]` expression form.
- `wpp/pm_16774.compact` — assignment-as-argument
  `august.insert(1, field = ...)` (tracked-as pm-16774).

## Future refresh procedure

When `SOURCE_VERSIONS.md` is updated to a new pin:

1. Run the WS1 Phase 1 refresh procedure (Task 5 of the plan).
2. Run the corpus test; reconcile the manifest.
3. Re-categorize any new failures via the T8 procedure.
4. Append a new section to this document:

   ```markdown
   ## YYYY-MM-DD — pin updated from <old-sha> to <new-sha>

   - Tree-sitter grammar diff: <summary or path to diff file>
   - Compiler source diff: <summary or path to diff file>
   - Syntactic surface changes: <enumerate>
   - Manifest changes: added N entries, removed M
   ```

5. Update the README compatibility matrix if the supported language
   version changed.
