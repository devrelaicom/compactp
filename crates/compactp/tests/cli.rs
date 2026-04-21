//! Integration tests for the compactp binary.
//!
//! Tests are grouped into:
//! - happy-path output: one human + one JSON snapshot per subcommand
//! - exit codes: 0, 1, 2, 3 verified against curated fixtures
//! - help / version: non-zero-friendly exits
//! - flag honoring: --max-diagnostics, --color (including 0-cap and auto)
//! - panic-resistance: invalid UTF-8, missing file, empty directory
//! - JSON envelope invariant: every subcommand's JSON has the same shape
//! - watch mode: working path syntax and initial-run behaviour
//!
//! Snapshots live in `tests/snapshots/`. They snapshot the exact pretty-printed
//! JSON stdout (not a round-tripped `serde_json::Value`) so field order,
//! duplicate keys, and numeric formatting are all covered by the contract.
//! After intentional changes, regenerate with
//! `cargo insta test --accept -p compactp --test cli`.

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
use std::io::Write;
use std::path::MAIN_SEPARATOR;
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

/// Normalise Windows backslashes to forward slashes inside JSON string values
/// so snapshots match across platforms.
fn normalise_path_separators(text: &str) -> String {
    if MAIN_SEPARATOR == '/' {
        return text.to_string();
    }
    text.replace('\\', "/")
}

