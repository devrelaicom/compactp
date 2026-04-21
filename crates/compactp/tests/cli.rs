//! Integration tests for the compactp binary.
//!
//! Tests are grouped into:
//! - happy-path output: one human + one JSON snapshot per subcommand
//! - exit codes: 0, 1, 2, 3 verified against curated fixtures
//! - help / version: non-zero-friendly exits
//! - flag honoring: --max-diagnostics, --color
//! - panic-resistance: invalid UTF-8, missing file, empty directory
//! - JSON envelope invariant: every subcommand's JSON has the same shape
//!
//! Snapshots live in `tests/snapshots/`. After intentional changes, regenerate
//! them with `cargo insta test --accept -p compactp --test cli`.

// Integration tests compile as a separate binary target, so the crate-level
// [lints.clippy] denies from Cargo.toml still apply. A panic in test code is
// how tests fail — exempt this file explicitly.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented
)]

use assert_cmd::Command;
use insta::assert_snapshot;
use serde_json::Value;
use std::fs;
use tempfile::NamedTempFile;

fn fixture(path: &str) -> String {
    format!("tests/fixtures/{path}")
}

fn bin() -> Command {
    let mut cmd = Command::cargo_bin("compactp").expect("compactp binary");
    cmd.current_dir(env!("CARGO_MANIFEST_DIR"));
    cmd
}

