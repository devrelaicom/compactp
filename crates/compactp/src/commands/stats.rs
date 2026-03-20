use crate::output::OutputEnvelope;
use compactp_syntax::SyntaxNode;
use serde::Serialize;
use std::time::Instant;

#[derive(Serialize)]
struct Stats {
    file_size_bytes: usize,
    token_count: usize,
    node_count: usize,
    error_count: usize,
    parse_time_ms: f64,
}

pub fn run(source: &str, input_name: &str, json: bool, timing: bool) -> i32 {
    let file_size = source.len();
    let token_count = compactp_lexer::lex(source).len();

    let start = Instant::now();
    let result = compactp_parser::parse(source);
    let parse_time = start.elapsed();

    let root = SyntaxNode::new_root(result.green);
    let node_count = count_nodes(&root);
    let error_count = result.errors.len();

    let stats = Stats {
        file_size_bytes: file_size,
        token_count,
        node_count,
        error_count,
        parse_time_ms: parse_time.as_secs_f64() * 1000.0,
    };

    if json {
        let timing_ms = if timing {
            Some(parse_time.as_secs_f64() * 1000.0)
        } else {
            None
        };
        let envelope = OutputEnvelope::new(input_name.to_string(), stats, timing_ms);
        println!("{}", serde_json::to_string_pretty(&envelope).unwrap());
    } else {
        println!("File:        {input_name}");
        println!("Size:        {file_size} bytes");
        println!("Tokens:      {token_count}");
        println!("Nodes:       {node_count}");
        println!("Errors:      {error_count}");
        println!("Parse time:  {:.2}ms", stats.parse_time_ms);
    }

    0
}

fn count_nodes(node: &SyntaxNode) -> usize {
    let mut count = 1;
    for child in node.children() {
        count += count_nodes(&child);
    }
    count
}
