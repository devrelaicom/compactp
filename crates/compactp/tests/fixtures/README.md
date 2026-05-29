# CLI integration fixtures

Hand-written inputs used exclusively by the CLI integration tests
(`crates/compactp/tests/cli.rs`) and any local demo workflows. The upstream
corpus (486 regular `.compact` source files) lives at `tests/corpus/` (repo
root) and covers valid-Compact
breadth; fixtures here target specific CLI behaviors — recovery output shape,
human-rendering layout, snapshot stability — that need small, reviewable
content.

Each fixture stays under ~40 lines. Changes to any fixture will move the
snapshots; update them intentionally and describe the schema impact in the PR
body.

| Path                                      | Exercises                                       |
| ----------------------------------------- | ----------------------------------------------- |
| `demo/valid.compact`                      | Happy path for every subcommand                 |
| `demo/invalid.compact`                    | Recovery behaviour; diag human/JSON             |
| `demo/outdated.compact`                   | Old pragma sample (`>= 0.19.0`, below current language floor) |
| `declarations/all_declarations.compact`   | Top-level declarations in AST dump              |
| `imports/all_import_forms.compact`        | Import variants in lex/cst/stats snapshots      |
| `recovery/missing_semicolons.compact`     | parse exit 1 + --max-diagnostics test           |
| `recovery/broken_expressions.compact`     | diag human/JSON snapshot and --max-diagnostics  |

Fixtures originate from the clean-room `compactp-whiteroom` reference; they
are small, hand-written (not copied from upstream Compact sources), and carry
no external licensing constraints beyond the repository's MIT license.