fn assert_envelope(value: &Value, expected_input: &str) {
    let expected = expected_input.replace('\\', "/");
    assert_eq!(
        value.get("tool_version").and_then(Value::as_str),
        Some(env!("CARGO_PKG_VERSION"))
    );
    assert_eq!(value.get("schema_version").and_then(Value::as_u64), Some(1));
    assert_eq!(
        value.get("language_version").and_then(Value::as_str),
        Some("0.22.0")
    );
    let actual = value
        .get("input")
        .and_then(Value::as_str)
        .map(|s| s.replace('\\', "/"));
    assert_eq!(actual.as_deref(), Some(expected.as_str()));
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
    assert_snapshot!("diag_human_output", normalise_path_separators(&out));
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

/// Redact `"parse_time_ms": <number>` in raw JSON text so the stats snapshot
/// does not drift run-to-run. Works on the pretty-printed form only.
fn redact_parse_time_ms(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.lines() {
        if line.trim_start().starts_with("\"parse_time_ms\":") {
            let indent = &line[..line.find('"').expect("indent")];
            out.push_str(indent);
            out.push_str("\"parse_time_ms\": \"<redacted>\"");
            if line.trim_end().ends_with(',') {
                out.push(',');
            }
            out.push('\n');
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

fn snapshot_raw(name: &str, output: &std::process::Output) {
    let raw = String::from_utf8(output.stdout.clone()).expect("stdout utf8");
    assert_snapshot!(name, normalise_path_separators(&raw));
}

#[test]
fn lex_json_output() {
    let path = fixture("imports/all_import_forms.compact");
    let output = run_ok(&["--format", "json", "--pretty", "lex", &path]);
    assert_envelope(&parse_json(&output), &path);
    snapshot_raw("lex_json_output", &output);
}

#[test]
fn parse_json_output() {
    let path = fixture("demo/valid.compact");
    let output = run_ok(&["--format", "json", "--pretty", "parse", &path]);
    assert_envelope(&parse_json(&output), &path);
    snapshot_raw("parse_json_output", &output);
}

#[test]
fn cst_json_output() {
    let path = fixture("imports/all_import_forms.compact");
    let output = run_ok(&["--format", "json", "--pretty", "cst", &path]);
    assert_envelope(&parse_json(&output), &path);
    snapshot_raw("cst_json_output", &output);
}

#[test]
fn ast_json_output() {
    let path = fixture("declarations/all_declarations.compact");
    let output = run_ok(&["--format", "json", "--pretty", "ast", &path]);
    let json = parse_json(&output);
    assert_envelope(&json, &path);
    // Every Item variant in the fixture must appear in the dump.
    let kinds: std::collections::BTreeSet<String> = json["data"]["items"]
        .as_array()
        .expect("items array")
        .iter()
        .filter_map(|item| item["kind"].as_str().map(str::to_owned))
        .collect();
    for expected in [
        "Pragma",
        "Include",
        "Import",
        "ExportList",
        "LedgerDecl",
        "ConstructorDef",
        "CircuitDef",
        "CircuitDecl",
        "WitnessDecl",
        "ContractDecl",
        "StructDef",
        "EnumDef",
        "ModuleDef",
        "TypeDecl",
    ] {
        assert!(
            kinds.contains(expected),
            "ast dump missing {expected}; got {kinds:?}"
        );
    }
    snapshot_raw("ast_json_output", &output);
}

#[test]
fn diag_json_output() {
    let path = fixture("recovery/broken_expressions.compact");
    let output = run_expect_code(&["--format", "json", "--pretty", "diag", &path], 1);
    assert_envelope(&parse_json(&output), &path);
    snapshot_raw("diag_json_output", &output);
}

#[test]
fn diag_json_code_is_structured_object() {
    let path = fixture("recovery/broken_expressions.compact");
    let output = run_expect_code(&["--format", "json", "--pretty", "diag", &path], 1);
    let json = parse_json(&output);
    let first = &json["data"][0];
    let code = first.get("code").expect("diagnostic has code");
    assert!(
        code.is_object(),
        "diagnostic code must be structured object per README; got {code}"
    );
    assert!(code.get("prefix").and_then(Value::as_str).is_some());
    assert!(code.get("number").and_then(Value::as_u64).is_some());
}

#[test]
fn stats_json_output() {
    let path = fixture("imports/all_import_forms.compact");
    let output = run_ok(&["--format", "json", "--pretty", "stats", &path]);
    assert_envelope(&parse_json(&output), &path);
    let raw = String::from_utf8(output.stdout.clone()).expect("utf8");
    let redacted = redact_parse_time_ms(&normalise_path_separators(&raw));
    assert_snapshot!("stats_json_output", redacted);
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
fn diag_max_diagnostics_zero_still_signals_failure() {
    let path = fixture("recovery/broken_expressions.compact");
    run_expect_code(&["diag", &path, "--max-diagnostics", "0"], 1);
}

#[test]
fn parse_json_max_diagnostics_zero_reports_truncation() {
    let path = fixture("recovery/broken_expressions.compact");
    let output = run_expect_code(
        &["--format", "json", "parse", &path, "--max-diagnostics", "0"],
        1,
    );
    let json = parse_json(&output);
    let data = &json["data"];
    assert_eq!(data["success"].as_bool(), Some(false));
    assert!(
        data["error_count"].as_u64().unwrap_or(0) > 0,
        "error_count must reflect real error count, not the cap"
    );
    assert_eq!(data["truncated"].as_bool(), Some(true));
    assert_eq!(
        data["diagnostics"].as_array().map(Vec::len),
        Some(0),
        "diagnostics array is capped at 0"
    );
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

#[test]
fn color_auto_without_terminal_defaults_to_never() {
    // assert_cmd spawns compactp with a non-tty stdout (a pipe); under --color
    // auto the binary must therefore emit no ANSI escapes.
    let path = fixture("recovery/broken_expressions.compact");
    let output = run_expect_code(&["diag", "--color", "auto", &path], 1);
    let s = stdout(output);
    assert!(
        !s.contains('\x1b'),
        "--color auto on a non-tty pipe should suppress ANSI: {s:?}"
    );
}

// ---------------------------------------------------------------------------
// Panic-resistance
// ---------------------------------------------------------------------------

#[test]
fn invalid_utf8_input_does_not_panic() {
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

// ---------------------------------------------------------------------------
// Watch mode
// ---------------------------------------------------------------------------

#[test]
fn watch_parse_accepts_paths_and_runs_once() {
    // `compactp watch parse <path>` must accept the path and emit the initial
    // run before we signal it. Without this test the clap subcommand/positional
    // ordering regression goes undetected.
    let dir = tempfile::tempdir().expect("tempdir");
    let file_path = dir.path().join("probe.compact");
    {
        let mut f = fs::File::create(&file_path).expect("create fixture");
        f.write_all(b"circuit nothing(): Field { return 0 as Field; }\n")
            .expect("write");
    }

    let binary = assert_cmd::cargo::cargo_bin("compactp");
    let mut child = std::process::Command::new(binary)
        .args(["watch", "parse", file_path.to_str().unwrap()])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn watch");

    // Give the initial run time to complete. 500ms is generous — the watch
    // implementation calls run_watchable synchronously before the change loop.
    std::thread::sleep(std::time::Duration::from_millis(500));
    let _ = child.kill();
    let output = child.wait_with_output().expect("reap");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("probe.compact") && stdout.contains("OK"),
        "watch did not perform an initial run: stdout={stdout:?}"
    );
}

// ---------------------------------------------------------------------------
// Mixed-source edge cases
// ---------------------------------------------------------------------------

#[test]
fn mixing_stdin_with_file_path_is_usage_error() {
    // "-" means "read stdin"; combining it with a file path is ambiguous and
    // must be rejected rather than silently discarding one side.
    let path = fixture("demo/valid.compact");
    let output = bin()
        .args(["parse", "-", &path])
        .write_stdin("")
        .output()
        .expect("spawn");
    assert_eq!(output.status.code(), Some(3));
}
