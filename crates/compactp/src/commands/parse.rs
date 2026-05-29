use crate::Cli;
use crate::error::CliError;
use crate::input::resolve_inputs;
use crate::output::OutputEnvelope;
use compactp_diagnostics::{render_human, render_json};
use compactp_parser::{ParseOptions, parse_with};
use serde::Serialize;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Debug, Clone, Serialize)]
struct ParseData {
    success: bool,
    error_count: usize,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    truncated: bool,
    diagnostics: Vec<serde_json::Value>,
}

pub fn run(cli: &Cli, paths: &[PathBuf]) -> Result<i32, CliError> {
    let inputs = resolve_inputs(paths, cli.stdin_filename.as_deref())?;
    let mut worst = 0;

    for input in inputs {
        let opts = ParseOptions {
            recover: !cli.no_recover,
            max_errors: cli.max_errors.unwrap_or(256),
            ..ParseOptions::default()
        };

        let start = Instant::now();
        let result = parse_with(&input.source, opts);
        let elapsed = start.elapsed();

        let total_errors = result.errors.len();
        let success = total_errors == 0;
        if !success {
            worst = worst.max(1);
        }

        let (diagnostics, truncated) = limit_diagnostics(result.errors, cli.max_diagnostics);

        match cli.format {
            crate::OutputFormat::Json => {
                // Diagnostics flow through the same `render_json` renderer as
                // the `diag` command so both subcommands publish the same
                // per-diagnostic shape (structured `code`, line/column-resolved
                // spans, and note list).
                let rendered: Vec<serde_json::Value> = diagnostics
                    .iter()
                    .map(|d| render_json(d, &input.source))
                    .collect();
                let timing_ms = cli.timing.then_some(elapsed.as_secs_f64() * 1000.0);
                let data = ParseData {
                    success,
                    error_count: total_errors,
                    truncated,
                    diagnostics: rendered,
                };
                let envelope = OutputEnvelope::new(input.label.clone(), data, timing_ms);
                crate::output::print_json(&envelope, cli.pretty)?;
            }
            crate::OutputFormat::Human => {
                if success {
                    println!("{}: OK", input.label);
                } else {
                    let colored = crate::output::use_color(cli.color);
                    for diag in &diagnostics {
                        print!(
                            "{}",
                            render_human(diag, &input.source, &input.label, colored)
                        );
                    }
                    eprintln!(
                        "{} error{} found{}",
                        total_errors,
                        if total_errors == 1 { "" } else { "s" },
                        if truncated {
                            format!(" (showing {} of {total_errors})", diagnostics.len())
                        } else {
                            String::new()
                        },
                    );
                }
                if cli.timing {
                    eprintln!("Parsed in {:.2}ms", elapsed.as_secs_f64() * 1000.0);
                }
            }
        }
    }

    Ok(worst)
}

fn limit_diagnostics(
    diagnostics: Vec<compactp_diagnostics::Diagnostic>,
    max_diagnostics: Option<usize>,
) -> (Vec<compactp_diagnostics::Diagnostic>, bool) {
    match max_diagnostics {
        Some(limit) if diagnostics.len() > limit => {
            (diagnostics.into_iter().take(limit).collect(), true)
        }
        _ => (diagnostics, false),
    }
}
