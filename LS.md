# Compact Language Surface Reference

## 1. Overview

This document describes the observed Compact language surface using the local upstream reference set in this repository. It is intended as an engineering reference for parser and AST implementation.

This is not a restatement of a single public grammar page. It is a reconciled view based on:

- upstream compiler source in `references/compact/compiler`
- upstream examples and bug corpus in `references/compact/examples`
- upstream e2e and fuzz artifacts in `references/compact/tests-e2e`
- official Compact tree-sitter grammar in `references/compact-tree-sitter`
- formal specification material in `references/compact/specification`

Where sources disagree, this document records the divergence explicitly.

## 2. Source Hierarchy

For parser implementation purposes, use this hierarchy:

1. upstream compiler implementation
2. upstream accepted examples and tests
3. official tree-sitter grammar
4. public grammar and language docs
5. formal specification material

## 2.1 Version Note

Current source mismatch:

- public grammar page: Compact language version `0.21.0`
- upstream compiler source: language version `0.22.0`

This document assumes the parser target is the latest upstream compiler syntax surface.

## 3. Lexical Surface

## 3.1 Whitespace And Trivia

Compact source permits:

- spaces
- tabs
- newlines
- comments

Whitespace and comments are semantically insignificant for parsing in most positions but must be preserved for a lossless frontend.

## 3.2 Comments

Observed comment forms:

- line comments: `// ...`
- block comments: `/* ... */`

Examples use both extensively.

## 3.3 Identifiers

Observed identifier space is richer than the tree-sitter `id` regex suggests.

Evidence:

- tree-sitter currently uses a TypeScript-like simplified identifier rule
- upstream examples include identifiers such as `private$secret_key`, `T$t`, `Prefix$compute`, and module-export aliases containing `$`
- historical ABNF also differs from current observed examples

Practical parser conclusion:

- identifiers must support at least ASCII letters, digits after the first character, `_`, and observed `$`
- additional identifier allowances should be implemented according to upstream lexer behavior, not the current tree-sitter simplification alone

## 3.4 Keywords

Observed keyword families include:

- declaration and module keywords:
  - `pragma`
  - `include`
  - `import`
  - `from`
  - `prefix`
  - `export`
  - `module`
  - `contract`
  - `struct`
  - `enum`
  - `type`
- state and callable keywords:
  - `ledger`
  - `constructor`
  - `circuit`
  - `witness`
  - `pure`
  - `sealed`
- statement and expression keywords:
  - `const`
  - `return`
  - `if`
  - `else`
  - `for`
  - `of`
  - `assert`
  - `as`
  - `map`
  - `fold`
  - `default`
  - `disclose`
  - `pad`
  - `new`
  - `slice`
- builtin type keywords:
  - `Boolean`
  - `Field`
  - `Uint`
  - `Bytes`
  - `Opaque`
  - `Vector`

The upstream compiler also reserves a larger set of future-use words.

## 3.5 Literals

Observed literal classes:

- boolean literals: `true`, `false`
- numeric literals:
  - decimal
  - hex in examples such as `0x31`
  - octal in examples such as `0o4`
  - binary in examples such as `0b11`
- string literals
- version literals in `pragma`

The current tree-sitter grammar simplifies numeric handling relative to examples. Parser implementation should follow upstream compiler behavior rather than tree-sitter’s narrower literal model.

## 3.6 Operators And Punctuation

Observed operators include:

- assignment:
  - `=`
  - `+=`
  - `-=`
- comparison:
  - `==`
  - `!=`
  - `<`
  - `<=`
  - `>`
  - `>=`
- boolean:
  - `!`
  - `&&`
  - `||`
- arithmetic:
  - `+`
  - `-`
  - `*`
- other syntax operators:
  - `? :`
  - `=>`
  - `..`
  - `...`
  - `as`
  - `.`

Observed delimiters:

- `(`
- `)`
- `{`
- `}`
- `[`
- `]`
- `<`
- `>`
- `,`
- `;`
- `:`

## 4. Top-Level Forms

## 4.1 `pragma`

Observed form:

```compact
pragma language_version >= 0.15.0;
```

Purpose:

- language-version constraint

Notes:

- version expressions support comparison and boolean composition in the formal grammar/tree-sitter
- exact accepted pragma names should follow compiler behavior

## 4.2 `include`

Observed documented form:

```compact
include "path";
```

Purpose:

- include another Compact source file

## 4.3 `import`

