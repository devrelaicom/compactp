use crate::Cli;
use crate::error::CliError;
use crate::input::resolve_inputs;
use crate::output::OutputEnvelope;
use compactp_diagnostics::render_human;
use compactp_parser::{ParseOptions, parse_with};
use serde::Serialize;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Debug, Clone, Serialize)]
struct ParseData {
    success: bool,
    error_count: usize,
    diagnostics: Vec<compactp_diagnostics::Diagnostic>,
}

pub fn run(cli: &Cli, paths: &[PathBuf]) -> Result<i32, CliError> {
    let inputs = resolve_inputs(paths, cli.stdin_filename.as_deref())?;
    let mut worst = 0;

    for input in inputs {
        let opts = ParseOptions {
            recover: !cli.no_recover,
            max_errors: cli.max_errors.unwrap_or(256),
        };

        let start = Instant::now();
        let result = parse_with(&input.source, opts);
        let elapsed = start.elapsed();

        let success = result.errors.is_empty();
        if !success {
            worst = worst.max(1);
        }

        let diagnostics = limit_diagnostics(result.errors, cli.max_diagnostics);

        match cli.format {
            crate::OutputFormat::Json => {
                let timing_ms = cli.timing.then_some(elapsed.as_secs_f64() * 1000.0);
                let data = ParseData {
                    success,
                    error_count: diagnostics.len(),
                    diagnostics,
                };
                let envelope = OutputEnvelope::new(input.label.clone(), data, timing_ms);
                crate::output::print_json(&envelope, cli.pretty)?;
            }
            crate::OutputFormat::Human => {
                if success {
                    println!("{}: OK", input.label);
                } else {
                    let colored = matches!(cli.color, crate::ColorChoice::Always);
                    for diag in &diagnostics {
                        print!(
                            "{}",
                            render_human(diag, &input.source, &input.label, colored)
                        );
                    }
                    eprintln!(
                        "{} error{} found",
                        diagnostics.len(),
                        if diagnostics.len() == 1 { "" } else { "s" }
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
) -> Vec<compactp_diagnostics::Diagnostic> {
    match max_diagnostics {
        Some(limit) => diagnostics.into_iter().take(limit).collect(),
        None => diagnostics,
    }
}
