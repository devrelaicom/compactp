# compactp_lexer

Lexer for the Compact language (Midnight Network), part of the
[`compactp`](https://github.com/devrelaicom/compactp) parser frontend.

Tokenizes UTF-8 source bytes into a `(SyntaxKind, &str)` stream with byte offsets,
suitable for direct consumption by `compactp_parser`. Each Compact keyword has a
dedicated `_KW` `SyntaxKind`; literals and identifiers carry their text. The lexer
never panics on arbitrary input (fuzz-tested).

## Status

Beta (`0.x`). APIs may change between minor versions. See the
[compatibility matrix](https://github.com/devrelaicom/compactp#compact-compatibility).

## License

MIT
