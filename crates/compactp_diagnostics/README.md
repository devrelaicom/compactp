# compactp_diagnostics

Structured diagnostics for the [`compactp`](https://github.com/devrelaicom/compactp)
parser frontend for the Compact language (Midnight Network).

Defines `Diagnostic`, `Severity`, `DiagnosticCode`, and `LabeledSpan` — the types
every parser-side error or warning flows through, and that every downstream
consumer (CLI, library users, IDEs) reads. Serializable to the JSON envelope used
by the `compactp` CLI.

## Status

Beta (`0.x`). APIs may change between minor versions. See the
[compatibility matrix](https://github.com/devrelaicom/compactp#compact-compatibility).

## License

MIT
