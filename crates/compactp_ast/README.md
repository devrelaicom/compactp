# compactp_ast

Typed AST wrappers for the Compact language (Midnight Network), part of the
[`compactp`](https://github.com/devrelaicom/compactp) parser frontend.

Provides zero-cost typed access to the lossless CST produced by `compactp_parser`.
Each AST type is a newtype over a `rowan` `SyntaxNode` exposing typed accessor
methods — no allocation, no re-parsing. Walk a `SourceFile` into its `Item`
variants and navigate from there.

## Status

Beta (`0.x`). APIs may change between minor versions. See the
[compatibility matrix](https://github.com/devrelaicom/compactp#compact-compatibility).

## License

MIT
