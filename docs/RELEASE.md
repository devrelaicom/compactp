# Release runbook — compactp

> **This runbook contains irreversible actions** (`cargo publish`, `git push --tags`).
> A crates.io publish cannot be undone — `yank` only hides a version from new
> resolution; the version number is burned forever. Do not run any step in §3
> until every precondition in §1 is green. Each §3 step is a deliberate,
> human-confirmed action.

This runbook was validated by a non-publishing rehearsal (WS4 T12): `release-plz`
and `cargo dist` were exercised locally and the findings below are folded in.

## 1. Preconditions (all must be GREEN before §3)

- [ ] **7-night fuzz gate.** The `fuzz-nightly` workflow has run for **7
      consecutive nights with zero crashes/timeouts**. Verify:
      `gh run list --workflow=fuzz-nightly.yml --limit 10` — the last 7 nightly
      runs are all green. This is a calendar gate; it cannot be shortcut.
- [ ] **All CI green on the release branch**: `ci.yml` (check + 3-OS build + MSRV
      `1.90`), `docs.yml` (`cargo doc` with `-D warnings`), `public-api.yml`
      (`cargo public-api --simplified` diff).
- [ ] **Workspace packaging is clean.** Run the workspace-aware pre-flight:
      `cargo package --workspace` — must exit 0 (it packages all six crates and
      verify-builds them against each other in a temp registry).
      NOTE: per-crate `cargo publish --dry-run -p <crate>` only succeeds for a
      crate whose dependencies are **already live on crates.io**. Before the
      first publish only the leaf (`compactp_syntax`) can be `--dry-run`'d in
      isolation; `cargo package --workspace` is the real pre-flight for the rest.
- [ ] **`dist build` green** locally for the host target
      (`dist build --artifacts=local --target $(rustc -vV | sed -n 's/host: //p')`)
      and on CI for all four targets (the `Release` workflow on the release PR).
- [ ] **Working tree is clean of stray fuzz corpus.** Local fuzz runs leave
      thousands of untracked files under `fuzz/corpus/`. **`release-plz` and
      `cargo publish` both refuse to run against a dirty working tree**, so this
      is load-bearing, not cosmetic. Inspect and clear:
      `git clean -nx fuzz/corpus/` (dry-run — review the list), then
      `git clean -fx fuzz/corpus/` to remove only the throwaway local fuzz
      output. Confirm `git status --porcelain` shows nothing unexpected before
      releasing. The committed 50+50 seed corpus is tracked and survives the
      clean.
- [ ] **`CHANGELOG.md` `0.1.0-beta.1` entry is final.** It is a single root
      changelog (release-plz is configured to aggregate all six crates here; it
      does NOT create per-crate changelog files — see §4).
- [ ] **Compatibility matrix in `README.md` is correct**: `0.1.0-beta.1` /
      `>= 0.23` / compiler `0.31.0` / schema `1` / MSRV `1.90`. The CLI emits
      `language_version "0.23.0"` (confirmed via `compact compile
      --language-version`).
- [ ] **`CARGO_REGISTRY_TOKEN`** is set in the repo's Actions secrets (used by
      the `release-plz release` job). Verify: `gh secret list`.

## 2. Publish order (dependency order — DO NOT deviate)

`cargo publish` requires each dependency to already be live on crates.io. The
order below is dependency-correct (`compactp_syntax` has no internal deps and
MUST go first):

1. `compactp_syntax`       (no internal deps)
2. `compactp_diagnostics`  (→ syntax)
3. `compactp_lexer`        (→ syntax)
4. `compactp_ast`          (→ syntax)
5. `compactp_parser`       (→ syntax, lexer, diagnostics)
6. `compactp`              (→ all five)

(Steps 2–4 may be done in any order among themselves; only the syntax-first and
parser-before-CLI constraints are hard.)

## 3. The release (irreversible — human-confirmed, one step at a time)

