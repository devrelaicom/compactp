use crate::Cli;
use crate::error::CliError;
use crate::input::resolve_inputs;
use compactp_diagnostics::{render_human, render_json};
use compactp_parser::{ParseOptions, parse_with};
use serde::Serialize;
use std::path::PathBuf;

/// JSON body for the `diag` subcommand. Mirrors the shape used by `parse` so
/// downstream consumers can handle both commands through one reader.
#[derive(Debug, Clone, Serialize)]
struct DiagData {
    error_count: usize,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    truncated: bool,
    diagnostics: Vec<serde_json::Value>,
}

pub fn run(cli: &Cli, paths: &[PathBuf]) -> Result<i32, CliError> {
    let inputs = resolve_inputs(paths, cli.stdin_filename.as_deref())?;
    let mut had_errors = false;

    for input in inputs {
        let result = parse_with(
            &input.source,
            ParseOptions {
                recover: !cli.no_recover,
                max_errors: cli.max_errors.unwrap_or(256),
            },
        );

        // Lock the exit-code signal before any user-supplied cap can erase it.
        had_errors |= !result.errors.is_empty();

        let total = result.errors.len();
        let (diagnostics, truncated) = match cli.max_diagnostics {
            Some(limit) if total > limit => (
                result
                    .errors
                    .iter()
                    .take(limit)
                    .cloned()
                    .collect::<Vec<_>>(),
                true,
            ),
            _ => (result.errors, false),
        };

        match cli.format {
            crate::OutputFormat::Human => {
                let colored = crate::output::use_color(cli.color);
                for d in &diagnostics {
                    print!("{}", render_human(d, &input.source, &input.label, colored));
                }
                if truncated {
                    eprintln!(
                        "{} error{} found (showing {} of {})",
                        total,
                        if total == 1 { "" } else { "s" },
                        diagnostics.len(),
                        total,
                    );
                }
            }
            crate::OutputFormat::Json => {
                let rendered: Vec<serde_json::Value> = diagnostics
                    .iter()
                    .map(|d| render_json(d, &input.source))
                    .collect();
                let data = DiagData {
                    error_count: total,
                    truncated,
                    diagnostics: rendered,
                };
                let envelope = crate::output::OutputEnvelope::new(input.label.clone(), data, None);
                crate::output::print_json(&envelope, cli.pretty)?;
            }
        }
    }

    Ok(if had_errors { 1 } else { 0 })
}
