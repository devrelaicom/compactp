## Summary

<!-- Brief description of what this PR does and why. -->

## Changes

<!-- Bulleted list of key changes. -->

-

## Test plan

<!-- How were these changes tested? Check every box before requesting review. -->

- [ ] `cargo fmt --all -- --check` clean
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] `cargo test --workspace` passes
- [ ] Lossless corpus invariant still holds (`cargo test -p compactp_parser --test corpus_test`)
- [ ] CLI snapshots regenerated intentionally (`cargo insta test --accept -p compactp --test cli`) — only if JSON or human output changed

## Public surface

<!-- Check any that apply. -->

- [ ] Changes CLI output (human or JSON)
- [ ] Changes JSON schema (`data` shape or envelope fields)
- [ ] Changes library API (`compactp_parser`, `compactp_ast`, `compactp_diagnostics`, `compactp_syntax`)
- [ ] Adds or changes a `SyntaxKind` variant
- [ ] None of the above

## Related issues

<!-- Fixes #123, Relates to #456 -->
