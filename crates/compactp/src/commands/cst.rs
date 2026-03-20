use crate::output::OutputEnvelope;
use compactp_syntax::SyntaxNode;
use serde::Serialize;
use std::time::Instant;

#[derive(Serialize)]
struct CstNode {
    kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    children: Vec<CstNode>,
}

pub fn run(source: &str, input_name: &str, json: bool, timing: bool) -> i32 {
    let start = Instant::now();
    let result = compactp_parser::parse(source);
    let elapsed = start.elapsed();

    let root = SyntaxNode::new_root(result.green);

    if json {
        let timing_ms = if timing {
            Some(elapsed.as_secs_f64() * 1000.0)
        } else {
            None
        };

        let tree = syntax_node_to_json(&root);
        let envelope = OutputEnvelope::new(input_name.to_string(), tree, timing_ms);
        println!("{}", serde_json::to_string_pretty(&envelope).unwrap());
    } else {
        print_tree(&root, 0);
        if timing {
            eprintln!("Parsed in {:.2}ms", elapsed.as_secs_f64() * 1000.0);
        }
    }

    if result.errors.is_empty() { 0 } else { 1 }
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
