# compactp Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a complete Rust parser frontend for the Compact language — lexer, parser, CST, typed AST, diagnostics, and CLI — following the design spec at `docs/superpowers/specs/2026-03-20-compactp-design.md`.

**Architecture:** 6-crate Cargo workspace. logos lexer → marker-based recursive descent + Pratt parser → rowan lossless CST → typed AST wrappers. Diagnostics collected separately. CLI via clap with 7 subcommands including watch mode.

**Tech Stack:** Rust (edition 2024), rowan 0.16, logos 0.16, clap 4.6, serde 1.0, serde_json 1.0, notify-debouncer-full 0.7, drop_bomb 0.1, insta 1.46, expect-test 1.5, criterion 0.8, assert_cmd 2.2

**Reference documents:**
- Design spec: `docs/superpowers/specs/2026-03-20-compactp-design.md`
- Functional spec: `FUNC_SPEC.md`
- Language surface: `LS.md`
- Upstream compiler lexer: `/tmp/compactp/references/compact/compiler/lexer.ss`
- Upstream compiler parser: `/tmp/compactp/references/compact/compiler/parser.ss`
- Tree-sitter grammar: `/tmp/compactp/references/compact-tree-sitter/grammar.js`
- Example corpus: `/tmp/compactp/references/compact/examples/` (489 .compact files)

---

### Task 1: Clone Reference Repositories

**Purpose:** Clone upstream repositories into `/tmp/compactp/references/` for use as read-only reference material during development. These are not modified or committed.

- [ ] **Step 1: Create references directory and clone repos**

```bash
mkdir -p /tmp/compactp/references
git clone --depth=1 git@github.com:LFDT-Minokawa/compact.git /tmp/compactp/references/compact
git clone --depth=1 git@github.com:midnightntwrk/compact-tree-sitter.git /tmp/compactp/references/compact-tree-sitter
git clone --depth=1 git@github.com:midnightntwrk/midnight-ledger.git /tmp/compactp/references/midnight-ledger
git clone --depth=1 git@github.com:midnightntwrk/midnight-zk.git /tmp/compactp/references/midnight-zk
```

- [ ] **Step 2: Verify clones**

```bash
ls /tmp/compactp/references/
# Should list: compact, compact-tree-sitter, midnight-ledger, midnight-zk
```

---

### Task 2: Workspace Scaffolding & .gitignore

**Files:**
- Create: `Cargo.toml` (workspace root)
- Create: `.gitignore`
- Create: `crates/compactp_syntax/Cargo.toml`
- Create: `crates/compactp_syntax/src/lib.rs` (empty placeholder)
- Create: `crates/compactp_lexer/Cargo.toml`
- Create: `crates/compactp_lexer/src/lib.rs` (empty placeholder)
- Create: `crates/compactp_parser/Cargo.toml`
- Create: `crates/compactp_parser/src/lib.rs` (empty placeholder)
- Create: `crates/compactp_ast/Cargo.toml`
- Create: `crates/compactp_ast/src/lib.rs` (empty placeholder)
- Create: `crates/compactp_diagnostics/Cargo.toml`
- Create: `crates/compactp_diagnostics/src/lib.rs` (empty placeholder)
- Create: `crates/compactp/Cargo.toml`
- Create: `crates/compactp/src/main.rs` (empty placeholder)

- [ ] **Step 1: Create workspace root Cargo.toml**

```toml
[workspace]
resolver = "3"
members = [
    "crates/compactp_syntax",
    "crates/compactp_lexer",
    "crates/compactp_parser",
    "crates/compactp_ast",
    "crates/compactp_diagnostics",
    "crates/compactp",
]

[workspace.package]
edition = "2024"
license = "MIT"
version = "0.1.0"
authors = ["Aaron Bassett"]

[workspace.dependencies]
# Internal crates
compactp_syntax = { path = "crates/compactp_syntax" }
compactp_lexer = { path = "crates/compactp_lexer" }
compactp_parser = { path = "crates/compactp_parser" }
compactp_ast = { path = "crates/compactp_ast" }
compactp_diagnostics = { path = "crates/compactp_diagnostics" }

# External dependencies — verify versions with `cargo search` before changing
rowan = "0.16"
logos = "0.16"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
clap = { version = "4.6", features = ["derive"] }
notify-debouncer-full = "0.7"
drop_bomb = "0.1"

# Dev dependencies
insta = "1.46"
expect-test = "1.5"
criterion = "0.8"
assert_cmd = "2.2"
walkdir = "2"
```

- [ ] **Step 2: Create .gitignore**

```
/target
FUNC_SPEC.md
LS.md
docs/superpowers/
.superpowers/
```

- [ ] **Step 3: Create all 6 crate Cargo.toml files and empty src/lib.rs or src/main.rs**

Each crate Cargo.toml should use `package.edition.workspace = true`, `package.license.workspace = true`, `package.version.workspace = true` and reference workspace dependencies. Example for compactp_syntax:

```toml
[package]
name = "compactp_syntax"
edition.workspace = true
license.workspace = true
version.workspace = true

[dependencies]
rowan.workspace = true
```

For compactp_lexer: depends on `compactp_syntax.workspace = true`, `logos.workspace = true`.
For compactp_parser: depends on `compactp_syntax`, `compactp_lexer`, `rowan`, `drop_bomb`. Dev-deps: `expect-test`, `insta`.
For compactp_ast: depends on `compactp_syntax`, `rowan`. Dev-deps: `compactp_parser` (for test helpers), `expect-test`.
For compactp_diagnostics: depends on `compactp_syntax`, `serde`, `serde_json`, `rowan`. Dev-deps: `insta`.
For compactp (binary): depends on all library crates, `clap`, `serde_json`, `notify-debouncer-full`. Dev-deps: `assert_cmd`, `insta`.

Create empty `src/lib.rs` for each library crate, empty `src/main.rs` with `fn main() {}` for the binary crate.

- [ ] **Step 4: Verify workspace builds**

Run: `cargo check --workspace`
Expected: compiles with no errors (empty crates)

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock .gitignore crates/
git commit -m "feat: scaffold Cargo workspace with 6 crates"
```

---

### Task 3: Automated Local Tooling — Lefthook Git Hooks

**Purpose:** Set up lefthook for managing git hooks to enforce code quality checks locally before code reaches CI. This ensures consistent commit messages (Conventional Commits), formatted code, and passing builds before any push.

**Files:**
- Create: `lefthook.yml`

- [ ] **Step 1: Check for lefthook and install if needed**

```bash
# Check if lefthook is installed globally
if command -v lefthook &> /dev/null; then
    echo "lefthook already installed: $(lefthook version)"
else
    echo "Installing lefthook locally..."
    cargo install lefthook --locked
