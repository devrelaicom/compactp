# compactp_parser

Recursive-descent parser for the Compact language (Midnight Network), the core of
the [`compactp`](https://github.com/devrelaicom/compactp) parser frontend.

Produces a lossless concrete syntax tree (every byte recoverable) plus a list of
structured diagnostics. Uses marker-based tree construction over
[`rowan`](https://crates.io/crates/rowan), Pratt-style expression precedence, and
explicit error recovery with `ERROR` nodes. Bounded recursion depth
(`ParseOptions::max_depth`, default 256) guarantees no stack overflow on
adversarial input.

## Example

```rust
use compactp_parser::parse;
use compactp_syntax::{SyntaxKind, SyntaxNode};

let result = parse("");
let root = SyntaxNode::new_root(result.green);
assert_eq!(root.kind(), SyntaxKind::SOURCE_FILE);
assert!(result.errors.is_empty());
```

## Status

Beta (`0.x`). APIs may change between minor versions. See the
[compatibility matrix](https://github.com/devrelaicom/compactp#compact-compatibility).

## License

MIT
