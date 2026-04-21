# compactp

<!-- Full project README lands in Step 8. This stub documents the stable
     exit-code contract and the current subcommand surface so early
     consumers don't have to read the source. -->

## Subcommands

| Command | Description                                                 |
| ------- | ----------------------------------------------------------- |
| `lex`   | Tokenize Compact source and print tokens with spans         |
| `parse` | Parse input and emit diagnostics (structured in JSON)       |
| `cst`   | Dump the full lossless concrete syntax tree                 |
| `ast`   | Dump the typed abstract syntax tree by source-order items   |
| `diag`  | Emit diagnostics only (parse silently unless something fails) |
| `stats` | Report file/token/node/error/recovery counts and parse time |
| `watch` | Re-run any of the above when `.compact` files change        |

## Global flags

- `--format <human|json>` — human-readable or machine-readable output (default: `human`)
- `--pretty` — pretty-print JSON output
- `--color <auto|always|never>` — ANSI color policy for human diagnostics (default: `auto`)
- `--timing` — include timing data in supported outputs
- `--stdin-filename <NAME>` — label for stdin input in diagnostics and JSON envelopes
- `--max-diagnostics <N>` — cap emitted diagnostics per input
- `--max-errors <N>` — limit parser error collection (default: `256`)
- `--no-recover` — disable recovery-oriented parsing

## Exit codes

| Code | Meaning                                    |
| ---- | ------------------------------------------ |
| 0    | Success (no parse errors)                  |
| 1    | Parse errors reported or runtime failure   |
| 2    | I/O error (unreadable file/dir, stdin)     |
| 3    | Usage error (invalid flags, bad arguments) |
| 4    | Internal failure (e.g., watch debouncer)   |