Observed forms include:

```compact
import CompactStandardLibrary;
import "SomeModule";
import { test as t } from Test;
import { test as t } from Test prefix T$;
import { Maybe as module_maybe } from CompactStandardLibrary;
import { test6a as t6a } from Test6a<Field>;
```

Important:

- the public/simple grammars do not fully capture the richer selective import surface used in real examples
- `from`-based selective imports are real language surface and must be treated as such

## 4.4 `export`

Observed forms:

```compact
export {Maybe}
export { t, T$t };
export ledger value: Field;
export circuit get(): Maybe<Field> { ... }
export struct Point { ... }
export enum Direction { ... }
```

Export may appear as:

- a standalone grouped export form
- a modifier on declarations

## 4.5 `module`

Observed forms:

```compact
module Test {
  export circuit test(v: Field): Field {
    return v;
  }
}

module Test6a<T> {
  export circuit test6a(var6: Bytes<1>): T {
    return var6[0] as T;
  }
}
```

Observed capabilities:

- nested declarations
- generic modules
- imports inside modules
- exports from modules
- module-level state and callable declarations

## 4.6 `ledger`

Observed forms:

```compact
ledger authority: Bytes<32>;
export ledger value: Field;
export ledger counter: Counter;
export ledger contractA: A;
```

Observed modifiers:

- `export`
- `sealed` in grammar/compiler sources

## 4.7 `constructor`

Observed forms:

```compact
constructor(v: Field) {
  ...
}
```

Observed capabilities:

- standard parameter lists
- state initialization
- some examples place contract declarations inside constructors in composability scenarios

## 4.8 `circuit`

Observed forms:

```compact
circuit in_state(s: STATE): Boolean {
  return state == s;
}

export pure circuit foo(x: Field): Field {
  return x;
}
```

Observed roles:

- internal circuits
- exported circuits
- pure circuits
- external/interface circuit declarations ending in `;`

## 4.9 `witness`

Observed forms:

```compact
witness private$secret_key(): Bytes<32>;
witness bob(a: A): Field;
```

Witnesses are declaration-only in observed source surface.

## 4.10 `contract`

Observed forms:

```compact
contract Calculator {
  circuit get_square(x: Field): Field;
  circuit get_cube(x: Field): Field;
}
```

Observed role:

- declare external contract interfaces
- use contract types in ledgers, parameters, returns, maps, vectors, and composability examples

## 4.11 `struct`

Observed forms:

```compact
export struct Point {
  x: Field,
  y: Field
}
```

Observed features:

- exported and non-exported structs
- generic structs in grammar/compiler model
- comma- and semicolon-flavored bodies appear in grammar artifacts

## 4.12 `enum`

Observed form:

```compact
enum STATE { unset, set }
```

Observed features:

- exported and non-exported enums
- enum member access via `EnumName.Member`

## 5. Imports, Exports, And Modules

This is one of the most important divergence areas.

## 5.1 Plain Imports

Examples:

```compact
import CompactStandardLibrary;
import "CompactStandardLibrary";
```

The compiler manual documents plain imports and file/module resolution behavior.

## 5.2 Selective Imports

Examples:

```compact
import { test as t } from Test;
import { var_test2a as vt2a, test2a as t2a } from Test2a;
import { Config as Cfg, Status as St, last_config as lc } from Test9a;
```

Observed features:

- grouped import specifiers
- `as` aliasing
- importing values, ledgers, circuits, structs, and enums

## 5.3 Prefixed Imports

Examples:

```compact
import { test as t } from Test prefix T$;
import { test3b as t3b, test3c as t3c } from Test3b prefix _;
```

Observed behavior:

- `prefix` augments imported symbol names
- prefixed imported names are then exported or consumed directly

## 5.4 Grouped Exports

Examples:

```compact
export {Maybe}
export { t, T$t };
export { Cfg, St, pc, cs, lc, ls };
```

## 5.5 Nested And Generic Modules

Observed:

- generic modules such as `module Test6a<T> { ... }`
- nested modular dependency chains
- re-export patterns inside modules

## 5.6 Divergence Note

The current tree-sitter grammar models a simpler `import` form than the real example corpus. Parser implementation must support the richer example-backed module/import/export surface.

## 6. Type Surface

## 6.1 Primitive And Builtin Types

Observed builtins:

- `Boolean`
- `Field`
- `Uint<...>`
- `Bytes<...>`
- `Opaque<...>`
- `Vector<..., ...>`