The repo uses **release-plz** to automate this. Rehearsal confirmed release-plz
keeps the version at `0.1.0-beta.1` for all six crates (no `set-version`
override needed).

1. **Merge the release-plz PR.** On push to `main`, the `release-plz-pr` job
   opens (or updates) a "release" PR. Review it against §1. The version should
   read `0.1.0-beta.1`; if release-plz ever proposes a different version,
   correct it with `release-plz set-version 0.1.0-beta.1` before merging.
2. **Merge triggers `release-plz release`**, which publishes the six crates in
   dependency order and pushes a single git tag `compactp-v<version>`.
   - **Tagging is configured so only `compactp` is tagged.** The five library
     crates set `git_tag_enable = false` in `release-plz.toml`; they publish to
     crates.io but are not tagged. This avoids release-plz's default of one tag
     per crate, which would make cargo-dist's `release.yml` spin up a failing
     run for each non-dist-able library tag. Do NOT instead try to narrow the
     `release.yml` tag glob — cargo-dist verifies the committed `release.yml`
     matches `dist generate` output and the `plan` job fails on any manual edit.
3. **The `compactp` tag triggers `release.yml`** (cargo dist), which builds the
   four target binaries + checksums and creates the GitHub Release with the
   CHANGELOG excerpt.
   - Note: `release-plz` is configured with `git_release_enable = false` so it
     does NOT create a GitHub release — cargo-dist owns release creation (it has
     the binaries). Do not re-enable it, or the two will collide on the same tag.
4. **Verify on crates.io**: all six crates show `0.1.0-beta.1`.
5. **Verify the GitHub Release**: four binary archives + `.sha256` files
   attached (`aarch64-apple-darwin`, `x86_64-apple-darwin`,
   `x86_64-unknown-linux-gnu`, `x86_64-pc-windows-msvc`).
6. **Announce.**

### Manual fallback (if release-plz automation fails mid-way)

Publish manually in the §2 order from a CLEAN working tree:

```bash
git clean -fx fuzz/corpus/            # clear stray local fuzz output first
cargo package --workspace             # pre-flight: must be green
for c in compactp_syntax compactp_diagnostics compactp_lexer compactp_ast compactp_parser compactp; do
  cargo publish -p "$c"               # NO --dry-run. Confirm success on crates.io before the next.
  echo "Published $c — waiting for the crates.io index to update before the next crate..."
  sleep 30
done
git tag compactp-v0.1.0-beta.1
git push origin compactp-v0.1.0-beta.1   # triggers release.yml (dist binaries + GitHub release)
```

> The `sleep 30` between crates lets the crates.io index update so the next
> crate's `=0.1.0-beta.1` dependency resolves. If a publish fails because the
> index hasn't caught up, wait longer and retry that crate.

## 4. release-plz configuration notes (from the WS4 rehearsal)

`release-plz.toml` was tuned during the rehearsal:

- **Single root changelog.** The `compactp` package owns `CHANGELOG.md` (via
  `changelog_path = "CHANGELOG.md"`) and aggregates the five libraries with
  `changelog_include`. The five libraries set `changelog_update = false`, so
  release-plz does NOT scatter per-crate `crates/*/CHANGELOG.md` files (its
  default behavior, which conflicts with the README's single-changelog link).
- **`git_release_enable = false`** — cargo-dist owns the GitHub release (§3.3).
- **`git_tag_enable = true`**, **`semver_check = true`** retained.

Three files that were previously both git-tracked AND gitignored
(`LS.md` and two `docs/superpowers/` design docs) were untracked during WS4
because release-plz refuses to run while such ambiguity exists. `LS.md` is no
longer in the repo; deviation rationale now lives inline in
`tests/corpus_known_failures.txt`.

## 5. Post-release

- [ ] Confirm the `## [Unreleased]` section in `CHANGELOG.md` is ready for the
      next cycle (release-plz prepends future entries above `0.1.0-beta.1`).
- [ ] Open a tracking issue for `0.2` scope (LSP, formatter — out of beta scope).
