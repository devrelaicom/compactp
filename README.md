# compactp

<!-- Full project README lands in Step 8. This stub documents the stable
     exit-code contract so early consumers don't have to read the source. -->

## Exit codes

| Code | Meaning                                    |
| ---- | ------------------------------------------ |
| 0    | Success (no parse errors)                  |
| 1    | Parse errors reported or runtime failure   |
| 2    | I/O error (unreadable file/dir, stdin)     |
| 3    | Usage error (invalid flags, bad arguments) |
| 4    | Internal failure (e.g., watch debouncer)   |