## 6.2 Tuple Types

Observed:

- empty tuple `[]`
- tuple types like `[Uint<32>, Uint<32>]`

## 6.3 Named Types

Observed:

- struct names
- enum names
- contract names
- ledger ADT names such as `Counter`, `Set<T>`, `Map<K, V>`, `MerkleTree<N, T>`, `Maybe<T>`

## 6.4 Generic Type References

Observed:

```compact
Maybe<Field>
Map<Field, Field>
Vector<16, Uint<0..255>>
Test6a<Field>
```

## 6.5 Unsized And Range-Based `Uint`

Observed forms:

- fixed-width style:
  - `Uint<8>`
  - `Uint<16>`
  - `Uint<32>`
  - `Uint<64>`
  - larger widths
- range-based:
  - `Uint<0..10>`
  - `Uint<0..255>`
  - generic bound usage such as `Uint<0..N>`

Range-based `Uint` is real language surface from examples and bug corpus.

## 6.6 `Bytes`

Observed:

- `Bytes<0>`
- `Bytes<1>`
- `Bytes<4>`
- `Bytes<32>`
- bytes literals via `Bytes[...]`

## 6.7 `Opaque`

Observed:

- `Opaque<"string">`
- `Opaque<"Uint8Array">`

The public/simple grammar and examples differ on quoting conventions in a few places. The parser should follow upstream accepted syntax.

## 6.8 `Vector`

Observed:

- `Vector<1, Field>`
- `Vector<10, Uint<8>>`
- deeply nested vectors

## 6.9 Contract Types

Observed:

```compact
contract Calculator { ... }
ledger calc: Calculator;
constructor(c: Calculator) { ... }
witness bob(): A;
```

Contract types are first-class in composability examples.

## 7. Patterns And Bindings

## 7.1 Identifier Patterns

Observed:

```compact
const x = 1;
```

## 7.2 Tuple Patterns

Observed in grammar/compiler model and examples:

```compact
const [a, b] = [1, 2];
```

Tuple destructuring exists as language surface.

## 7.3 Struct Patterns

Observed in grammar/compiler surface:

```compact
const { x, y } = point;
const { x: px, y: py } = point;
```

## 7.4 Typed Const Bindings

Observed:

```compact
const a: Field = default<Field>;
const config: Cfg = Cfg { threshold: 100, enabled: true };
```

## 7.5 Multi-Const Forms

Examples show comma-separated multi-const constructs in some corpus files. This is part of the observed surface and should be captured even if later lowered internally.

## 8. Statements

## 8.1 Assignment

Observed:

```compact
value = disclose(v);
calc = c;
```

## 8.2 Compound Assignment

Observed:

```compact
counter += disclose(x as Uint<16>);
counter -= disclose(x as Uint<16>);
```

## 8.3 Expression Statements

Observed:

```compact
numbers.insert(disclose(num));
```

## 8.4 `return`

Observed:

```compact
return v;
return [];
return;
return a, b;
```

Expression-sequence returns appear in examples and compiler artifacts.

## 8.5 `if`

Observed:

```compact
if (condition) stmt
if (condition) stmt else stmt
```

Block and non-block branch forms appear in examples, including negative tests.

## 8.6 `for`

Observed:

```compact
for (const i of 1..10) { ... }
for (const value of [...vec1, ...vec2]) { ... }
```

Observed loop domains:

- numeric ranges
- expressions yielding iterable/vector-like values

## 8.7 `assert`

Observed:

```compact
assert(result == 0, "Result is 0");
```

Grammar artifacts sometimes show a no-parentheses style, but real examples overwhelmingly use call-like parentheses. Parser implementation should support the accepted upstream form.

## 8.8 `const`

Observed:

```compact
const result = t(0) + T$t(0);
const status = St.Active;
const [a, b] = [1, 2];
```

## 8.9 Blocks

Observed:

```compact
{
  const x = 1;
  return x;
}
```

## 9. Expressions

## 9.1 Precedence Overview

Observed precedence, from low to high:

1. conditional `?:`
2. logical OR `||`
3. logical AND `&&`
4. equality `== !=`
5. relational `< <= > >=`
6. cast `as`
7. additive `+ -`
8. multiplicative `*`
9. unary `!`
10. postfix/member/index/call
11. terms/literals

This ordering is supported by tree-sitter and aligns with examples.

## 9.2 Conditional

Observed:

```compact
condition ? some<Field>(value) : none<Field>()
```

## 9.3 Boolean Expressions

Observed:

- `&&`
- `||`
- unary `!`

## 9.4 Equality And Relational Expressions

Observed:

- `==`
- `!=`
- `<`
- `<=`
- `>`
- `>=`

## 9.5 Cast Expressions

Observed heavily:

```compact
x as Uint<16>
var3[0] as T
result3c == [0,0,0] as Bytes<3>
```

## 9.6 Arithmetic

Observed:

- `+`
- `-`
- `*`

## 9.7 Calls

Observed:

```compact
t(0)
public_key(sk)
counter.increment(disclose(amount))
```

## 9.8 Member Access

Observed:

```compact
point.x
Dir.North
calc.get_square(i)
lc.threshold
```

## 9.9 Indexing

Observed:

```compact
var6[0]
bytes[1]
vector[a]
```

## 9.10 Literals

Observed:

- booleans
- numeric literals
- string literals
- tuple/vector-like bracket literals
- bytes literals
- struct literals

## 9.11 Struct Literals

Observed:

```compact
Point { x, y }
Cfg { threshold: 100, enabled: true }
```

Observed field forms:

- shorthand field init
- named field init

Compiler/tree-sitter artifacts also reference update/spread-style struct construction.

## 9.12 Sequence And Bracket Literals

Observed:

```compact
[1, 2, 3]
[1 as Field, 2 as Field]
[]
```

This bracket syntax is overloaded across tuple-like, vector-like, and bytes-cast workflows.

## 9.13 Bytes Literals

Observed explicit bytes constructor surface:

```compact
Bytes[1, 2, 3, 4]
Bytes[]
Bytes[...a, ...b]
```

This surface is not fully captured by the current tree-sitter grammar and must be treated as real language syntax.

## 9.14 Spread-Like Forms

Observed:

```compact
[...var3]
Bytes[...a, ...b]
[...vec1, ...vec2]
```

Observed contexts:

- bytes construction
- vector/list-like construction
- casted bracket expressions

## 9.15 `default<T>`

Observed:

```compact
default<Field>
default<Bytes<32>>
default<Vector<1, Boolean>>
default<Map<Uint<0..255>, Bytes<16>>>
```

## 9.16 `disclose`

Observed:

```compact
disclose(v)
disclose(config)
disclose(amount)
```

Used to move witness/private values into public/stateful operations.

## 9.17 `map`

Observed:

```compact
map((x: Field) => Bytes[x as Uint<8>], fields)
```

Supports:

- named functions
- anonymous functions
- one or more iterable arguments

## 9.18 `fold`

Observed:

```compact
fold((acc, val) => acc + val, 0 as Field, [1, 2, 3])
```

Supports:

- function
- initial accumulator
- one or more iterable inputs

## 9.19 `pad`

Observed:

```compact
pad(32, "lares:tiny:pk:")
```

## 9.20 `slice`

Observed heavily in vector/bytes examples:

```compact
slice<1>(d as Vector<5, Uint<8>>, 1)
slice<2>([...vec1, ...vec2], 1)
```

This is real language surface even though not fully modeled in the current tree-sitter grammar excerpt.

## 9.21 Anonymous Functions

Observed:

```compact
(x: Field) => x
(a, b) => a + b
() => { return default<Field>; }
```

Supports:

- typed and untyped parameters
- expression bodies
- block bodies
- optional return type declaration in grammar artifacts

## 9.22 Enum References

Observed:

```compact
STATE.set
Dir.North
St.Active
```

## 10. Callable Declarations

## 10.1 Circuit Definitions

Observed:

```compact
export circuit test(v: Field): Field { ... }
pure circuit test(): [] { ... }
```

## 10.2 External Circuit Declarations

Observed in contract interfaces:

```compact
circuit get_square(x: Field): Field;
```

## 10.3 Witness Declarations

Observed:

```compact
witness get_secret(): Field;
```

## 11. Composability Surface

Compact supports a substantial contract-composability layer in the example corpus.

Observed capabilities:

- contract interface declarations
- contract-typed ledgers
- contract-typed parameters
- contract-typed witness returns
- contracts in maps and vectors
- member calls on contract values
- contract declarations inside broader composability examples

Representative examples:

- `references/compact/examples/composable/direct`
- `references/compact/examples/composable/cases`
- `references/compact/examples/composable/graph`

## 12. Representative Example Patterns

