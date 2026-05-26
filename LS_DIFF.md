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

- `tests/corpus/` — 486 `.compact` source files (489 path entries via
  `find -name '*.compact'`; the difference is 3 symlinks) copied from
  upstream `examples/`.
- `tests/corpus_known_failures.txt` — 71 manifest entries categorized
  as: 21 grammar-gap, 28 upstream-bug-repro, 22 intentional-strictness.
- `LS.md` §18 — three intentional-strictness deviations documented.

### Grammar-gap themes — WS1 Phase 2 outcome

WS1 Phase 2 closed 18 of the 21 grammar-gap entries identified at
baseline, plus 14 `bugs/pm-*` entries that turned out to share root
causes with the planned features. The manifest dropped from 71 → 39.

Closed themes (with bonus closure counts in parentheses):

- **IIFE arrow-function pattern** `(() => {...})()` — 6 of 7 planned closed; 5 bonus `bugs/pm-*` closed (T1)
- **Spread operator `...` in array/Bytes literals** — 4 of 5 planned closed; 8 bonus `bugs/pm-*` closed (T2)
- **Composable contracts** — 3 of 3 planned closed (T3)
- **Trailing-comma / contract-member terminators** — 1 of 1 planned closed (T4)
- **`fold` expression form** — 1 of 1 planned closed (T5)
- **Type alias declarations** — feature implemented; fixture not closed (T6, see below)
- **Two-word type `Unsigned Integer[N]`** — 1 of 1 planned closed (T7)
- **Vector indexing with expression** — 1 of 1 planned closed; 1 bonus pm closed (T8)
- **Named-argument call form `name = expr`** — feature implemented; fixture not closed (T9, see below)

### Newly-discovered grammar gaps

Three fixtures had a SECOND, distinct grammar gap revealed only after
the first gap was fixed. These remain in `tests/corpus_known_failures.txt`
with updated annotations:

- `assert/example_one.compact` — **compound-assignment expression** `(x += n)` (discovered during T1; T1 fixed the outer IIFE shape)
- `proposal.compact` — **`ledger` keyword used as expression prefix** `ledger.field.write(...)` (discovered during T6; T6 implemented type aliases)
- `wpp/pm_16774.compact` — **parenthesized assignment-as-expression** `(field = expr)` in lambda return body (discovered during T9; T9 implemented named-arg form)

These are candidates for a future WS1 Phase 2.5 or WS2 follow-up plan.

### Latent issues flagged for follow-up

- `expr_bp` progress-failure loop discovered during T2 (when a child
  parse fails silently the loop may produce many empty `EXPR_STMT@N..N`
  nodes; pre-existing, not introduced by Phase 2).
- T5's tolerant-recovery hack for the legacy `fold ... over ...`
  syntax — a future cleanup may prefer moving the fixture
  `errors/noimport.compact` to `tests/corpus/errors/negative/` and
  reverting the parser hack.

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
