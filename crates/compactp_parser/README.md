# compactp_parser

Recursive-descent parser for the Compact language (Midnight Network), the core of
the [`compactp`](https://github.com/devrelaicom/compactp) parser frontend.

Produces a lossless concrete syntax tree (every byte recoverable) plus a list of
structured diagnostics. Uses marker-based tree construction over
[`rowan`](https://crates.io/crates/rowan), Pratt-style expression precedence, and
explicit error recovery with `ERROR` nodes. Bounded nesting depth
(`ParseOptions::max_depth`, default 256) caps the parser's own recursion and
the depth of the expression, type, statement, block, pattern, module and
version-term nesting it builds, so at the default neither the parser nor a
recursive consumer of the tree overflows a 2 MiB thread stack on adversarial
input. Nested `contract` declarations are the one production not yet capped
([#28](https://github.com/devrelaicom/compactp/issues/28)).
The knob is a stack budget for both — see the bounded-depth guarantee in
[`SECURITY.md`](https://github.com/devrelaicom/compactp/blob/main/SECURITY.md)
before raising it.

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