## 12.1 State Machine Style

Example:

- `references/compact/examples/tiny.compact`

Observed features:

- enum states
- ledger fields
- constructor initialization
- witness access
- public/private boundary via `disclose`

## 12.2 Module And Re-Export Workflows

Example:

- `references/compact/examples/modules/selective_examples.compact`

Observed features:

- selective imports
- aliasing
- prefixed imports
- nested module dependency chains
- exports of circuits, ledgers, structs, enums

## 12.3 Bytes And Vector Operations

Examples:

- `references/compact/examples/bytes/test_basic_bytes.compact`
- `references/compact/examples/vectors/slice_part_one.compact`

Observed features:

- `Bytes[...]`
- spreads
- slices
- indexing
- casts between bytes/vector-related shapes

## 13. Divergences And Open Questions

## 13.1 Public Docs Version vs Compiler Version

Conflict:

- docs grammar page says `0.21.0`
- compiler source encodes `0.22.0`

Parser stance:

- follow latest upstream compiler target

## 13.2 Tree-Sitter Import Grammar vs Real Examples

Conflict:

- tree-sitter grammar models a simpler `import` form
- examples show `import { ... } from ... prefix ...`

Parser stance:

- support the richer example-backed syntax

## 13.3 Tree-Sitter Identifier Rule vs Real Examples

Conflict:

- tree-sitter uses a simplified identifier regex
- examples use `$` in names and aliases

Parser stance:

- follow upstream accepted identifier behavior, not the simplified tree-sitter regex alone

## 13.4 Numeric Literal Surface

Conflict:

- tree-sitter excerpt shows only simple naturals
- examples show hex, octal, and binary literals

Parser stance:

- support the upstream numeric literal surface used in examples

## 13.5 Bytes/Spread/Slice Surface

Conflict:

- tree-sitter excerpt does not model all observed bytes/spread/slice forms
- examples clearly use them

Parser stance:

- treat these as required syntax coverage

## 13.6 `assert` Surface

Conflict:

- some simplified grammar comments show `assert expr str`
- examples use `assert(condition, "message")`

Parser stance:

- prioritize the example/compiler-backed source syntax

## 13.7 `new` And Internal IR Forms

Compiler IR references forms such as:

- `new`
- `tuple-slice`
- `vector-slice`
- `bytes-slice`

Open question:

- some of these may be source syntax
- some may be internal normalized forms

Parser stance:

- implement directly observed source forms first
- document IR-only constructs separately until source syntax is confirmed

## 14. Operator Inventory

- assignment: `=`, `+=`, `-=`
- boolean: `!`, `&&`, `||`
- comparison: `==`, `!=`, `<`, `<=`, `>`, `>=`
- arithmetic: `+`, `-`, `*`
- conditional: `?`, `:`
- range: `..`
- spread: `...`
- cast keyword: `as`
- member access: `.`
- arrow: `=>`

## 15. Keyword Inventory

Observed major keywords:

- `pragma`
- `include`
- `import`
- `from`
- `prefix`
- `export`
- `module`
- `ledger`
- `constructor`
- `circuit`
- `witness`
- `contract`
- `struct`
- `enum`
- `type`
- `const`
- `return`
- `if`
- `else`
- `for`
- `of`
- `assert`
- `pure`
- `sealed`
- `as`
- `map`
- `fold`
- `default`
- `disclose`
- `pad`
- `new`
- `slice`
- `Boolean`
- `Field`
- `Uint`
- `Bytes`
- `Opaque`
- `Vector`
- `true`
- `false`

## 16. Parser Implementation Guidance

For frontend implementation:

- do not assume tree-sitter is the full language
- do not assume the public grammar page is fully current
- treat the upstream compiler plus accepted examples as the primary acceptance boundary
- build tests directly from the real corpus, especially modules, composability, bytes, vectors, and bug repro cases

## 17. Summary

Compact is a richer language surface than the simplified grammar artifacts alone suggest. The true implementation target includes:

- module and selective import/export workflows
- contract composability types and calls
- range-based `Uint`
- bytes/vector/slice/spread forms
- lossless trivia preservation needs
- syntax recovery needs for malformed real-world code

Any serious parser implementation must be built against the full upstream corpus, not just the current tree-sitter grammar or public grammar page.

## 18. Deliberate Deviations From Upstream Acceptance

