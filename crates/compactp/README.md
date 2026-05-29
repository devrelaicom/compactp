# compactp

Command-line parser frontend for the Compact language (Midnight Network).

`compactp` tokenizes, parses, and inspects Compact source, emitting a lossless
CST, a typed AST, rustc-style human diagnostics, or a versioned JSON envelope for
every command. It does **not** compile, type-check, or run Compact — it is a fast,
embeddable syntactic frontend for tooling and editor backends.

## Install

```bash
cargo install compactp
```

## Usage

```bash
compactp parse path/to/program.compact
compactp --format json --pretty diag path/to/program.compact
compactp ast path/to/program.compact
compactp watch parse src/
```

See the [full README](https://github.com/devrelaicom/compactp#readme) for the
command reference, JSON schema, exit codes, and the Compact compatibility matrix.

## License

MIT
