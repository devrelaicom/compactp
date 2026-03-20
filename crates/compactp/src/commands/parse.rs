use crate::output::OutputEnvelope;
use compactp_parser::ParseOptions;
use serde::Serialize;
use std::time::Instant;

#[derive(Serialize)]
struct ParseInfo {
    success: bool,
    error_count: usize,
    errors: Vec<String>,
}

pub fn run(
    source: &str,
    input_name: &str,
    json: bool,
    timing: bool,
    no_recover: bool,
    max_errors: Option<usize>,
    pretty: bool,
) -> i32 {
    let opts = ParseOptions {
        recover: !no_recover,
        max_errors: max_errors.unwrap_or(256),
    };

    let start = Instant::now();
    let result = compactp_parser::parse_with(source, opts);
    let elapsed = start.elapsed();

    let has_errors = !result.errors.is_empty();

    if json {
        let timing_ms = if timing {
            Some(elapsed.as_secs_f64() * 1000.0)
        } else {
            None
        };

        let info = ParseInfo {
            success: !has_errors,
            error_count: result.errors.len(),
            errors: result.errors.iter().map(|d| d.message.clone()).collect(),
        };

        let envelope = OutputEnvelope::new(input_name.to_string(), info, timing_ms);
        let output = if pretty {
            serde_json::to_string_pretty(&envelope).unwrap()
        } else {
            serde_json::to_string(&envelope).unwrap()
        };
        println!("{output}");
    } else {
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
            println!("OK");
        }
        if timing {
            eprintln!("Parsed in {:.2}ms", elapsed.as_secs_f64() * 1000.0);
        }
    }

    if has_errors { 1 } else { 0 }
}
