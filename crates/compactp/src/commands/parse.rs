use crate::Cli;
use crate::error::CliError;
use crate::input::resolve_inputs;
use crate::output::OutputEnvelope;
use compactp_parser::{ParseOptions, parse_with};
use serde::Serialize;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Debug, Clone, Serialize)]
struct ParseInfo {
    success: bool,
    error_count: usize,
    errors: Vec<String>,
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

        let has_errors = !result.errors.is_empty();
        if has_errors {
            worst = worst.max(1);
        }

        match cli.format {
            crate::OutputFormat::Json => {
                let timing_ms = cli.timing.then_some(elapsed.as_secs_f64() * 1000.0);
                let info = ParseInfo {
                    success: !has_errors,
                    error_count: result.errors.len(),
                    errors: result.errors.iter().map(|d| d.message.clone()).collect(),
                };
                let envelope = OutputEnvelope::new(input.label.clone(), info, timing_ms);
                crate::output::print_json(&envelope, cli.pretty)?;
            }
            crate::OutputFormat::Human => {
                if has_errors {
                    for err in &result.errors {
                        eprintln!("error: {}", err.message);
                    }
                    eprintln!(
                        "{} error{} found",
                        result.errors.len(),
                        if result.errors.len() == 1 { "" } else { "s" }
                    );
                } else {
                    println!("{}: OK", input.label);
                }
                if cli.timing {
                    eprintln!("Parsed in {:.2}ms", elapsed.as_secs_f64() * 1000.0);
                }
            }
        }
    }

    Ok(worst)
}
