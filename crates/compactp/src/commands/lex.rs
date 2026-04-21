use crate::Cli;
use crate::error::CliError;
use crate::input::resolve_inputs;
use crate::output::OutputEnvelope;
use serde::Serialize;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Debug, Clone, Serialize)]
struct TokenInfo {
    kind: String,
    text: String,
    offset: u32,
    len: u32,
}

pub fn run(cli: &Cli, paths: &[PathBuf]) -> Result<i32, CliError> {
    let inputs = resolve_inputs(paths, cli.stdin_filename.as_deref())?;

    for input in inputs {
        let start = Instant::now();
        let tokens = compactp_lexer::lex(&input.source);
        let elapsed = start.elapsed();

        match cli.format {
            crate::OutputFormat::Json => {
                let mut offset = 0u32;
                let token_infos: Vec<TokenInfo> = tokens
                    .iter()
                    .map(|(kind, text)| {
                        let len = text.len() as u32;
                        let info = TokenInfo {
                            kind: format!("{kind:?}"),
                            text: text.to_string(),
                            offset,
                            len,
                        };
                        offset += len;
                        info
                    })
                    .collect();

                let timing_ms = cli.timing.then_some(elapsed.as_secs_f64() * 1000.0);
                let envelope = OutputEnvelope::new(input.label.clone(), token_infos, timing_ms);
                crate::output::print_json(&envelope, cli.pretty)?;
            }
            crate::OutputFormat::Human => {
                let mut offset = 0usize;
                for (kind, text) in &tokens {
                    let len = text.len();
                    println!("{kind:?} {offset}..{} {text:?}", offset + len);
                    offset += len;
                }
                if cli.timing {
                    eprintln!("Lexed in {:.2}ms", elapsed.as_secs_f64() * 1000.0);
                }
            }
        }
    }

    Ok(0)
}