fn run_ok(args: &[&str]) -> std::process::Output {
    let output = bin().args(args).output().expect("spawn compactp");
    assert!(
        output.status.success(),
        "compactp {args:?} failed: status={:?} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn run_expect_code(args: &[&str], expected: i32) -> std::process::Output {
    let output = bin().args(args).output().expect("spawn compactp");
    assert_eq!(
        output.status.code(),
        Some(expected),
        "compactp {args:?} wrong exit: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    output
}

fn stdout(output: std::process::Output) -> String {
    String::from_utf8(output.stdout).expect("stdout is utf8")
}

fn parse_json(output: &std::process::Output) -> Value {
    serde_json::from_slice(&output.stdout).expect("stdout is JSON")
}

fn snapshot_json(name: &str, value: &Value) {
    let rendered = serde_json::to_string_pretty(value).expect("pretty json");
    assert_snapshot!(name, rendered);
}

fn assert_envelope(value: &Value, expected_input: &str) {
    assert_eq!(
        value.get("tool_version").and_then(Value::as_str),
        Some(env!("CARGO_PKG_VERSION"))
    );
    assert_eq!(value.get("schema_version").and_then(Value::as_u64), Some(1));
    assert_eq!(
        value.get("language_version").and_then(Value::as_str),
        Some("0.22.0")
    );
    assert_eq!(
        value.get("input").and_then(Value::as_str),
        Some(expected_input)
    );
    assert!(value.get("data").is_some(), "expected data field");
}

// ---------------------------------------------------------------------------
// Happy-path human output snapshots (one per subcommand)
// ---------------------------------------------------------------------------

#[test]
fn lex_human_output() {
    let path = fixture("imports/all_import_forms.compact");
    assert_snapshot!("lex_human_output", stdout(run_ok(&["lex", &path])));
}

#[test]
fn parse_human_ok() {
    let path = fixture("demo/valid.compact");
    let out = stdout(run_ok(&["parse", &path]));
    assert!(
        out.trim_end().ends_with(": OK"),
        "unexpected output: {out:?}"
    );
}

#[test]
fn cst_human_output() {
    let path = fixture("imports/all_import_forms.compact");
    assert_snapshot!("cst_human_output", stdout(run_ok(&["cst", &path])));
}

#[test]
fn ast_human_output() {
    let path = fixture("declarations/all_declarations.compact");
    assert_snapshot!("ast_human_output", stdout(run_ok(&["ast", &path])));
}

#[test]
fn diag_human_output() {
    let path = fixture("recovery/broken_expressions.compact");
    let out = stdout(run_expect_code(&["diag", "--color", "never", &path], 1));
    assert_snapshot!("diag_human_output", out);
}

#[test]
fn stats_human_output() {
    let path = fixture("imports/all_import_forms.compact");
    let raw = stdout(run_ok(&["stats", &path]));
    let redacted = redact_timing_line(&raw);
    assert_snapshot!("stats_human_output", redacted);
}

/// Replace the `Parse time: <number>ms` line with a stable placeholder so the
/// snapshot does not fluctuate run-to-run.
fn redact_timing_line(text: &str) -> String {
    text.lines()
        .map(|line| {
            if line.starts_with("Parse time:") {
                "Parse time:  <redacted>".to_string()
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

// ---------------------------------------------------------------------------
// Happy-path JSON output snapshots
// ---------------------------------------------------------------------------

#[test]
fn lex_json_output() {
    let path = fixture("imports/all_import_forms.compact");
    let output = run_ok(&["--format", "json", "--pretty", "lex", &path]);
    let json = parse_json(&output);
    assert_envelope(&json, &path);
    snapshot_json("lex_json_output", &json);
}

#[test]
fn parse_json_output() {
    let path = fixture("demo/valid.compact");
    let output = run_ok(&["--format", "json", "--pretty", "parse", &path]);
    let json = parse_json(&output);
    assert_envelope(&json, &path);
    snapshot_json("parse_json_output", &json);
}

#[test]
fn cst_json_output() {
    let path = fixture("imports/all_import_forms.compact");
    let output = run_ok(&["--format", "json", "--pretty", "cst", &path]);
    let json = parse_json(&output);
    assert_envelope(&json, &path);
    snapshot_json("cst_json_output", &json);
}

#[test]
fn ast_json_output() {
    let path = fixture("declarations/all_declarations.compact");
    let output = run_ok(&["--format", "json", "--pretty", "ast", &path]);
    let json = parse_json(&output);
    assert_envelope(&json, &path);
    snapshot_json("ast_json_output", &json);
}

#[test]
fn diag_json_output() {
    let path = fixture("recovery/broken_expressions.compact");
    let output = run_expect_code(&["--format", "json", "--pretty", "diag", &path], 1);
    let json = parse_json(&output);
    assert_envelope(&json, &path);
    snapshot_json("diag_json_output", &json);
}

#[test]
fn stats_json_output() {
    let path = fixture("imports/all_import_forms.compact");
    let output = run_ok(&["--format", "json", "--pretty", "stats", &path]);
    let mut json = parse_json(&output);
    assert_envelope(&json, &path);
    // parse_time_ms varies run-to-run — redact before snapshotting.
    if let Some(obj) = json.get_mut("data").and_then(|data| data.as_object_mut())
        && obj.contains_key("parse_time_ms")
    {
        obj["parse_time_ms"] = Value::from("<redacted>");
    }
    snapshot_json("stats_json_output", &json);
}

// ---------------------------------------------------------------------------
// Exit codes
// ---------------------------------------------------------------------------

#[test]
fn parse_exit_zero_on_success() {
    let path = fixture("demo/valid.compact");
    run_expect_code(&["parse", &path], 0);
}

#[test]
fn parse_exit_one_on_errors() {
    let path = fixture("demo/invalid.compact");
    run_expect_code(&["parse", &path], 1);
}

#[test]
fn missing_file_exits_io_error() {
    run_expect_code(&["parse", "does-not-exist.compact"], 2);
}

#[test]
fn invalid_flag_exits_usage_error() {
    run_expect_code(&["--nonexistent-flag"], 3);
}

#[test]
fn watch_without_paths_is_usage_error() {
    run_expect_code(&["watch", "parse"], 3);
}

// ---------------------------------------------------------------------------
// Help / version
// ---------------------------------------------------------------------------

#[test]
fn help_exits_zero() {
    let output = run_expect_code(&["--help"], 0);
    let s = stdout(output);
    // Every subcommand surface should be advertised in --help.
    for cmd in ["lex", "parse", "cst", "ast", "diag", "stats", "watch"] {
        assert!(s.contains(cmd), "--help missing {cmd}");
    }
}

#[test]
fn version_prints_pkg_version() {
    let output = run_expect_code(&["--version"], 0);
    assert!(stdout(output).contains(env!("CARGO_PKG_VERSION")));
}

// ---------------------------------------------------------------------------
// Flag honoring
// ---------------------------------------------------------------------------

#[test]
fn parse_respects_max_diagnostics_human() {
    let path = fixture("recovery/missing_semicolons.compact");
    let output = run_expect_code(&["parse", &path, "--max-diagnostics", "1"], 1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let count = stdout.matches("error[E").count();
    assert_eq!(
        count, 1,
        "expected exactly one error line under --max-diagnostics 1, got {count}: {stdout}"
    );
}

#[test]
fn diag_json_respects_max_diagnostics() {
    let path = fixture("recovery/broken_expressions.compact");
    let output = run_expect_code(
        &[
            "--format",
            "json",
            "--pretty",
            "diag",
            &path,
            "--max-diagnostics",
            "1",
        ],
        1,
    );
    let json = parse_json(&output);
    let arr = json["data"].as_array().expect("data is array");
    assert_eq!(arr.len(), 1, "expected one diagnostic, got {arr:?}");
}

#[test]
fn color_never_suppresses_ansi() {
    let path = fixture("recovery/broken_expressions.compact");
    let output = run_expect_code(&["diag", "--color", "never", &path], 1);
    let s = stdout(output);
    assert!(
        !s.contains('\x1b'),
        "--color never should suppress ANSI escapes: {s:?}"
    );
}

#[test]
fn color_always_emits_ansi() {
    let path = fixture("recovery/broken_expressions.compact");
    let output = run_expect_code(&["diag", "--color", "always", &path], 1);
    let s = stdout(output);
    assert!(
        s.contains('\x1b'),
        "--color always should emit ANSI escapes: {s:?}"
    );
}

// ---------------------------------------------------------------------------
// Panic-resistance
// ---------------------------------------------------------------------------

#[test]
fn invalid_utf8_input_does_not_panic() {
    // Write non-UTF8 bytes to a temp file and confirm compactp exits cleanly
    // (non-zero, but Some(_) — i.e. not aborted by a panic/signal).
    let mut tmp = NamedTempFile::with_suffix(".compact").expect("tempfile");
    std::io::Write::write_all(tmp.as_file_mut(), &[0xFF, 0xFE, 0xFD, b'a']).expect("write");
    let path = tmp.path().to_string_lossy().to_string();

    let output = bin()
        .args(["parse", &path])
        .output()
        .expect("spawn compactp");
    assert!(
        output.status.code().is_some(),
        "process was terminated by signal/panic"
    );
    assert_ne!(output.status.code(), Some(0));
}

#[test]
fn empty_directory_exits_ok() {
    let dir = tempfile::tempdir().expect("tempdir");
    run_expect_code(&["parse", dir.path().to_str().unwrap()], 0);
}

// ---------------------------------------------------------------------------
// JSON envelope invariant — every subcommand's envelope has the same shape
// ---------------------------------------------------------------------------

#[test]
fn every_subcommand_emits_versioned_envelope() {
    let path = fixture("demo/valid.compact");
    for cmd in ["lex", "parse", "cst", "ast", "diag", "stats"] {
        let output = bin()
            .args(["--format", "json", cmd, &path])
            .output()
            .expect("spawn");
        // diag on a clean file emits an envelope with an empty array — still JSON.
        let json: Value = serde_json::from_slice(&output.stdout)
            .unwrap_or_else(|err| panic!("{cmd} did not emit valid JSON: {err}"));
        assert_envelope(&json, &path);
    }
}

// ---------------------------------------------------------------------------
// Stdin read path
// ---------------------------------------------------------------------------

#[test]
fn stdin_with_stdin_filename_label() {
    let path = fixture("demo/valid.compact");
    let source = fs::read_to_string(&path).expect("read fixture");

    let output = bin()
        .args([
            "--stdin-filename",
            "piped.compact",
            "--format",
            "json",
            "parse",
        ])
        .write_stdin(source)
        .output()
        .expect("spawn");
    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(json["input"].as_str(), Some("piped.compact"));
}
