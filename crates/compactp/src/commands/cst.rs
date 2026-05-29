use crate::Cli;
use crate::error::CliError;
use crate::input::resolve_inputs;
use crate::output::OutputEnvelope;
use compactp_syntax::SyntaxNode;
use rowan::GreenNode;
use serde::Serialize;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Debug, Clone, Serialize)]
struct CstNode {
    kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    children: Vec<CstNode>,
}

pub fn run(cli: &Cli, paths: &[PathBuf]) -> Result<i32, CliError> {
    let inputs = resolve_inputs(paths, cli.stdin_filename.as_deref())?;
    let mut worst = 0;

    for input in inputs {
        let start = Instant::now();
        let result = compactp_parser::parse(&input.source);
        let elapsed = start.elapsed();

        let root = root_from_green(result.green);

        match cli.format {
            crate::OutputFormat::Json => {
                let timing_ms = cli.timing.then_some(elapsed.as_secs_f64() * 1000.0);
                let tree = syntax_node_to_json(&root);
                let envelope = OutputEnvelope::new(input.label.clone(), tree, timing_ms);
                crate::output::print_json(&envelope, cli.pretty)?;
            }
            crate::OutputFormat::Human => {
                print_tree(&root, 0);
                if cli.timing {
                    eprintln!("Parsed in {:.2}ms", elapsed.as_secs_f64() * 1000.0);
                }
            }
        }

        if !result.errors.is_empty() {
            worst = worst.max(1);
        }
    }

    Ok(worst)
}

pub(crate) fn root_from_green(green: GreenNode) -> SyntaxNode {
    SyntaxNode::new_root(green)
}

fn print_tree(node: &SyntaxNode, indent: usize) {
    let pad = "  ".repeat(indent);
    println!("{pad}{:?}@{:?}", node.kind(), node.text_range());
    for child in node.children_with_tokens() {
        match child {
            rowan::NodeOrToken::Node(n) => print_tree(&n, indent + 1),
            rowan::NodeOrToken::Token(t) => {
                let pad = "  ".repeat(indent + 1);
                println!("{pad}{:?}@{:?} {:?}", t.kind(), t.text_range(), t.text());
            }
        }
    }
}

fn syntax_node_to_json(node: &SyntaxNode) -> CstNode {
    let mut children = Vec::new();
    for child in node.children_with_tokens() {
        match child {
            rowan::NodeOrToken::Node(n) => {
                children.push(syntax_node_to_json(&n));
            }
            rowan::NodeOrToken::Token(t) => {
                children.push(CstNode {
                    kind: format!("{:?}", t.kind()),
                    text: Some(t.text().to_string()),
                    children: Vec::new(),
                });
            }
        }
    }
    CstNode {
        kind: format!("{:?}", node.kind()),
        text: None,
        children,
    }
}