fi
```

- [ ] **Step 2: Create lefthook.yml**

```yaml
commit-msg:
  commands:
    conventional-commit:
      run: |
        MSG=$(cat {1})
        FIRST_LINE=$(head -1 {1})
        # Check conventional commit format
        if ! echo "$FIRST_LINE" | grep -qE '^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)(\(.+\))?: .+'; then
          echo "ERROR: Commit message must follow Conventional Commits format"
          echo "  Format: type(scope): description"
          echo "  Types: feat, fix, docs, style, refactor, perf, test, build, ci, chore, revert"
          exit 1
        fi
        # Check commit body is under 500 characters
        BODY=$(tail -n +3 {1})
        BODY_LEN=${#BODY}
        if [ "$BODY_LEN" -gt 500 ]; then
          echo "ERROR: Commit body must be under 500 characters (currently $BODY_LEN)"
          exit 1
        fi
        # Check commit body is under 10 lines
        BODY_LINES=$(tail -n +3 {1} | wc -l | tr -d ' ')
        if [ "$BODY_LINES" -gt 10 ]; then
          echo "ERROR: Commit body must be under 10 lines (currently $BODY_LINES)"
          exit 1
        fi

pre-commit:
  parallel: true
  commands:
    clippy-staged:
      glob: "*.rs"
      run: cargo clippy --workspace -- -D warnings
    format-check:
      glob: "*.rs"
      run: cargo fmt --all -- --check

pre-push:
  commands:
    lint:
      run: cargo clippy --workspace -- -D warnings
    format:
      run: cargo fmt --all -- --check
    build:
      run: cargo build --workspace
    test:
      run: cargo test --workspace
```

- [ ] **Step 3: Install lefthook hooks**

```bash
lefthook install
```

- [ ] **Step 4: Verify hooks are installed**

```bash
ls -la .git/hooks/
# Should show commit-msg, pre-commit, pre-push managed by lefthook
```

- [ ] **Step 5: Test commit-msg hook**

```bash
# Test invalid commit message (should fail)
echo "bad message" | lefthook run commit-msg

# Test valid commit message (should pass)
echo "feat: test message" | lefthook run commit-msg
```

- [ ] **Step 6: Commit**

```bash
git add lefthook.yml
git commit -m "build: add lefthook git hooks for commit-msg, pre-commit, and pre-push checks"
```

---

### Task 4: CI Code Quality — GitHub Actions

**Purpose:** Set up CI to run the same quality checks as the pre-push hooks on every PR and push to main, plus verify the project builds on all target platforms.

**Files:**
- Create: `.github/workflows/ci.yml`

- [ ] **Step 1: Create CI workflow**

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always
  RUSTFLAGS: "-D warnings"

jobs:
  check:
    name: Quality checks
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy, rustfmt
      - uses: Swatinem/rust-cache@v2
      - name: Check formatting
        run: cargo fmt --all -- --check
      - name: Clippy
        run: cargo clippy --workspace --all-targets -- -D warnings
      - name: Run tests
        run: cargo test --workspace

  build:
    name: Build (${{ matrix.os }})
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - name: Build
        run: cargo build --workspace --release
```

- [ ] **Step 2: Commit**

```bash
git add .github/
git commit -m "ci: add GitHub Actions workflow for quality checks and cross-platform builds"
```

---

### Task 5: Automated Release Management — release-plz & cargo-dist

**Purpose:** Set up automated release management to publish releases on crates.io, Homebrew, and GitHub Releases with correct version bumps and changelog updates using [release-plz](https://github.com/release-plz/release-plz) for release orchestration and [cargo-dist](https://github.com/axodotdev/cargo-dist) for artifact building and distribution.

**Files:**
- Create: `release-plz.toml`
- Create: `dist-workspace.toml` (generated by `cargo dist init`)
- Create: `.github/workflows/release-plz.yml`
- Create: `.github/workflows/release.yml` (generated by `cargo dist init`)

- [ ] **Step 1: Configure release-plz**

Create `release-plz.toml`:

```toml
[workspace]
changelog_update = true
git_tag_enable = true
git_release_enable = true
semver_check = true

[[package]]
name = "compactp"
publish = true

[[package]]
name = "compactp_syntax"
publish = true

[[package]]
name = "compactp_lexer"
publish = true

[[package]]
name = "compactp_parser"
publish = true

[[package]]
name = "compactp_ast"
publish = true

[[package]]
name = "compactp_diagnostics"
publish = true
```

- [ ] **Step 2: Create release-plz GitHub Actions workflow**

Create `.github/workflows/release-plz.yml`:

```yaml
name: Release-plz

permissions:
  pull-requests: write
  contents: write

on:
  push:
    branches:
      - main

jobs:
  release-plz-release:
    name: Release-plz release
    runs-on: ubuntu-latest
    steps:
      - name: Checkout repository
        uses: actions/checkout@v4
        with:
          fetch-depth: 0
      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable
      - name: Run release-plz
        uses: release-plz/action@v0.5
        with:
          command: release
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}

  release-plz-pr:
    name: Release-plz PR
    runs-on: ubuntu-latest
    concurrency:
      group: release-plz-${{ github.ref }}
      cancel-in-progress: false
    steps:
      - name: Checkout repository
        uses: actions/checkout@v4
        with:
          fetch-depth: 0
      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable
      - name: Run release-plz
        uses: release-plz/action@v0.5
        with:
          command: release-pr
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}
```

- [ ] **Step 3: Configure cargo-dist**

```bash
cargo install cargo-dist
cargo dist init --ci=github
```

After `cargo dist init` generates the initial config, update `dist-workspace.toml` to include Homebrew tap configuration and target platforms:

```toml
[dist]
cargo-dist-version = "0.28.4"
ci = "github"
installers = ["shell", "powershell", "homebrew"]
targets = [
    "aarch64-apple-darwin",
    "x86_64-apple-darwin",
    "x86_64-unknown-linux-gnu",
    "x86_64-pc-windows-msvc",
]
install-updater = false

[dist.github-custom-runners]
aarch64-apple-darwin = "macos-14"

[dist.homebrew-tap]
owner = "aaronbassett"
name = "homebrew-tap"
```

Note: `cargo dist init` will also generate `.github/workflows/release.yml`. This workflow is triggered by tag pushes (`v*`) and handles:
- Building release binaries for all target platforms
- Creating GitHub Releases with built artifacts
- Updating the Homebrew formula in `aaronbassett/homebrew-tap`

The release-plz workflow creates the version bump PRs and git tags; when those tags are pushed, cargo-dist's release workflow picks them up and handles distribution.

- [ ] **Step 4: Verify configuration**

```bash
cargo dist plan
```

Expected: shows planned artifacts for all target platforms and installers.

- [ ] **Step 5: Add required repository secrets**

The following secrets must be configured in the GitHub repository settings:
- `CARGO_REGISTRY_TOKEN`: API token for crates.io publishing
- `GITHUB_TOKEN`: automatically provided by GitHub Actions (no setup needed)

The Homebrew tap at `aaronbassett/homebrew-tap` must grant write access to the GitHub Actions workflow (via the default `GITHUB_TOKEN` or a PAT with repo scope if the tap is in a different org).

- [ ] **Step 6: Commit**

```bash
git add release-plz.toml dist-workspace.toml .github/
git commit -m "ci: add automated release management with release-plz and cargo-dist

Publishes to crates.io, GitHub Releases, and Homebrew (aaronbassett/homebrew-tap).
Uses release-plz for version bumps/changelogs and cargo-dist for artifact builds."
```

---

### Task 6: compactp_syntax — SyntaxKind Enum & Language Impl

**Files:**
- Create: `crates/compactp_syntax/src/lib.rs`
- Create: `crates/compactp_syntax/src/syntax_kind.rs`

**Reference:** Design spec Section 4 for the complete SyntaxKind inventory.

- [ ] **Step 1: Write test for SyntaxKind basics**

Add to `crates/compactp_syntax/src/lib.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn syntax_kind_is_u16() {
        // SyntaxKind must be repr(u16) for rowan
        let kind = SyntaxKind::WHITESPACE;
        let _raw: u16 = kind.into();
    }

    #[test]
    fn language_impl_exists() {
        // CompactLanguage must implement rowan::Language
        let kind = CompactLanguage::kind_from_raw(rowan::SyntaxKind(0));
        assert_eq!(kind, SyntaxKind::WHITESPACE);
    }

    #[test]
    fn syntax_kind_is_trivia() {
        assert!(SyntaxKind::WHITESPACE.is_trivia());
        assert!(SyntaxKind::LINE_COMMENT.is_trivia());
        assert!(SyntaxKind::BLOCK_COMMENT.is_trivia());
        assert!(!SyntaxKind::IDENT.is_trivia());
        assert!(!SyntaxKind::CIRCUIT_KW.is_trivia());
    }

    #[test]
    fn syntax_kind_is_keyword() {
        assert!(SyntaxKind::CIRCUIT_KW.is_keyword());
        assert!(SyntaxKind::BOOLEAN_KW.is_keyword());
        assert!(SyntaxKind::TRUE_KW.is_keyword());
        assert!(!SyntaxKind::IDENT.is_keyword());
        assert!(!SyntaxKind::PLUS.is_keyword());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p compactp_syntax`
Expected: compilation errors (types don't exist yet)

- [ ] **Step 3: Implement SyntaxKind enum**

Create `crates/compactp_syntax/src/syntax_kind.rs` with the complete `#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)] #[repr(u16)]` enum. Include ALL variants from design spec Section 4:

**Tokens (~55 variants):** WHITESPACE, LINE_COMMENT, BLOCK_COMMENT, INT_LIT, HEX_LIT, OCT_LIT, BIN_LIT, STRING_LIT, VERSION_LIT, TRUE_KW, FALSE_KW, all *_KW keywords, all operators (EQ through DOT_DOT_DOT), all delimiters, IDENT, ERROR, EOF.

**Nodes (~80 variants):** SOURCE_FILE, PRAGMA, INCLUDE, IMPORT, IMPORT_SPECIFIER, IMPORT_SPECIFIER_LIST, EXPORT_LIST, MODULE_DEF, LEDGER_DECL, CONSTRUCTOR_DEF, CIRCUIT_DEF, CIRCUIT_DECL, WITNESS_DECL, CONTRACT_DECL, CONTRACT_CIRCUIT, STRUCT_DEF, STRUCT_FIELD, ENUM_DEF, ENUM_VARIANT, all type nodes, all pattern nodes, all statement nodes, all expression nodes, all version expression nodes, ERROR (node).

Note: ERROR appears as both a token kind (from the lexer for unrecognized characters) and conceptually as a node kind (wrapping malformed syntax). Use a single `ERROR` variant — rowan handles the token-vs-node distinction via the tree structure.

Add helper methods:
- `is_trivia(&self) -> bool` — true for WHITESPACE, LINE_COMMENT, BLOCK_COMMENT
- `is_keyword(&self) -> bool` — true for all *_KW variants
- `From<SyntaxKind> for rowan::SyntaxKind` and `From<u16> for SyntaxKind`

- [ ] **Step 4: Implement CompactLanguage and type aliases**

In `crates/compactp_syntax/src/lib.rs`:

```rust
mod syntax_kind;
pub use syntax_kind::SyntaxKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CompactLanguage {}

impl rowan::Language for CompactLanguage {
    type Kind = SyntaxKind;

    fn kind_from_raw(raw: rowan::SyntaxKind) -> SyntaxKind {
        SyntaxKind::from(raw.0)
    }

    fn kind_to_raw(kind: SyntaxKind) -> rowan::SyntaxKind {
        rowan::SyntaxKind(kind.into())
    }
}

pub type SyntaxNode = rowan::SyntaxNode<CompactLanguage>;
pub type SyntaxToken = rowan::SyntaxToken<CompactLanguage>;
pub type SyntaxElement = rowan::SyntaxElement<CompactLanguage>;
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p compactp_syntax`
Expected: all 4 tests pass

- [ ] **Step 6: Commit**

```bash
git add crates/compactp_syntax/
git commit -m "feat(syntax): add SyntaxKind enum with all token and node variants"
```

---

### Task 7: compactp_lexer — Logos Tokenizer

**Files:**
- Create: `crates/compactp_lexer/src/lib.rs`

**Reference:** Design spec Section 4.1 for token kinds, LS.md Section 3 for lexical surface, upstream `/tmp/compactp/references/compact/compiler/lexer.ss` for exact behavior.

- [ ] **Step 1: Write lexer snapshot tests**

Create tests in `crates/compactp_lexer/src/lib.rs` (or a tests module). Use `expect-test` for inline snapshots. Tests should cover:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use expect_test::{expect, Expect};

    fn check(input: &str, expected: Expect) {
        let tokens: Vec<_> = lex(input)
            .iter()
            .map(|(kind, text)| format!("{kind:?} {:?}", text))
            .collect();
        expected.assert_eq(&tokens.join("\n"));
    }

    #[test]
    fn lex_whitespace() {
        check("  \n\t", expect![[r#"WHITESPACE "  \n\t""#]]);
    }

    #[test]
    fn lex_keywords() {
        check("circuit pure export", expect![[r#"
            CIRCUIT_KW "circuit"
            WHITESPACE " "
            PURE_KW "pure"
            WHITESPACE " "
            EXPORT_KW "export""#]]);
    }

    #[test]
    fn lex_identifier_with_dollar() {
        check("private$secret_key", expect![[r#"IDENT "private$secret_key""#]]);
    }

    #[test]
    fn lex_numeric_literals() {
        check("42 0x1F 0o77 0b1010", expect![[r#"
            INT_LIT "42"
            WHITESPACE " "
            HEX_LIT "0x1F"
            WHITESPACE " "
            OCT_LIT "0o77"
            WHITESPACE " "
            BIN_LIT "0b1010""#]]);
    }

    #[test]
    fn lex_operators() {
        check("== != <= >= && || += -= =>", expect![[r#"
            EQ_EQ "=="
            WHITESPACE " "
            BANG_EQ "!="
            WHITESPACE " "
            LT_EQ "<="
            WHITESPACE " "
            GT_EQ ">="
            WHITESPACE " "
            AMP_AMP "&&"
            WHITESPACE " "
            PIPE_PIPE "||"
            WHITESPACE " "
            PLUS_EQ "+="
            WHITESPACE " "
            MINUS_EQ "-="
            WHITESPACE " "
            FAT_ARROW "=>""#]]);
    }

    #[test]
    fn lex_dots() {
        check(". .. ...", expect![[r#"
            DOT "."
            WHITESPACE " "
            DOT_DOT ".."
            WHITESPACE " "
            DOT_DOT_DOT "...""#]]);
    }

    #[test]
    fn lex_string() {
        check(r#""hello world""#, expect![[r#"STRING_LIT "\"hello world\"""#]]);
    }

    #[test]
    fn lex_line_comment() {
        check("// a comment\ncode", expect![[r#"
            LINE_COMMENT "// a comment"
            WHITESPACE "\n"
            IDENT "code""#]]);
    }

    #[test]
    fn lex_block_comment() {
        check("/* block */", expect![[r#"BLOCK_COMMENT "/* block */""#]]);
    }

    #[test]
    fn lex_version_literal() {
        check("0.15.0", expect![[r#"VERSION_LIT "0.15.0""#]]);
    }

    #[test]
    fn lex_boolean_keywords() {
        check("true false", expect![[r#"
            TRUE_KW "true"
            WHITESPACE " "
            FALSE_KW "false""#]]);
    }

    #[test]
    fn lex_builtin_type_keywords() {
        check("Boolean Field Uint Bytes Opaque Vector", expect![[r#"
            BOOLEAN_KW "Boolean"
            WHITESPACE " "
            FIELD_KW "Field"
            WHITESPACE " "
            UINT_KW "Uint"
            WHITESPACE " "
            BYTES_KW "Bytes"
            WHITESPACE " "
            OPAQUE_KW "Opaque"
            WHITESPACE " "
            VECTOR_KW "Vector""#]]);
    }

    #[test]
    fn lex_delimiters() {
        check("(){}<>[],:;#", expect![[r#"
            L_PAREN "("
            R_PAREN ")"
            L_BRACE "{"
            R_BRACE "}"
            LT "<"
            GT ">"
            L_BRACKET "["
            R_BRACKET "]"
            COMMA ","
            COLON ":"
            SEMICOLON ";"
            HASH "#""#]]);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p compactp_lexer`
Expected: compilation errors (lex function doesn't exist)

- [ ] **Step 3: Implement the logos lexer**

In `crates/compactp_lexer/src/lib.rs`, implement:

1. A `#[derive(Logos)]` enum `LogosToken` with patterns for all token kinds. Key patterns:
   - Identifiers: `[a-zA-Z_$][a-zA-Z0-9_$]*` (note `$` per upstream compiler)
   - After lexing, check if identifier text matches a keyword and reclassify
   - Numeric literals: `[1-9][0-9]*` and `0` for INT_LIT, `0[xX][0-9a-fA-F]+` for HEX_LIT, `0[oO][0-7]+` for OCT_LIT, `0[bB][01]+` for BIN_LIT
   - Version literals: `[0-9]+\.[0-9]+(\.[0-9]+)?` — must be checked carefully to not conflict with integer + dot + integer sequences. Version literals only appear after pragma identifiers; the lexer should lex them as sequences and let the parser handle version assembly, OR use a mode/context. **Simplest approach:** lex `0.15.0` as INT_LIT DOT INT_LIT DOT INT_LIT and let the parser assemble VERSION_LIT nodes. This matches how the upstream Scheme lexer handles it (it returns a version token only when it sees the `N.N.N` pattern). If logos can handle the pattern unambiguously, prefer a single VERSION_LIT token.
   - String literals: `"[^"]*"` (including escaped chars) and `'[^']*'`
   - Multi-char operators: `==`, `!=`, `<=`, `>=`, `&&`, `||`, `+=`, `-=`, `=>`, `..`, `...`
   - Single-char operators/delimiters: all the rest
   - Comments: `//[^\n]*` for line comments, `/* ... */` for block comments (note: upstream does NOT allow nested block comments per lexer.ss)
   - Whitespace: `[\s]+` (spaces, tabs, newlines grouped together)

2. A public `fn lex(source: &str) -> Vec<(SyntaxKind, &str)>` that runs the logos lexer and returns token pairs.

Important implementation note: logos may struggle with some patterns (version literals vs integers+dots, multi-char operators). If logos can't handle the disambiguation, use a two-pass approach: logos for the base tokens, then a post-pass to merge/reclassify (e.g., merge `.` `.` into `..`, or merge `INT DOT INT DOT INT` into VERSION_LIT in pragma context). Keep the lexer simple and correct over clever.

- [ ] **Step 4: Run tests and iterate**

Run: `cargo test -p compactp_lexer`
Expected: all tests pass. Update expect-test snapshots with `UPDATE_EXPECT=1 cargo test -p compactp_lexer` if needed after verifying output is correct.

- [ ] **Step 5: Add edge case tests**

Test: empty input, single characters, unterminated strings (should produce ERROR token), unterminated block comments, reserved future keywords, zero literal (`0`), identifier that starts with keyword prefix (e.g., `forked` should be IDENT not FOR_KW + IDENT).

- [ ] **Step 6: Run all tests**

Run: `cargo test -p compactp_lexer`
Expected: all tests pass

- [ ] **Step 7: Commit**

```bash
git add crates/compactp_lexer/
git commit -m "feat(lexer): implement logos-based tokenizer with full token coverage"
```

---

### Task 8: Parser Infrastructure — Events, Markers, Parser, Sink

**Files:**
- Create: `crates/compactp_parser/src/lib.rs`
- Create: `crates/compactp_parser/src/event.rs`
- Create: `crates/compactp_parser/src/marker.rs`
- Create: `crates/compactp_parser/src/parser.rs`
- Create: `crates/compactp_parser/src/sink.rs`
- Create: `crates/compactp_parser/src/grammar/mod.rs`

**Reference:** Design spec Section 3.2 for core types.

- [ ] **Step 1: Write infrastructure tests**

Test that the marker system works correctly:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use compactp_syntax::SyntaxKind::*;

    #[test]
    fn parse_single_token() {
        // Parsing a single identifier should produce a SOURCE_FILE with one IDENT token
        let tokens = vec![(IDENT, "x")];
        let (green, errors) = parse_tokens(tokens);
        let node = SyntaxNode::new_root(green);
        assert_eq!(node.kind(), SOURCE_FILE);
        assert!(errors.is_empty());
    }

    #[test]
    fn marker_complete_creates_node() {
        // A completed marker should create a node wrapping its contents
        let source = "circuit foo(): Field {}";
        let result = parse(source);
        let root = SyntaxNode::new_root(result.green);
        // Should have a CIRCUIT_DEF child
        assert!(root.children().any(|c| c.kind() == CIRCUIT_DEF));
    }
}
```

- [ ] **Step 2: Implement Event enum**

In `crates/compactp_parser/src/event.rs`:

```rust
use compactp_syntax::SyntaxKind;

#[derive(Debug)]
pub(crate) enum Event {
    StartNode {
        kind: SyntaxKind,
        forward_parent: Option<u32>,
    },
    Token {
        kind: SyntaxKind,
        n_raw_tokens: u8,
    },
    FinishNode,
    Error {
        message: String,
    },
    Placeholder,
}
```

- [ ] **Step 3: Implement Marker and CompletedMarker**

In `crates/compactp_parser/src/marker.rs`:

```rust
use drop_bomb::DropBomb;
use crate::event::Event;
use crate::parser::Parser;
use compactp_syntax::SyntaxKind;

pub(crate) struct Marker {
    pos: u32,
    bomb: DropBomb,
}

impl Marker {
    pub(crate) fn new(pos: u32) -> Self {
        Self {
            pos,
            bomb: DropBomb::new("Marker must be either completed or abandoned"),
        }
    }

    pub(crate) fn complete(mut self, p: &mut Parser, kind: SyntaxKind) -> CompletedMarker {
        self.bomb.defuse();
        match &mut p.events[self.pos as usize] {
            Event::StartNode { kind: slot, .. } => *slot = kind,
            _ => unreachable!(),
        }
        p.push_event(Event::FinishNode);
        CompletedMarker { pos: self.pos }
    }

    pub(crate) fn abandon(mut self, p: &mut Parser) {
        self.bomb.defuse();
        if self.pos as usize == p.events.len() - 1 {
            match p.events.pop() {
                Some(Event::StartNode { kind, forward_parent: None }) => {
                    assert_eq!(kind, SyntaxKind::PLACEHOLDER_FOR_ABANDON);
                    // Event removed, nothing to do
                }
                _ => unreachable!(),
            }
        } else {
            p.events[self.pos as usize] = Event::Placeholder;
        }
    }
}

pub(crate) struct CompletedMarker {
    pos: u32,
}

impl CompletedMarker {
    pub(crate) fn precede(self, p: &mut Parser) -> Marker {
        let new_pos = p.start();
        match &mut p.events[new_pos.pos as usize] {
            Event::StartNode { forward_parent, .. } => {
                *forward_parent = Some(self.pos);
            }
            _ => unreachable!(),
        }
        new_pos
    }
}
```

Note: The abandon implementation above is simplified. In practice, use a tombstone/placeholder sentinel `SyntaxKind` value for the initial StartNode kind, and replace it on complete. See rust-analyzer's parser for the canonical approach. Adjust as needed during implementation.

- [ ] **Step 4: Implement Parser struct**

In `crates/compactp_parser/src/parser.rs`:

```rust
use compactp_syntax::SyntaxKind;
use crate::event::Event;
use crate::marker::Marker;

pub(crate) struct Parser<'src> {
    tokens: Vec<(SyntaxKind, &'src str)>,
    pos: usize,
    pub(crate) events: Vec<Event>,
    expected: Vec<SyntaxKind>,
    recover: bool,
    max_errors: usize,
    error_count: usize,
}

impl<'src> Parser<'src> {
    pub(crate) fn new(tokens: Vec<(SyntaxKind, &'src str)>) -> Self {
        Self {
            tokens, pos: 0, events: Vec::new(), expected: Vec::new(),
            recover: true, max_errors: 256, error_count: 0,
        }
    }

    pub(crate) fn set_options(&mut self, opts: &crate::ParseOptions) {
        self.recover = opts.recover;
        self.max_errors = opts.max_errors;
    }

    /// Peek at the current non-trivia token kind
    pub(crate) fn current(&self) -> SyntaxKind {
        self.nth(0)
    }

    /// Lookahead n non-trivia tokens
    pub(crate) fn nth(&self, n: usize) -> SyntaxKind {
        let mut i = self.pos;
        let mut non_trivia = 0;
        while i < self.tokens.len() {
            let kind = self.tokens[i].0;
            if !kind.is_trivia() {
                if non_trivia == n {
                    return kind;
                }
                non_trivia += 1;
            }
            i += 1;
        }
        SyntaxKind::EOF
    }

    /// Check if current non-trivia token matches
    pub(crate) fn at(&self, kind: SyntaxKind) -> bool {
        self.current() == kind
    }

    /// Consume current token if it matches, return true. Otherwise false.
    pub(crate) fn eat(&mut self, kind: SyntaxKind) -> bool {
        if self.at(kind) {
            self.bump(kind);
            true
        } else {
            false
        }
    }

    /// Consume current token or emit error
    pub(crate) fn expect(&mut self, kind: SyntaxKind) {
        if !self.eat(kind) {
            self.error(format!("expected {:?}", kind));
        }
    }

    /// Unconditionally consume current token (eating leading trivia first)
    pub(crate) fn bump(&mut self, kind: SyntaxKind) {
        self.eat_trivia();
        assert_eq!(self.tokens[self.pos].0, kind);
        self.push_event(Event::Token { kind, n_raw_tokens: 1 });
        self.pos += 1;
    }

    /// Consume any token regardless of kind
    pub(crate) fn bump_any(&mut self) {
        self.eat_trivia();
        let kind = self.tokens[self.pos].0;
        self.push_event(Event::Token { kind, n_raw_tokens: 1 });
        self.pos += 1;
    }

    /// Open a new marker
    pub(crate) fn start(&mut self) -> Marker {
        let pos = self.events.len() as u32;
        self.push_event(Event::StartNode {
            kind: SyntaxKind::ERROR, // placeholder, overwritten by complete()
            forward_parent: None,
        });
        Marker::new(pos)
    }

    /// Emit an error
    pub(crate) fn error(&mut self, message: impl Into<String>) {
        self.push_event(Event::Error { message: message.into() });
    }

    pub(crate) fn push_event(&mut self, event: Event) {
        self.events.push(event);
    }

    fn eat_trivia(&mut self) {
        while self.pos < self.tokens.len() && self.tokens[self.pos].0.is_trivia() {
            let kind = self.tokens[self.pos].0;
            self.push_event(Event::Token { kind, n_raw_tokens: 1 });
            self.pos += 1;
        }
    }

    pub(crate) fn at_end(&self) -> bool {
        self.current() == SyntaxKind::EOF
    }
}
```

- [ ] **Step 5: Implement Sink (event → GreenNode conversion)**

In `crates/compactp_parser/src/sink.rs`:

The sink walks the event list, resolves forward_parent chains, and calls `rowan::GreenNodeBuilder` to build the tree. This is the trickiest infrastructure piece.

Key logic:
1. Walk events in order
2. For each `StartNode`, check if it has a `forward_parent`. If so, walk the forward_parent chain to find all ancestors, then open them in reverse order.
3. For each `Token`, use the raw token text from the source to feed `GreenNodeBuilder::token()`.
4. For each `FinishNode`, call `GreenNodeBuilder::finish_node()`.
5. Skip `Placeholder` events (abandoned markers).
6. Collect `Error` events into a diagnostics vec.

```rust
use rowan::GreenNode;
use compactp_syntax::{SyntaxKind, CompactLanguage};
use crate::event::Event;

pub(crate) struct Sink<'src> {
    events: Vec<Event>,
    tokens: Vec<(SyntaxKind, &'src str)>,
    token_pos: usize,
    builder: rowan::GreenNodeBuilder<'static>,
    diagnostics: Vec<compactp_diagnostics::Diagnostic>,
}

impl<'src> Sink<'src> {
    pub(crate) fn new(events: Vec<Event>, tokens: Vec<(SyntaxKind, &'src str)>) -> Self {
        Self {
            events,
            tokens,
            token_pos: 0,
            builder: rowan::GreenNodeBuilder::new(),
            diagnostics: Vec::new(),
        }
    }

    pub(crate) fn finish(mut self) -> (GreenNode, Vec<compactp_diagnostics::Diagnostic>) {
        // Process events and resolve forward parents
        // ... (implement forward parent resolution)
        // Convert Error events into Diagnostic structs with proper spans and codes
        (self.builder.finish(), self.diagnostics)
    }
}
```

The forward_parent resolution algorithm: for each event index, if it's a `StartNode` with `forward_parent = Some(parent_idx)`, follow the chain to collect all forward parents, then process them in reverse (outermost first). See rust-analyzer's `sink.rs` for the reference implementation.

- [ ] **Step 6: Wire up public API**

In `crates/compactp_parser/src/lib.rs`:

```rust
mod event;
mod marker;
mod parser;
mod sink;
pub mod grammar;

use compactp_syntax::{SyntaxKind, SyntaxNode};
use rowan::GreenNode;

pub struct ParseResult {
    pub green: GreenNode,
    pub diagnostics: Vec<compactp_diagnostics::Diagnostic>,
}

pub fn parse(source: &str) -> ParseResult {
    parse_with(source, ParseOptions::default())
}

pub fn parse_with(source: &str, opts: ParseOptions) -> ParseResult {
    let tokens = compactp_lexer::lex(source);
    let mut p = parser::Parser::new(tokens.clone());
    p.set_options(&opts);

    grammar::source_file(&mut p);

    let events = p.events;
    let (green, diagnostics) = sink::Sink::new(events, tokens).finish();

    ParseResult { green, diagnostics }
}
```

- [ ] **Step 7: Implement minimal grammar::source_file**

In `crates/compactp_parser/src/grammar/mod.rs`:

```rust
use crate::parser::Parser;
use compactp_syntax::SyntaxKind::*;

pub(crate) fn source_file(p: &mut Parser) {
    let m = p.start();
    // For now, just consume all tokens
    while !p.at_end() {
        p.bump_any();
    }
    m.complete(p, SOURCE_FILE);
}
```

- [ ] **Step 8: Run tests**

Run: `cargo test -p compactp_parser`
Expected: basic infrastructure tests pass

- [ ] **Step 9: Commit**

```bash
git add crates/compactp_parser/
git commit -m "feat(parser): add marker-based event pipeline infrastructure"
```

---

### Task 9: Grammar — Top-Level Declarations

**Files:**
- Modify: `crates/compactp_parser/src/grammar/mod.rs`
- Create: `crates/compactp_parser/src/grammar/declarations.rs`

**Reference:** Tree-sitter grammar.js lines 66-353 for all top-level forms. LS.md Sections 4.1-4.12.

- [ ] **Step 1: Write CST snapshot tests for each declaration form**

Create tests for: pragma, include, ledger, constructor, circuit (def + decl), witness, contract, struct, enum. Use `expect-test` inline snapshots showing the expected CST debug output. Example:

```rust
#[test]
fn parse_pragma() {
    check("pragma language_version >= 0.15.0;", expect![[r#"
        SOURCE_FILE
          PRAGMA
            PRAGMA_KW "pragma"
            WHITESPACE " "
            IDENT "language_version"
            WHITESPACE " "
            ..."#]]);
}

#[test]
fn parse_ledger() {
    check("ledger count: Field;", expect![[r#"
        SOURCE_FILE
          LEDGER_DECL
            LEDGER_KW "ledger"
            WHITESPACE " "
            IDENT "count"
            COLON ":"
            WHITESPACE " "
            FIELD_TYPE
              FIELD_KW "Field"
            SEMICOLON ";""#]]);
}
```

Write tests for ALL declaration forms listed in the design spec Section 4.2 top-level nodes.

Include at least one **recovery test** per declaration form — e.g., a circuit missing its closing brace, a ledger missing its semicolon, a struct with a malformed field. Verify: no panic, ERROR node present, diagnostic emitted, surrounding declarations intact. (Constitution Principle VII: Error Recovery Is a Feature.)

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p compactp_parser`
Expected: CST output doesn't match expected shape

- [ ] **Step 3: Implement declaration parsing**

In `crates/compactp_parser/src/grammar/declarations.rs`:

Implement functions for each declaration form. The dispatcher in `source_file` should look at the current token to decide which declaration to parse:

```rust
pub(crate) fn source_file(p: &mut Parser) {
    let m = p.start();
    while !p.at_end() {
        declaration(p);
    }
    m.complete(p, SOURCE_FILE);
}

fn declaration(p: &mut Parser) {
    match p.current() {
        PRAGMA_KW => pragma(p),
        INCLUDE_KW => include(p),
        EXPORT_KW => export_or_exported_decl(p),
        LEDGER_KW => ledger(p),
        SEALED_KW => ledger(p),  // sealed ledger
        CONSTRUCTOR_KW => constructor(p),
        CIRCUIT_KW => circuit(p),
        WITNESS_KW => witness(p),
        CONTRACT_KW => contract(p),
        STRUCT_KW => struct_def(p),
        ENUM_KW => enum_def(p),
        MODULE_KW => module_def(p),
        PURE_KW => circuit(p),   // pure circuit
        IMPORT_KW => import(p),
        _ => {
            // error recovery: skip token
            let m = p.start();
            p.error(format!("expected declaration, found {:?}", p.current()));
            p.bump_any();
            m.complete(p, ERROR);
        }
    }
}
```

Implement each declaration function following the grammar from tree-sitter grammar.js. Each function should:
1. Open a marker with `p.start()`
2. Consume expected tokens with `p.expect()` or `p.bump()`
3. Call sub-parsers for types, parameter lists, blocks, etc.
4. Complete the marker with the appropriate SyntaxKind

- [ ] **Step 4: Run tests and iterate**

Run: `cargo test -p compactp_parser`
Expected: declaration tests pass

- [ ] **Step 5: Commit**

```bash
git add crates/compactp_parser/
git commit -m "feat(parser): implement top-level declaration parsing"
```

---

### Task 10: Grammar — Imports, Exports, Modules

**Files:**
- Create: `crates/compactp_parser/src/grammar/imports.rs`
- Modify: `crates/compactp_parser/src/grammar/mod.rs`

**Reference:** LS.md Sections 5.1-5.6 for the full import/export/module surface. Tree-sitter grammar.js lines 144-214.

This is one of the most divergence-prone areas — the tree-sitter grammar models simpler imports than the real corpus uses.

- [ ] **Step 1: Write tests covering all import/export forms**

Test all forms from LS.md:
- `import CompactStandardLibrary;`
- `import "SomeModule";`
- `import { test as t } from Test;`
- `import { test as t } from Test prefix T$;`
- `import { test6a as t6a } from Test6a<Field>;`
- `export {Maybe}`
- `export { t, T$t };`
- `module Test { ... }`
- `module Test6a<T> { ... }`

Include recovery tests: malformed import specifiers, missing `from` keyword, unclosed `{` in export list.

- [ ] **Step 2: Implement import/export/module parsing**

Key forms:
- Plain import: `import` name `;`
- Selective import: `import` `{` specifiers `}` `from` name generic-args? prefix? `;`
- Import specifier: `id` (`as` `id`)?
- Export list: `export` `{` id-list `}` `;`?
- Export modifier on declarations: handled in declarations.rs dispatcher
- Module def: `export`? `module` name generic-params? `{` declarations `}`

- [ ] **Step 3: Run tests**

Run: `cargo test -p compactp_parser`
Expected: all import/export/module tests pass

- [ ] **Step 4: Commit**

```bash
git add crates/compactp_parser/
git commit -m "feat(parser): implement import/export/module parsing with full selective import support"
```

---

### Task 11: Grammar — Types

**Files:**
- Create: `crates/compactp_parser/src/grammar/types.rs`
- Modify: `crates/compactp_parser/src/grammar/mod.rs`

**Reference:** LS.md Section 6, tree-sitter grammar.js lines 376-408, design spec Section 4.2 type nodes.

- [ ] **Step 1: Write type parsing tests**

Cover all type forms:
- `Boolean`, `Field` (keyword types)
- `Uint<8>`, `Uint<0..255>`, `Uint<0..N>` (range-based)
- `Bytes<32>`
- `Opaque<"string">`
- `Vector<10, Uint<8>>`
- `[Uint<32>, Uint<32>]` (tuple type)
- `[]` (empty tuple)
- `Maybe<Field>` (named generic type)
- `Map<Field, Field>` (multi-arg generic)

Include recovery tests: unclosed `<` in generics, missing `,` between type arguments, malformed `Uint<..>` ranges.

- [ ] **Step 2: Implement type parsing**

```rust
pub(crate) fn type_(p: &mut Parser) {
    match p.current() {
        BOOLEAN_KW => { let m = p.start(); p.bump(BOOLEAN_KW); m.complete(p, BOOLEAN_TYPE); }
        FIELD_KW => { let m = p.start(); p.bump(FIELD_KW); m.complete(p, FIELD_TYPE); }
        UINT_KW => uint_type(p),
        BYTES_KW => bytes_type(p),
        OPAQUE_KW => opaque_type(p),
        VECTOR_KW => vector_type(p),
        L_BRACKET => tuple_type(p),
        IDENT => type_ref(p),
        _ => p.error("expected type"),
    }
}
```

Important: `LT`/`GT` disambiguation — when parsing `Uint<8>`, the `<` and `>` are generic delimiters, not comparison operators. The type parser always treats `<`/`>` as angle brackets. The expression parser handles `<`/`>` as comparisons.

- [ ] **Step 3: Implement generic argument and parameter lists**

`generic_arg_list`: `<` comma-sep gargs `>`
`generic_param_list`: `<` comma-sep generic-params `>`
`generic_param`: `#`? identifier

- [ ] **Step 4: Run tests**

Run: `cargo test -p compactp_parser`
Expected: all type tests pass

- [ ] **Step 5: Commit**

```bash
git add crates/compactp_parser/
git commit -m "feat(parser): implement type parsing including generics and range-based Uint"
```

---

### Task 12: Grammar — Patterns

**Files:**
- Create: `crates/compactp_parser/src/grammar/patterns.rs`
- Modify: `crates/compactp_parser/src/grammar/mod.rs`

**Reference:** LS.md Section 7, tree-sitter grammar.js lines 458-480.

- [ ] **Step 1: Write pattern tests**

- `x` (identifier pattern)
- `[a, b]` (tuple destructuring)
- `{ x, y }` (struct destructuring)
- `{ x: px, y: py }` (struct with renaming)
- `x: Field` (typed pattern in parameter position — produces `TYPED_PAT`)

Include recovery tests: unclosed `[` in tuple pattern, missing `,` in struct pattern.

- [ ] **Step 2: Implement pattern parsing**

```rust
pub(crate) fn pattern(p: &mut Parser) {
    match p.current() {
        IDENT => { let m = p.start(); p.bump(IDENT); m.complete(p, IDENT_PAT); }
        L_BRACKET => tuple_pattern(p),
        L_BRACE => struct_pattern(p),
        _ => p.error("expected pattern"),
    }
}
```

- [ ] **Step 3: Run tests, commit**

```bash
git add crates/compactp_parser/
git commit -m "feat(parser): implement pattern parsing (ident, tuple, struct)"
```

---

### Task 13: Grammar — Statements

**Files:**
- Create: `crates/compactp_parser/src/grammar/statements.rs`
- Modify: `crates/compactp_parser/src/grammar/mod.rs`

**Reference:** LS.md Section 8, tree-sitter grammar.js lines 415-456, design spec Section 4.2 statement nodes.

- [ ] **Step 1: Write statement tests**

Cover: assignment (`x = 1;`), compound assignment (`x += 1;`), expression statement (`foo();`), return (`return x;`, `return;`, `return a, b;`), if/else, for, assert, const (with and without type annotation), blocks, multi-const.

Include recovery tests: missing `;` after assignment, missing `)` in if condition, missing `of` in for loop, broken assert arguments.

- [ ] **Step 2: Implement statement parsing**

The statement dispatcher inspects the current token:

```rust
pub(crate) fn statement(p: &mut Parser) {
    match p.current() {
        RETURN_KW => return_stmt(p),
        IF_KW => if_stmt(p),
        FOR_KW => for_stmt(p),
        ASSERT_KW => assert_stmt(p),
        CONST_KW => const_stmt(p),
        L_BRACE => block(p),
        _ => expr_or_assign_stmt(p),
    }
}
```

`expr_or_assign_stmt` parses an expression, then checks if the next token is `=`, `+=`, or `-=`. If so, it's an assignment; otherwise it's an expression statement.

Note: `assert` is parsed as `assert(condition, "message")` per LS.md Section 8.7 — call-like syntax, not bare `assert expr str`.

- [ ] **Step 3: Run tests, commit**

```bash
git add crates/compactp_parser/
git commit -m "feat(parser): implement statement parsing"
```

---

### Task 14: Grammar — Expressions (Pratt Parser)

**Files:**
- Create: `crates/compactp_parser/src/grammar/expressions.rs`
- Modify: `crates/compactp_parser/src/grammar/mod.rs`

**Reference:** Design spec Section 3.3 for precedence table. LS.md Section 9 for all expression forms. Tree-sitter grammar.js lines 489-687.

This is the largest and most complex grammar task.

- [ ] **Step 1: Write expression tests**

Cover every expression form:
- Literals: `42`, `true`, `"hello"`, `0xFF`
- Binary: `a + b`, `a * b + c` (precedence), `a == b`
- Unary: `!x`
- Ternary: `a ? b : c`
- Cast: `x as Uint<16>`
- Call: `foo(a, b)`, `T$t(0)`
- Member access: `point.x`, `STATE.set`
- Method call: `calc.get_square(i)`
- Index: `arr[0]`
- Array literal: `[1, 2, 3]`, `[]`
- Bytes literal: `Bytes[1, 2, 3]`, `Bytes[...a, ...b]`
- Spread: `[...vec1, ...vec2]`
- Struct literal: `Point { x, y }`, `Cfg { threshold: 100 }`
- Struct update: `Point { x: 1, ...other }`
- Default: `default<Field>`
- Map: `map((x: Field) => Bytes[x as Uint<8>], fields)`
- Fold: `fold((acc, val) => acc + val, 0 as Field, [1, 2, 3])`
- Disclose: `disclose(v)`
- Pad: `pad(32, "string")`
- Slice: `slice<1>(vec, offset)`
- Lambda: `(x: Field) => x`, `(a, b) => a + b`, `() => { return default<Field>; }`
- Parenthesized: `(a + b)`
- Expression sequence: `return a, b;` (comma-separated in return context)
- Precedence: `a + b * c` → `a + (b * c)`, `a || b && c` → `a || (b && c)`

Include recovery tests: broken binary expressions (missing RHS), broken cast (missing type after `as`), unclosed parenthesized expressions, broken member access chains.

- [ ] **Step 2: Implement Pratt expression parser**

The core Pratt loop:

```rust
pub(crate) fn expr(p: &mut Parser) -> Option<CompletedMarker> {
    expr_bp(p, 0)
}

fn expr_bp(p: &mut Parser, min_bp: u8) -> Option<CompletedMarker> {
    let mut lhs = lhs(p)?;

    loop {
        let (op_kind, left_bp, right_bp) = match p.current() {
            QUESTION => {
                // Ternary: special case
                let m = lhs.precede(p);
                p.bump(QUESTION);
                expr(p); // then branch
                p.expect(COLON);
                expr(p); // else branch
                lhs = m.complete(p, TERNARY_EXPR);
                continue;
            }
            PIPE_PIPE => (PIPE_PIPE, 1, 2),
            AMP_AMP => (AMP_AMP, 3, 4),
            EQ_EQ | BANG_EQ => (p.current(), 5, 6),
            LT | LT_EQ | GT | GT_EQ => (p.current(), 7, 8),
            AS_KW => {
                // Cast: special — RHS is a type, not an expression
                if min_bp > 9 { break; }
                let m = lhs.precede(p);
                p.bump(AS_KW);
                type_(p);
                lhs = m.complete(p, CAST_EXPR);
                continue;
            }
            PLUS | MINUS => (p.current(), 11, 12),
            STAR => (STAR, 13, 14),
            // Postfix: member access, indexing, method calls
            DOT => {
                let m = lhs.precede(p);
                p.bump(DOT);
                p.expect(IDENT);
                if p.at(L_PAREN) {
                    // Method call: expr.id(args) — produces CALL_EXPR wrapping MEMBER_EXPR
                    // First complete the member access as MEMBER_EXPR
                    let member = m.complete(p, MEMBER_EXPR);
                    // Then wrap in CALL_EXPR for the argument list
                    let call_m = member.precede(p);
                    p.bump(L_PAREN);
                    arg_list(p);
                    p.expect(R_PAREN);
                    lhs = call_m.complete(p, CALL_EXPR);
                } else {
                    // Plain member access: expr.id
                    lhs = m.complete(p, MEMBER_EXPR);
                }
                continue;
            }
            L_BRACKET => {
                let m = lhs.precede(p);
                p.bump(L_BRACKET);
                expr(p);
                p.expect(R_BRACKET);
                lhs = m.complete(p, INDEX_EXPR);
                continue;
            }
            _ => break,
        };

        if left_bp < min_bp { break; }

        let m = lhs.precede(p);
        p.bump(op_kind);
        expr_bp(p, right_bp);
        lhs = m.complete(p, BINARY_EXPR);
    }

    Some(lhs)
}
```

The `lhs` function handles atoms and prefix operators:

```rust
fn lhs(p: &mut Parser) -> Option<CompletedMarker> {
    match p.current() {
        BANG => {
            let m = p.start();
            p.bump(BANG);
            expr_bp(p, 15); // unary BP
            Some(m.complete(p, UNARY_EXPR))
        }
        INT_LIT | HEX_LIT | OCT_LIT | BIN_LIT | STRING_LIT | TRUE_KW | FALSE_KW => {
            // Literals are raw tokens in the CST — no wrapper node needed.
            // The token itself (e.g., INT_LIT "42") is the leaf node.
            // Return a CompletedMarker so the Pratt loop can wrap it in binary expressions.
            let m = p.start();
            p.bump_any();
            Some(m.complete(p, p.previous_kind())) // complete with the token's own kind
            // Note: in practice, you may want a thin wrapper node for uniformity.
            // The design spec has no LITERAL node kind, so keep literals as unwrapped tokens.
            // An alternative is to complete with the expression's own kind — adjust during implementation.
        }
        IDENT => ident_or_call_or_struct(p), // handles IDENT, CALL_EXPR, STRUCT_EXPR
        L_PAREN => paren_expr_or_lambda(p),
        L_BRACKET => array_expr(p),
        DEFAULT_KW => default_expr(p),
        MAP_KW => map_expr(p),
        FOLD_KW => fold_expr(p),
        DISCLOSE_KW => disclose_expr(p),
        PAD_KW => pad_expr(p),
        SLICE_KW => slice_expr(p),
        BYTES_KW => bytes_expr(p),
        _ => {
            p.error("expected expression");
            None
        }
    }
}
```

Note: The exact binding power numbers need to match the design spec precedence table. Use even numbers for left-associative (left_bp = N, right_bp = N+1) and odd for right-associative. Adjust values so the ordering matches spec Section 3.3.

- [ ] **Step 3: Implement all expression-specific sub-parsers**

Each special expression form (default, map, fold, disclose, pad, slice, bytes, lambda, struct literal) needs its own parsing function. These are called from `lhs()`.

Key tricky spots:
- **Lambda vs parenthesized expr:** `(x) => x` is a lambda, `(x)` is a parenthesized expr, `(x, y)` is an expr sequence. Look ahead after `)` for `=>` to disambiguate.
- **Struct literal vs block:** `Ident { ... }` — when `IDENT` is followed by `{`, it could be a struct literal. Check if the identifier could be a type name (heuristic: in expression position after seeing an identifier, if `{` follows, try parsing as struct literal).
- **Bytes literal:** `Bytes[1, 2, 3]` — the `Bytes` keyword followed by `[` is special syntax, not a generic type + index.

- [ ] **Step 4: Run tests and iterate**

Run: `cargo test -p compactp_parser`
Expected: all expression tests pass

- [ ] **Step 5: Commit**

```bash
git add crates/compactp_parser/
git commit -m "feat(parser): implement Pratt expression parser with full operator precedence"
```

---

### Task 15: Grammar — Version Expressions & Error Recovery

**Files:**
- Create: `crates/compactp_parser/src/grammar/version.rs`
- Modify: `crates/compactp_parser/src/grammar/declarations.rs`
- Modify: `crates/compactp_parser/src/parser.rs`

**Reference:** Tree-sitter grammar.js lines 86-142 for version expressions. Design spec Section 3.4 for error recovery.

- [ ] **Step 1: Write version expression tests**

```rust
#[test]
fn parse_pragma_with_comparison() {
    check("pragma language_version >= 0.15.0;", /* expected CST */);
}

#[test]
fn parse_pragma_with_boolean_version_expr() {
    check("pragma language_version >= 0.15.0 && < 1.0.0;", /* expected CST */);
}
```

- [ ] **Step 2: Implement version expression parsing**

Version expressions have their own mini-grammar: `||`, `&&`, `!`, comparisons, parenthesized groups. Parse them in `version.rs` and call from `pragma()` in `declarations.rs`.

- [ ] **Step 3: Write error recovery tests**

Test all recovery scenarios from design spec Section 3.4:
- Missing semicolons
- Missing commas
- Missing closing delimiters
- Malformed parameter lists
- Broken expressions
- Broken casts

For each, verify: no panic, ERROR nodes present, diagnostics generated, surrounding tree intact.

- [ ] **Step 4: Implement error recovery**

Add recovery logic to the parser:
1. Define recovery sets as `&[SyntaxKind]` for each grammar position
2. Add `fn recover(p: &mut Parser, recovery: &[SyntaxKind])` that wraps unexpected tokens in ERROR nodes until a recovery token is found
3. Add error budgeting: track error count, stop recovery after max_errors (256 default)

- [ ] **Step 5: Run all tests**

Run: `cargo test -p compactp_parser`
Expected: all tests pass including recovery tests

- [ ] **Step 6: Commit**

```bash
git add crates/compactp_parser/
git commit -m "feat(parser): add version expressions and error recovery"
```

---

### Task 16: Public Parse API & Initial Corpus Smoke Test

**Files:**
- Modify: `crates/compactp_parser/src/lib.rs`

**Reference:** Design spec Section 8 for API shape.

- [ ] **Step 1: Write API tests**

```rust
#[test]
fn parse_api_returns_green_node() {
    let result = parse("ledger x: Field;");
    let root = SyntaxNode::new_root(result.green);
    assert_eq!(root.kind(), SyntaxKind::SOURCE_FILE);
    assert!(result.diagnostics.is_empty());
}

#[test]
fn parse_with_options_no_recovery() {
    let opts = ParseOptions { recover: false, max_errors: 0 };
    let result = parse_with("ledger x: Field", opts); // missing semicolon
    assert!(!result.diagnostics.is_empty());
}

#[test]
fn parse_file_reads_from_disk() {
    // Create a temp file, parse it
    let result = parse_file(Path::new("tests/fixtures/tiny.compact"));
    assert!(result.is_ok());
}
```

- [ ] **Step 2: Implement parse_file and finalize public API**

The `parse()`, `parse_with()`, `ParseOptions`, and `ParseResult` were already defined in Task 8 Step 6. Add `parse_file`:

```rust
pub fn parse_file(path: &std::path::Path) -> Result<ParseResult, std::io::Error> {
    let source = std::fs::read_to_string(path)?;
    Ok(parse(&source))
}
```

Ensure `ParseResult` uses `Vec<compactp_diagnostics::Diagnostic>` (not `Vec<String>`) so the diagnostics carry severity, codes, spans, and notes as structured data. This is a public contract requirement (Constitution Principle IV).

- [ ] **Step 3: Smoke test with a real .compact file**

Copy `/tmp/compactp/references/compact/examples/tiny.compact` to a temp location and parse it:

```rust
#[test]
fn parse_tiny_compact() {
    let source = include_str!("/tmp/compactp/references/compact/examples/tiny.compact");
    let result = parse(source);
    // Should parse without panics. May have errors initially as grammar is refined.
    let root = SyntaxNode::new_root(result.green);
    assert_eq!(root.kind(), SyntaxKind::SOURCE_FILE);
}
```

- [ ] **Step 4: Commit**

```bash
git add crates/compactp_parser/
git commit -m "feat(parser): add public parse API with ParseOptions"
```

---

### Task 17: Extract Test Corpus

**Files:**
- Create: `tests/corpus/` (copy from references)
- Create: `tests/corpus/LICENSE-APACHE-2.0`
- Create: `tests/fixtures/` (curated subset)

**Reference:** The 489 .compact files in `/tmp/compactp/references/compact/examples/`.

- [ ] **Step 1: Copy upstream corpus**

```bash
cp -r /tmp/compactp/references/compact/examples/* tests/corpus/
```

Preserve the directory structure (adt/, bytes/, composable/, modules/, vectors/, etc.).

- [ ] **Step 2: Add Apache-2.0 license**

Create `tests/corpus/LICENSE-APACHE-2.0` noting these files are from the upstream Compact project, copyright Midnight Foundation, Apache-2.0 licensed.

- [ ] **Step 3: Create curated fixtures**

Create `tests/fixtures/` with hand-picked files organized by parser concern:
- `tests/fixtures/declarations/` — one file per declaration type
- `tests/fixtures/expressions/` — expression edge cases
- `tests/fixtures/imports/` — all import forms
- `tests/fixtures/types/` — all type forms
- `tests/fixtures/recovery/` — intentionally broken files

For each fixture, create the `.compact` file. These will be used for snapshot tests.

- [ ] **Step 4: Commit**

```bash
git add tests/
git commit -m "feat: extract upstream test corpus and create curated fixtures"
```

---

### Task 18: compactp_ast — Typed AST Wrappers

**Files:**
- Create: `crates/compactp_ast/src/lib.rs`
- Create: `crates/compactp_ast/src/support.rs`
- Create: `crates/compactp_ast/src/nodes.rs`
- Create: `crates/compactp_ast/src/expr.rs`

**Reference:** Design spec Section 5 for AST types. Section 5.3 for the complete type inventory.

- [ ] **Step 1: Write AST accessor tests**

```rust
#[test]
fn circuit_def_accessors() {
    let source = "export pure circuit foo(x: Field): Field { return x; }";
    let result = compactp_parser::parse(source);
    let root = SyntaxNode::new_root(result.green);
    let circuit = root.children()
        .find_map(CircuitDef::cast)
        .expect("should have a CircuitDef");

    assert!(circuit.is_exported());
    assert!(circuit.is_pure());
    assert_eq!(circuit.name().unwrap().text(), "foo");
    assert!(circuit.body().is_some());
}

#[test]
fn ledger_decl_accessors() {
    let source = "export ledger value: Field;";
    let result = compactp_parser::parse(source);
    let root = SyntaxNode::new_root(result.green);
    let ledger = root.children()
        .find_map(LedgerDecl::cast)
        .expect("should have a LedgerDecl");

    assert!(ledger.export_kw().is_some());
    assert_eq!(ledger.name().unwrap().text(), "value");
}
```

- [ ] **Step 2: Implement AstNode trait and support helpers**

In `crates/compactp_ast/src/support.rs`:

```rust
use compactp_syntax::{SyntaxKind, SyntaxNode, SyntaxToken};

pub(crate) fn child_token(parent: &SyntaxNode, kind: SyntaxKind) -> Option<SyntaxToken> {
    parent.children_with_tokens()
        .filter_map(|it| it.into_token())
        .find(|it| it.kind() == kind)
}

pub(crate) fn child_node<N: AstNode>(parent: &SyntaxNode) -> Option<N> {
    parent.children().find_map(N::cast)
}

pub(crate) fn children_nodes<N: AstNode>(parent: &SyntaxNode) -> impl Iterator<Item = N> + '_ {
    parent.children().filter_map(N::cast)
}
```

In `crates/compactp_ast/src/lib.rs`:

```rust
mod support;
mod nodes;
mod expr;

use compactp_syntax::{SyntaxNode, SyntaxToken, SyntaxKind};

pub trait AstNode: Sized {
    fn can_cast(kind: SyntaxKind) -> bool;
    fn cast(node: SyntaxNode) -> Option<Self>;
    fn syntax(&self) -> &SyntaxNode;
}
```

- [ ] **Step 3: Implement all AST node types**

In `nodes.rs`: SourceFile, Pragma, Include, Import, ImportSpecifier, ExportList, ModuleDef, LedgerDecl, ConstructorDef, CircuitDef, CircuitDecl, WitnessDecl, ContractDecl, ContractCircuit, StructDef, EnumDef, Block, all statement types, all type types, all pattern types.

In `expr.rs`: All expression types and the `Expr` sum-type enum.

Each type follows the pattern:
```rust
pub struct CircuitDef(SyntaxNode);

impl AstNode for CircuitDef {
    fn can_cast(kind: SyntaxKind) -> bool { kind == SyntaxKind::CIRCUIT_DEF }
    fn cast(node: SyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) { Some(Self(node)) } else { None }
    }
    fn syntax(&self) -> &SyntaxNode { &self.0 }
}

impl CircuitDef {
    pub fn export_kw(&self) -> Option<SyntaxToken> { support::child_token(&self.0, SyntaxKind::EXPORT_KW) }
    pub fn pure_kw(&self) -> Option<SyntaxToken> { support::child_token(&self.0, SyntaxKind::PURE_KW) }
    pub fn name(&self) -> Option<SyntaxToken> { support::child_token(&self.0, SyntaxKind::IDENT) }
    pub fn generic_params(&self) -> Option<GenericParamList> { support::child_node(&self.0) }
    pub fn params(&self) -> impl Iterator<Item = Param> + '_ { support::children_nodes(&self.0) }
    pub fn return_type(&self) -> Option<Type> { support::child_node(&self.0) }
    pub fn body(&self) -> Option<Block> { support::child_node(&self.0) }
    pub fn is_exported(&self) -> bool { self.export_kw().is_some() }
    pub fn is_pure(&self) -> bool { self.pure_kw().is_some() }
}
```

Implement the Stmt, Expr, Type, and Pat sum-type enums as Rust enums with AstNode implementations that try each variant.

- [ ] **Step 4: Run tests**

Run: `cargo test -p compactp_ast`
Expected: all accessor tests pass

- [ ] **Step 5: Commit**

```bash
git add crates/compactp_ast/
git commit -m "feat(ast): add typed AST wrappers for all language constructs"
```

---

### Task 19: compactp_diagnostics — Model & Renderers

**Files:**
- Create: `crates/compactp_diagnostics/src/lib.rs`
- Create: `crates/compactp_diagnostics/src/render.rs`
- Create: `crates/compactp_diagnostics/src/json.rs`

**Reference:** Design spec Section 6.

- [ ] **Step 1: Write diagnostic rendering tests**

Use `insta` snapshot tests for both human and JSON rendering. This ensures rendering output is tracked as a stable contract, consistent with how CLI JSON contract tests work in Task 20.

```rust
#[test]
fn render_human_diagnostic() {
    let diag = Diagnostic {
        severity: Severity::Error,
        code: DiagnosticCode::new("E", 12),
        message: "expected `;`".into(),
        primary_span: TextRange::new(23.into(), 24.into()),
        secondary_spans: vec![],
        notes: vec![],
    };
    let source = "ledger count: Field\n";
    let rendered = render_human(&diag, source, "test.compact", false);
    insta::assert_snapshot!(rendered);
}

#[test]
fn render_json_diagnostic() {
    let diag = Diagnostic {
        severity: Severity::Error,
        code: DiagnosticCode::new("E", 12),
        message: "expected `;`".into(),
        primary_span: TextRange::new(23.into(), 24.into()),
        secondary_spans: vec![],
        notes: vec![],
    };
    let json = render_json(&diag);
    insta::assert_json_snapshot!(json);
}
```

- [ ] **Step 2: Implement Diagnostic model**

```rust
use rowan::TextRange;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: DiagnosticCode,
    pub message: String,
    #[serde(serialize_with = "serialize_text_range")]
    pub primary_span: TextRange,
    pub secondary_spans: Vec<LabeledSpan>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity { Error, Warning, Note }

#[derive(Debug, Clone, Serialize)]
pub struct DiagnosticCode {
    pub prefix: &'static str,
    pub number: u16,
}

#[derive(Debug, Clone, Serialize)]
pub struct LabeledSpan {
    #[serde(serialize_with = "serialize_text_range")]
    pub span: TextRange,
    pub label: Option<String>,
}
```

- [ ] **Step 3: Implement human renderer**

In `render.rs`: format diagnostics in rustc-style with source snippets, line/column numbers. Support optional ANSI color codes.

- [ ] **Step 4: Implement JSON renderer**

In `json.rs`: serialize diagnostics to `serde_json::Value` with file, start/end line/col.

- [ ] **Step 5: Run tests, commit**

```bash
git add crates/compactp_diagnostics/
git commit -m "feat(diagnostics): add diagnostic model with human and JSON renderers"
```

---

### Task 20: CLI — Skeleton & Input Handling

**Files:**
- Create: `crates/compactp/src/main.rs`
- Create: `crates/compactp/src/input.rs`
- Create: `crates/compactp/src/output.rs`
- Create: `crates/compactp/src/commands/mod.rs`

**Reference:** Design spec Section 7.

- [ ] **Step 1: Write CLI integration test**

```rust
// tests in crates/compactp/tests/ or using assert_cmd
#[test]
fn cli_parse_exit_code_success() {
    Command::cargo_bin("compactp").unwrap()
        .arg("parse")
        .arg("tests/fixtures/declarations/ledger.compact")
        .assert()
        .success();
}

#[test]
fn cli_parse_exit_code_error() {
    Command::cargo_bin("compactp").unwrap()
        .arg("parse")
        .arg("tests/fixtures/recovery/missing_semicolon.compact")
        .assert()
        .code(1);
}

#[test]
fn cli_invalid_usage() {
    // Note: clap defaults to exit code 2 for usage errors. We need exit code 3
    // per the design spec. Override clap's error handler in main.rs:
    //   if let Err(e) = cli.try_parse() {
    //       e.print().ok();
    //       std::process::exit(3);
    //   }
    Command::cargo_bin("compactp").unwrap()
        .arg("--nonexistent-flag")
        .assert()
        .code(3);
}
```

- [ ] **Step 2: Implement clap CLI structure**

```rust
use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(name = "compactp", about = "Compact language parser")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(long, default_value = "human", global = true)]
    format: OutputFormat,

    #[arg(long, global = true)]
    pretty: bool,

    #[arg(long, default_value = "auto", global = true)]
    color: ColorChoice,

    #[arg(long, global = true)]
    timing: bool,

    #[arg(long, global = true)]
    stdin_filename: Option<String>,

    #[arg(long, global = true)]
    max_diagnostics: Option<usize>,

    /// Maximum parse errors before the parser stops recovery (default: 256)
    #[arg(long, global = true)]
    max_errors: Option<usize>,

    #[arg(long, global = true)]
    no_recover: bool,

    #[arg(long, global = true)]
    stop_after: Option<StopAfter>,

    #[arg(long, global = true)]
    language_version: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    Lex { paths: Vec<PathBuf> },
    Parse { paths: Vec<PathBuf> },
    Cst { paths: Vec<PathBuf> },
    Ast { paths: Vec<PathBuf> },
    Diag { paths: Vec<PathBuf> },
    Stats { paths: Vec<PathBuf> },
    Watch {
        #[command(subcommand)]
        command: WatchableCommand,
        paths: Vec<PathBuf>,
    },
}
```

- [ ] **Step 3: Implement input resolution**

In `input.rs`: resolve paths (files, directories recursive with `*.compact` filter, stdin), deterministic ordering, symlink following with cycle detection.

- [ ] **Step 4: Implement JSON output envelope**

In `output.rs`:

```rust
use serde::Serialize;

#[derive(Serialize)]
pub struct OutputEnvelope<T: Serialize> {
    pub tool_version: &'static str,
    pub schema_version: u32,
    pub language_version: &'static str,
    pub input: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timing_ms: Option<f64>,
    pub data: T,
}

pub const TOOL_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const SCHEMA_VERSION: u32 = 1;
pub const LANGUAGE_VERSION: &str = "0.22.0";
```

- [ ] **Step 5: Run tests, commit**

```bash
git add crates/compactp/
git commit -m "feat(cli): add clap CLI skeleton with input resolution and JSON envelope"
```

---

### Task 21: CLI — All Commands

**Files:**
- Create: `crates/compactp/src/commands/lex.rs`
- Create: `crates/compactp/src/commands/parse.rs`
- Create: `crates/compactp/src/commands/cst.rs`
- Create: `crates/compactp/src/commands/ast.rs`
- Create: `crates/compactp/src/commands/diag.rs`
- Create: `crates/compactp/src/commands/stats.rs`

**Reference:** Design spec Section 7.1.

- [ ] **Step 1: Write command output tests**

For each command, test human and JSON output format. Use insta snapshot tests for JSON contract stability.

- [ ] **Step 2: Implement lex command**

Print token stream in human format (kind, text, span) or JSON format (array of token objects).

- [ ] **Step 3: Implement parse command**

Parse files, report diagnostics, exit 0 for no errors, exit 1 for errors.

- [ ] **Step 4: Implement cst command**

Dump CST using rowan's debug format (human) or a JSON tree representation.

- [ ] **Step 5: Implement ast command**

Dump typed AST. Human format shows typed node names and accessor values. JSON format provides the structured AST.

- [ ] **Step 6: Implement diag command**

Emit diagnostics only (no tree output). Human and JSON modes.

- [ ] **Step 7: Implement stats command**

Report: token count, node count, parse time, file size, error count, recovery count.

- [ ] **Step 8: Run all tests, commit**

```bash
git add crates/compactp/
git commit -m "feat(cli): implement all CLI commands (lex, parse, cst, ast, diag, stats)"
```

---

### Task 22: CLI — Watch Mode

**Files:**
- Create: `crates/compactp/src/commands/watch.rs`

**Reference:** Design spec Section 7.5.

- [ ] **Step 1: Implement watch mode**

Use `notify-debouncer-full` (0.7) to watch files/directories. On change:
- Human mode: clear terminal, show changed file, run selected command, show results
- Machine mode: emit JSONL event with timestamp, files, result

```rust
use notify_debouncer_full::{new_debouncer, DebouncedEvent};
use std::time::Duration;

pub fn run_watch(command: &WatchableCommand, paths: &[PathBuf], opts: &GlobalOpts) -> Result<(), Error> {
    let (tx, rx) = std::sync::mpsc::channel();
    let mut debouncer = new_debouncer(Duration::from_millis(200), None, tx)?;

    for path in paths {
        debouncer.watch(path, notify::RecursiveMode::Recursive)?;
    }

    // Initial run
    run_command_on_files(command, paths, opts)?;

    // Watch loop
    for events in rx {
        match events {
            Ok(events) => {
                let changed: Vec<_> = events.iter()
                    .filter_map(|e| e.paths.first())
                    .filter(|p| p.extension().is_some_and(|ext| ext == "compact"))
                    .collect();
                if !changed.is_empty() {
                    run_command_on_files(command, paths, opts)?;
                }
            }
            Err(e) => eprintln!("Watch error: {e}"),
        }
    }
    Ok(())
}
```

- [ ] **Step 2: Test watch mode**

Manual testing is acceptable for watch mode given its interactive nature. Add a basic test that starts watch, triggers a file change, and verifies output is produced.

- [ ] **Step 3: Commit**

```bash
git add crates/compactp/
git commit -m "feat(cli): implement watch mode with debounced file watching"
```

---

### Task 23: Corpus Integration Tests & Benchmarks

**Files:**
- Create: `tests/corpus_test.rs` (workspace-level integration test)
- Create: `benches/parse_bench.rs`

**Reference:** Design spec Section 11.

- [ ] **Step 1: Write corpus integration test**

```rust
use std::path::Path;
use walkdir::WalkDir;

#[test]
fn parse_entire_corpus_without_panics() {
    let corpus_dir = Path::new("tests/corpus");
    let mut total = 0;
    let mut errors = 0;

    for entry in WalkDir::new(corpus_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "compact"))
    {
        total += 1;
        let source = std::fs::read_to_string(entry.path()).unwrap();
        let result = compactp_parser::parse(&source);
        // The parse must never panic
        let root = compactp_syntax::SyntaxNode::new_root(result.green);
        assert_eq!(root.kind(), compactp_syntax::SyntaxKind::SOURCE_FILE);

        if !result.errors.is_empty() {
            // Files in "negative/" directories are expected to have errors
            let is_negative = entry.path().components()
                .any(|c| c.as_os_str() == "negative");
            if !is_negative {
                errors += 1;
                eprintln!("ERRORS in {}: {:?}", entry.path().display(), &result.errors[..result.errors.len().min(3)]);
            }
        }
    }

    eprintln!("Parsed {total} files, {errors} unexpected errors");
    // This assertion MUST be active — a green corpus test that hides failures
    // violates Constitution Principle III (Correctness) and V (Test What Matters).
    // If the parser can't handle all files yet, fix the parser — don't disable the test.
    assert_eq!(errors, 0, "{errors} files had unexpected parse errors out of {total} total");
}
```

Note: Add `walkdir` as a dev-dependency in the workspace root Cargo.toml for this test. Verify version with `cargo search walkdir`.

- [ ] **Step 2: Run corpus test and fix parser issues**

Run: `cargo test --test corpus_test`
Expected: no panics. Iteratively fix parser grammar issues discovered by the corpus.

- [ ] **Step 3: Write benchmarks**

```rust
use criterion::{criterion_group, criterion_main, Criterion};

fn bench_parse_tiny(c: &mut Criterion) {
    let source = include_str!("../tests/corpus/tiny.compact");
    c.bench_function("parse tiny.compact", |b| {
        b.iter(|| compactp_parser::parse(source));
    });
}

fn bench_lex_tiny(c: &mut Criterion) {
    let source = include_str!("../tests/corpus/tiny.compact");
    c.bench_function("lex tiny.compact", |b| {
        b.iter(|| compactp_lexer::lex(source));
    });
}

criterion_group!(benches, bench_parse_tiny, bench_lex_tiny);
criterion_main!(benches);
```

- [ ] **Step 4: Run benchmarks**

Run: `cargo bench`
Expected: benchmarks produce results

- [ ] **Step 5: Commit**

```bash
git add tests/ benches/ Cargo.toml
git commit -m "feat: add corpus integration tests and parse benchmarks"
```

---

### Task 24: JSON Contract Snapshot Tests

**Files:**
- Create snapshot tests for each CLI command's JSON output

- [ ] **Step 1: Write JSON snapshot tests**

Use `insta` to snapshot the JSON output from each command for a known input file. These become the public contract — any change to the JSON shape requires updating the snapshot and bumping the schema version.

Test: `lex --format json`, `parse --format json`, `cst --format json`, `ast --format json`, `diag --format json`, `stats --format json`.

Verify each output includes the envelope metadata (tool_version, schema_version, language_version, input).

**Schema version governance (Constitution Principle IV):** Any change to a JSON snapshot file constitutes a potential contract break. Before accepting a snapshot update:
1. Determine if the change is additive (new field) or breaking (removed/renamed field, changed type).
2. Additive changes: acceptable without version bump.
3. Breaking changes: bump `SCHEMA_VERSION` in `output.rs` before updating snapshots.
4. Document the change in the commit message.

- [ ] **Step 2: Run snapshot tests**

Run: `cargo test -p compactp` (or workspace-level integration tests)
Expected: snapshots created on first run, pass on subsequent runs

- [ ] **Step 3: Commit**

```bash
git add crates/compactp/tests/ crates/compactp/snapshots/
git commit -m "test: add JSON contract snapshot tests for all CLI commands"
```

---

### Task 25: Interactive Demo Script

**Purpose:** Create a `demo.sh` script that builds all targets and walks a user through every CLI command with narrated explanations, diverse inputs (valid, invalid, and outdated Compact code), and pauses between screens. This serves as both a showcase and a manual smoke test.

**Files:**
- Create: `demo.sh`
- Create: `tests/fixtures/demo/valid.compact`
- Create: `tests/fixtures/demo/invalid.compact`
- Create: `tests/fixtures/demo/outdated.compact`

- [ ] **Step 1: Create demo fixture files**

Create three fixture files that cover the key scenarios:

`tests/fixtures/demo/valid.compact` — well-formed, current-version Compact code exercising multiple language features:

```compact
pragma language_version >= 0.22.0;

export ledger count: Counter;

export circuit increment(): [] {
    count.increment(1);
}

export circuit get_count(): Field {
    return count.value();
}
```

`tests/fixtures/demo/invalid.compact` — syntactically broken code that should produce clear diagnostics:

```compact
pragma language_version >= 0.22.0;

ledger value: Field

circuit broken(x: Field): Field {
    return x +;
}

circuit missing_brace(): [] {
    const a = 1;
```

`tests/fixtures/demo/outdated.compact` — code that was valid under an earlier language version but uses constructs no longer valid (adjust to match actual deprecated/removed syntax from the Compact changelog):

```compact
pragma language_version >= 0.10.0;

// Uses syntax or patterns that were valid in 0.10.x but have since
// been removed or changed in 0.22.x. Consult the upstream compact
// release notes to identify real examples of breaking changes.
// Placeholder — replace with actual outdated constructs during implementation.

ledger old_field: Field;

circuit old_style(): Field {
    return old_field;
}
```

Note: During implementation, consult `/tmp/compactp/references/compact/` release notes and changelogs to find real examples of syntax that was valid in earlier versions but is now rejected or deprecated. Replace the placeholder `outdated.compact` with authentic examples.

- [ ] **Step 2: Create demo.sh**

```bash
#!/usr/bin/env bash
set -euo pipefail

# Colors
BOLD='\033[1m'
DIM='\033[2m'
CYAN='\033[36m'
GREEN='\033[32m'
YELLOW='\033[33m'
RED='\033[31m'
RESET='\033[0m'

VALID="tests/fixtures/demo/valid.compact"
INVALID="tests/fixtures/demo/invalid.compact"
OUTDATED="tests/fixtures/demo/outdated.compact"

pause() {
    echo ""
    echo -e "${DIM}Press Enter to continue...${RESET}"
    read -r
    clear
}

header() {
    echo -e "${BOLD}${CYAN}════════════════════════════════════════════════════════════════${RESET}"
    echo -e "${BOLD}${CYAN}  $1${RESET}"
    echo -e "${BOLD}${CYAN}════════════════════════════════════════════════════════════════${RESET}"
    echo ""
}

explain() {
    echo -e "${BOLD}$1${RESET}"
    echo -e "${DIM}$2${RESET}"
    echo ""
}

show_input() {
    echo -e "${YELLOW}Input file: $1${RESET}"
    echo -e "${DIM}$(cat "$1")${RESET}"
    echo ""
}

run_cmd() {
    echo -e "${GREEN}\$ $1${RESET}"
    echo ""
    eval "$1"
}

# ── Build ─────────────────────────────────────────────────────────
clear
header "compactp — Interactive Demo"
echo "This demo walks through every CLI command with valid, invalid,"
echo "and outdated Compact source code."
echo ""
echo "Building all targets first..."
echo ""
cargo build --workspace --release
echo ""
echo -e "${GREEN}Build complete.${RESET}"
pause

# ── lex ───────────────────────────────────────────────────────────
header "1. lex — Tokenize source code"
explain "The 'lex' command breaks source code into a stream of tokens." \
       "Each token has a kind (e.g., IDENT, CIRCUIT_KW, SEMICOLON) and its text."

echo -e "${BOLD}── Valid input ──${RESET}"
show_input "$VALID"
run_cmd "cargo run --release -- lex $VALID"
pause

header "1. lex — Invalid input"
explain "Lexing invalid code still produces tokens — errors appear as ERROR tokens." \
       "The lexer is intentionally resilient; it never panics."
show_input "$INVALID"
run_cmd "cargo run --release -- lex $INVALID"
pause

header "1. lex — JSON output"
explain "All commands support --format json for machine consumption." \
       "The JSON envelope includes tool_version, schema_version, and language_version."
run_cmd "cargo run --release -- lex $VALID --format json --pretty"
pause

# ── parse ─────────────────────────────────────────────────────────
header "2. parse — Parse and report diagnostics"
explain "The 'parse' command parses a file and reports any diagnostics." \
       "Exit code 0 = no errors, exit code 1 = errors found."

echo -e "${BOLD}── Valid input (should succeed) ──${RESET}"
show_input "$VALID"
run_cmd "cargo run --release -- parse $VALID" || true
pause

header "2. parse — Invalid input (should report errors)"
explain "Invalid code produces structured diagnostics with line/column info," \
       "error codes, and suggestions where possible."
show_input "$INVALID"
run_cmd "cargo run --release -- parse $INVALID" || true
pause

header "2. parse — Outdated input"
explain "Code written for an earlier language version may parse differently." \
       "The parser reports diagnostics for constructs that are no longer valid."
show_input "$OUTDATED"
run_cmd "cargo run --release -- parse $OUTDATED" || true
pause

# ── cst ───────────────────────────────────────────────────────────
header "3. cst — Dump Concrete Syntax Tree"
explain "The 'cst' command prints the full lossless concrete syntax tree." \
       "Every character of the source is preserved — whitespace, comments, all of it."

echo -e "${BOLD}── Valid input ──${RESET}"
run_cmd "cargo run --release -- cst $VALID"
pause

header "3. cst — Invalid input (shows ERROR nodes)"
explain "When the parser encounters invalid syntax, it wraps the broken" \
       "region in an ERROR node but continues parsing the rest of the file."
run_cmd "cargo run --release -- cst $INVALID"
pause

# ── ast ───────────────────────────────────────────────────────────
header "4. ast — Dump Typed Abstract Syntax Tree"
explain "The 'ast' command prints the typed AST with accessor values." \
       "This is a higher-level view than the CST — trivia is hidden."
run_cmd "cargo run --release -- ast $VALID"
pause

header "4. ast — JSON output"
run_cmd "cargo run --release -- ast $VALID --format json --pretty"
pause

# ── diag ──────────────────────────────────────────────────────────
header "5. diag — Diagnostics only"
explain "The 'diag' command emits only diagnostics (no tree output)." \
       "Useful for CI integration and editor tooling."

echo -e "${BOLD}── Valid input (no diagnostics) ──${RESET}"
run_cmd "cargo run --release -- diag $VALID" || true
pause

header "5. diag — Invalid input"
show_input "$INVALID"
run_cmd "cargo run --release -- diag $INVALID" || true
pause

header "5. diag — JSON diagnostics"
run_cmd "cargo run --release -- diag $INVALID --format json --pretty" || true
pause

# ── stats ─────────────────────────────────────────────────────────
header "6. stats — Parse statistics"
explain "The 'stats' command reports token count, node count, parse time," \
       "file size, error count, and recovery count."
run_cmd "cargo run --release -- stats $VALID"
pause

header "6. stats — Stats for invalid input"
run_cmd "cargo run --release -- stats $INVALID"
pause

# ── Batch processing ─────────────────────────────────────────────
header "7. Batch processing"
explain "All commands accept multiple files or directories." \
       "Directories are scanned recursively for .compact files."
run_cmd "cargo run --release -- parse tests/fixtures/demo/"
pause

# ── Done ──────────────────────────────────────────────────────────
header "Demo complete!"
echo "For more information:"
echo "  cargo run --release -- --help"
echo "  cargo run --release -- parse --help"
echo ""
echo "To start watch mode:"
echo "  cargo run --release -- watch parse tests/fixtures/demo/"
echo ""
```

- [ ] **Step 3: Make demo.sh executable and verify**

```bash
chmod +x demo.sh
# Quick sanity check — just run the build step
bash -c 'source demo.sh <<< ""' || echo "Script created, full run requires built binary"
```

- [ ] **Step 4: Commit**

```bash
git add demo.sh tests/fixtures/demo/
git commit -m "feat: add interactive demo script showcasing all CLI commands

Demonstrates lex, parse, cst, ast, diag, and stats commands against
valid, invalid, and outdated Compact source code with narrated pauses."
```

---

### Task 26: Project Documentation & README

**Purpose:** Review and create all project documentation including the README, ensuring it accurately reflects the implemented CLI, architecture, and usage.

**Files:**
- Create: `README.md`

- [ ] **Step 1: Write README.md**

The README should include:

1. **Project title and badges** — CI status, crates.io version, license
2. **One-line description** — what compactp is and what it does
3. **Features list** — key capabilities (lossless CST, typed AST, error recovery, JSON output, watch mode)
4. **Installation** — via cargo install, Homebrew (`brew install aaronbassett/tap/compactp`), and from source
5. **Quick start** — 3-4 commands showing basic usage
6. **CLI reference** — brief table of all subcommands (lex, parse, cst, ast, diag, stats, watch) with one-line descriptions
7. **Global flags** — `--format`, `--pretty`, `--color`, `--timing`, `--no-recover`, `--max-errors`, `--stop-after`
8. **Exit codes** — 0 (success), 1 (parse errors), 2 (I/O errors), 3 (usage errors)
9. **Architecture overview** — the 6-crate workspace and data flow (source → lexer → parser → CST → AST → diagnostics)
10. **Library usage** — brief example of using `compactp_parser::parse()` as a library
11. **Development** — how to build, test, run benchmarks, and run the demo
12. **License** — MIT
13. **Acknowledgments** — Compact language by Midnight Foundation, rust-analyzer for parser architecture inspiration

Cross-check all CLI flags, subcommands, exit codes, and examples against the actual binary output (`cargo run -- --help`, `cargo run -- parse --help`, etc.) to ensure documentation matches implementation.

- [ ] **Step 2: Review existing documentation**

Verify that doc comments on all public types and functions (generated in earlier tasks) are:
- Accurate — descriptions match actual behavior
- Complete — no undocumented public items
- Consistent — terminology matches README and design spec

```bash
# Check for undocumented public items
cargo doc --workspace --no-deps 2>&1 | grep -i "warning.*missing"
```

- [ ] **Step 3: Commit**

```bash
git add README.md
git commit -m "docs: add comprehensive README with installation, usage, and architecture"
```

---

### Task 27: GitHub Community Docs & Templates

**Purpose:** Add GitHub community health files — issue templates, PR template, contributing guide, code of conduct, and security policy — to encourage quality contributions and consistent issue reporting.

**Files:**
- Create: `.github/ISSUE_TEMPLATE/bug_report.yml`
- Create: `.github/ISSUE_TEMPLATE/feature_request.yml`
- Create: `.github/ISSUE_TEMPLATE/config.yml`
- Create: `.github/PULL_REQUEST_TEMPLATE.md`
- Create: `CONTRIBUTING.md`
- Create: `CODE_OF_CONDUCT.md`
- Create: `SECURITY.md`

- [ ] **Step 1: Create bug report issue template**

Create `.github/ISSUE_TEMPLATE/bug_report.yml`:

```yaml
name: Bug Report
description: Report a parsing bug, incorrect output, or crash
title: "[Bug]: "
labels: ["bug"]
body:
  - type: textarea
    id: description
    attributes:
      label: Describe the bug
      description: A clear description of what went wrong.
    validations:
      required: true
  - type: textarea
    id: input
    attributes:
      label: Input (.compact source)
      description: The Compact source code that triggers the bug.
      render: text
    validations:
      required: true
  - type: textarea
    id: expected
    attributes:
      label: Expected behavior
      description: What you expected to happen.
    validations:
      required: true
  - type: textarea
    id: actual
    attributes:
      label: Actual behavior
      description: What actually happened. Include CLI output if applicable.
      render: shell
    validations:
      required: true
  - type: input
    id: version
    attributes:
      label: compactp version
      description: Output of `compactp --version`
      placeholder: "compactp 0.1.0"
    validations:
      required: true
  - type: dropdown
    id: command
    attributes:
      label: CLI command
      options:
        - lex
        - parse
        - cst
        - ast
        - diag
        - stats
        - watch
        - Other
    validations:
      required: true
  - type: textarea
    id: context
    attributes:
      label: Additional context
      description: OS, upstream Compact compiler version, or anything else relevant.
```

- [ ] **Step 2: Create feature request issue template**

Create `.github/ISSUE_TEMPLATE/feature_request.yml`:

```yaml
name: Feature Request
description: Suggest a new feature or enhancement
title: "[Feature]: "
labels: ["enhancement"]
body:
  - type: textarea
    id: problem
    attributes:
      label: Problem or motivation
      description: What problem does this solve or what workflow does it improve?
    validations:
      required: true
  - type: textarea
    id: solution
    attributes:
      label: Proposed solution
      description: Describe the feature you'd like to see.
    validations:
      required: true
  - type: textarea
    id: alternatives
    attributes:
      label: Alternatives considered
      description: Any other approaches you've thought about.
  - type: dropdown
    id: area
    attributes:
      label: Area
      options:
        - Lexer
        - Parser / Grammar
        - AST
        - Diagnostics
        - CLI
        - Library API
        - Documentation
        - Other
```

- [ ] **Step 3: Create issue template config**

Create `.github/ISSUE_TEMPLATE/config.yml`:

```yaml
blank_issues_enabled: true
contact_links:
  - name: Compact Language (upstream)
    url: https://github.com/LFDT-Minokawa/compact
    about: For issues with the Compact language itself (not the parser)
```

- [ ] **Step 4: Create pull request template**

Create `.github/PULL_REQUEST_TEMPLATE.md`:

```markdown
## Summary

<!-- Brief description of what this PR does and why -->

## Changes

<!-- Bulleted list of key changes -->

-

## Test plan

<!-- How were these changes tested? -->

- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace -- -D warnings` clean
- [ ] `cargo fmt --all -- --check` clean

## Related issues

<!-- Link any related issues: Fixes #123, Relates to #456 -->
```

- [ ] **Step 5: Create CONTRIBUTING.md**

The contributing guide should cover:

1. **Development setup** — clone, install Rust, `cargo build --workspace`
2. **Running tests** — `cargo test --workspace`, corpus tests, benchmarks
3. **Code style** — enforced by `cargo fmt`, `cargo clippy -D warnings`; lefthook runs checks on commit/push
4. **Commit conventions** — Conventional Commits format, body under 500 chars / 10 lines (enforced by lefthook commit-msg hook)
5. **PR workflow** — fork, branch, implement with tests, submit PR against `main`
6. **Adding a new grammar construct** — where to add the SyntaxKind, grammar function, AST node, and tests
7. **Test fixtures** — how to add corpus files and snapshot tests; note Apache-2.0 license on upstream corpus files
8. **Release process** — managed by release-plz (automated version bumps, changelogs, crate publishing)

- [ ] **Step 6: Create CODE_OF_CONDUCT.md**

Adopt the Contributor Covenant v2.1. Include the standard text with project-specific contact information.

- [ ] **Step 7: Create SECURITY.md**

Include:
- Supported versions (latest release)
- How to report vulnerabilities (private email or GitHub security advisories)
- Expected response timeline
- Note: compactp is a parser — it processes untrusted input. Report any input that causes panics, excessive memory use, or hangs.

- [ ] **Step 8: Commit**

```bash
git add .github/ISSUE_TEMPLATE/ .github/PULL_REQUEST_TEMPLATE.md CONTRIBUTING.md CODE_OF_CONDUCT.md SECURITY.md
git commit -m "docs: add GitHub community docs, issue/PR templates, and contributing guide"
```

---

### Task 28: Final Polish & Cleanup

- [ ] **Step 1: Run full test suite**

Run: `cargo test --workspace`
Expected: all tests pass

- [ ] **Step 2: Run clippy**

Run: `cargo clippy --workspace -- -D warnings`
Expected: no warnings

- [ ] **Step 3: Verify documentation**

Run: `cargo doc --workspace --no-deps`
Expected: builds without errors. All public types have doc comments.

- [ ] **Step 4: Verify the binary works end-to-end**

```bash
cargo run -- parse tests/corpus/tiny.compact
cargo run -- lex tests/corpus/tiny.compact --format json
cargo run -- cst tests/corpus/tiny.compact
cargo run -- ast tests/corpus/tiny.compact
cargo run -- diag tests/corpus/tiny.compact
cargo run -- stats tests/corpus/tiny.compact
```

Expected: all commands produce sensible output

- [ ] **Step 5: Commit clippy fixes**

```bash
git add crates/
git commit -m "fix: resolve clippy warnings across all crates"
```

- [ ] **Step 6: Commit documentation**

```bash
git add crates/
git commit -m "docs: add doc comments to all public types and functions"
```
