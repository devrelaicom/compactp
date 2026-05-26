# Upstream Source Versions

This file records the exact upstream commits that compactp's grammar
and corpus are validated against. It is the auditable counterpart to
the README's compatibility matrix.

The actual reference clones are not stored in this repository; they
live at `/tmp/compactp/references/` per the project convention (see
`docs/superpowers/plans/2026-03-20-compactp-implementation.md`).

## Pinned commits as of 2026-05-26

### LFDT-Minokawa/compact

- Repository: <https://github.com/LFDT-Minokawa/compact>
- Tag: `compactc-v0.31.0`
- Resolved sha: `878045a9746cf2de8287abbbbe23dd9f0f547571`
- Commit date: `2026-04-29T19:35:23+02:00`
- Targeted compiler version: `0.31.0` (tag name)
- In-source compiler version string: `0.31.101` (per
  `compiler/compiler-version.ss:23`). Upstream uses a `.101` suffix
  convention on in-source version literals; the public release tag is
  the canonical version. Downstream users running `compactc --version`
  may see the `.101` form.
- Targeted Compact language version: `0.23.101` (per
  `compiler/language-version.ss:22`). This is the value users write in
  `pragma language_version >= X.Y.Z;`.

### midnightntwrk/compact-tree-sitter

- Repository: <https://github.com/midnightntwrk/compact-tree-sitter>
- Identifier: latest `main` HEAD (the repo has zero tags)
- Resolved sha: `6693aff01384a8cf1d928d830f67978c441a2506`
- Commit date: `2026-04-08T14:25:26+01:00`

Tree-sitter grammar work appears to lag the compiler — recent activity
on `main` is dependency/CI maintenance rather than grammar evolution.
This pin should be revisited if the grammar resumes active development.

## Refresh procedure

To refresh against a new upstream commit:

1. Run `git fetch origin --tags` in each clone under `/tmp/compactp/references/`.
2. `git checkout <new-tag-or-sha>` in each clone.
3. Re-run the corpus refresh task (WS1 Phase 1 Task 5).
4. Update this file with the new sha and date.
5. Re-categorize any new entries in `tests/corpus_known_failures.txt`.