The following Compact syntax forms are accepted by the upstream compiler
(LFDT-Minokawa/compact at `compactc-v0.31.0`) but deliberately rejected
by compactp. Each entry corresponds to one or more files in
`tests/corpus_known_failures.txt` tagged `intentional-strictness`.

These rejections are not parser bugs. They are choices that trade
upstream input compatibility for cleaner downstream tooling guarantees.
If upstream behavior is ever required for a given form, the right path
is to lift the rejection in the parser and remove the corresponding
entry from `corpus_known_failures.txt`, not to silently accept the
form somewhere else in the pipeline.

### 18.1 Reserved words used as identifiers

Compactp rejects any use of a reserved keyword as an ordinary identifier
(variable name, parameter name, type-size token, struct field, struct
name, enum name, circuit name, ledger name, alias target, expression
operand, etc.). Upstream accepts many of these because its lexer is
permissive about keyword/identifier overlap in contexts where parsing
is still locally unambiguous; downstream tooling (syntax highlighters,
formatters, linters, refactoring engines, language servers) cannot
treat a token as both a keyword and an identifier without either
context-sensitive lexing or per-tool special cases. Compactp rejects
the overloading at parse time so every consumer of compactp's token
stream and AST can rely on keyword tokens being semantic and identifier
tokens being free.

This category also covers a few related shapes that all reduce to the
same root cause: reserved words appearing in positions that the grammar
reserves for identifiers (top-level `new while bob = Boolean;`, type
arguments like `Opaque<delete>` or `Uint<0..default>`, `import { false }`
selective imports, `import { test as as }` alias targets, and
`for (const i of of)` expression operands).

Affected corpus files:

- `reserved/example_6.compact` (reserved word: `const` as local-binding identifier)
- `reserved/example_9.compact` (reserved word: `default` as type-size identifier)
- `reserved/example_10.compact` (reserved word: `delete` as type argument)
- `reserved/example_12.compact` (reserved word: `else` as pragma feature name)
- `reserved/example_13.compact` (reserved word: `enum` as enum name)
- `reserved/example_14.compact` (reserved word: `export` as ledger name)
- `reserved/example_16.compact` (reserved word: `false` as selective-import name)
- `reserved/example_18.compact` (reserved word: `for` as circuit name)
- `reserved/example_20.compact` (reserved word: `if` as loop variable name)
- `reserved/example_21.compact` (reserved word: `import` as struct name)
- `reserved/example_24.compact` (reserved word: `new` as ledger name)
- `reserved/example_26.compact` (reserved word: `return` as parameter type)
- `reserved/example_31.compact` (reserved word: `true` as circuit name)
- `reserved/example_32.compact` (reserved word: `try` as include target)
- `reserved/example_36.compact` (reserved words: `new` / `while` at top level)
- `reserved/example_47.compact` (reserved word: `as` as alias target in `import { test as as }`)
- `reserved/example_48.compact` (reserved word: `of` as expression operand in `for (const i of of)`)

### 18.2 Non-Compact visibility/lifetime modifiers on declarations

Compactp rejects `private`, `protected`, `public`, and `static` as
modifiers on Compact declarations. These keywords are reserved in
Compact but are not part of the declaration grammar; they have no
defined semantics in the language. Upstream accepts some of these
shapes via lenient parsing that then surfaces a later error (or, in
some cases, silently ignores them); compactp refuses them at parse
time because admitting a modifier with no defined semantics into the
AST would force every downstream consumer (the type checker, the IR
generator, formatters, codegen tools) to either invent semantics or
re-reject the same form. Rejecting at parse time keeps the AST honest:
every modifier in the AST is a modifier the language defines.

Affected corpus files:

- `reserved/example_42.compact` (`private` on `export ledger`)
- `reserved/example_43.compact` (`protected` on `circuit`)
- `reserved/example_44.compact` (`public` on `witness`)
- `reserved/example_46.compact` (`static` on `constructor`)

### 18.3 `let` as a statement keyword

Compactp rejects `let` as a local-binding statement keyword. Compact
has no `let`; local bindings inside a circuit body use `const`. The
upstream compiler tolerates `let` in some positions because its lexer
reserves the word but its parser is permissive; the resulting AST has
no place to put a "`let` binding" because the language does not have
one. Compactp rejects the form at parse time rather than silently
treating it as `const` (which would mask user error) or admitting an
ill-defined statement variant into the AST.

Affected corpus files:

- `reserved/example_45.compact` (uses `let` as a local-binding statement keyword)
