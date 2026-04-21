use crate::Cli;
use crate::commands::cst::root_from_green;
use crate::error::CliError;
use crate::input::resolve_inputs;
use crate::output::OutputEnvelope;
use compactp_syntax::SyntaxNode;
use serde::Serialize;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Debug, Clone, Serialize)]
struct Stats {
    file_size_bytes: usize,
    token_count: usize,
    node_count: usize,
    error_count: usize,
    parse_time_ms: f64,
}

pub fn run(cli: &Cli, paths: &[PathBuf]) -> Result<i32, CliError> {
    let inputs = resolve_inputs(paths, cli.stdin_filename.as_deref())?;
    let mut worst = 0;

    for input in inputs {
        let file_size = input.source.len();
        let token_count = compactp_lexer::lex(&input.source).len();

        let start = Instant::now();
        let result = compactp_parser::parse(&input.source);
        let parse_time = start.elapsed();

        let root = root_from_green(result.green);
        let node_count = count_nodes(&root);
        let error_count = result.errors.len();

        let stats = Stats {
            file_size_bytes: file_size,
            token_count,
            node_count,
            error_count,
            parse_time_ms: parse_time.as_secs_f64() * 1000.0,
        };

        match cli.format {
            crate::OutputFormat::Json => {
                let timing_ms = cli.timing.then_some(parse_time.as_secs_f64() * 1000.0);
                let envelope = OutputEnvelope::new(input.label.clone(), stats, timing_ms);
                crate::output::print_json(&envelope, cli.pretty)?;
            }
            crate::OutputFormat::Human => {
                println!("File:        {}", input.label);
                println!("Size:        {file_size} bytes");
                println!("Tokens:      {token_count}");
                println!("Nodes:       {node_count}");
                println!("Errors:      {error_count}");
                println!("Parse time:  {:.2}ms", stats.parse_time_ms);
            }
        }

        if error_count > 0 {
            worst = worst.max(1);
        }
    }

    Ok(worst)
}

fn count_nodes(node: &SyntaxNode) -> usize {
    let mut count = 1;
    for child in node.children() {
        count += count_nodes(&child);
    }
    count
}
