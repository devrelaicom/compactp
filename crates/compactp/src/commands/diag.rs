use crate::Cli;
use crate::error::CliError;
use crate::input::resolve_inputs;
use compactp_diagnostics::{render_human, render_json};
use compactp_parser::{ParseOptions, parse_with};
use std::path::PathBuf;

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

        let (diagnostics, _truncated) = match cli.max_diagnostics {
            Some(limit) if result.errors.len() > limit => (
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
            }
            crate::OutputFormat::Json => {
                let data: Vec<_> = diagnostics
                    .iter()
                    .map(|d| render_json(d, &input.source))
                    .collect();
                let envelope = crate::output::OutputEnvelope::new(input.label.clone(), data, None);
                crate::output::print_json(&envelope, cli.pretty)?;
            }
        }
    }

    Ok(if had_errors { 1 } else { 0 })
}
