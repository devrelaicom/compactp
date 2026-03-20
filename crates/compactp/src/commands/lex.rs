use crate::output::OutputEnvelope;
use serde::Serialize;
use std::time::Instant;

#[derive(Serialize)]
struct TokenInfo {
    kind: String,
    text: String,
    offset: u32,
    len: u32,
}

pub fn run(source: &str, input_name: &str, json: bool, timing: bool) -> i32 {
    let start = Instant::now();
    let tokens = compactp_lexer::lex(source);
    let elapsed = start.elapsed();

    if json {
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

        let timing_ms = if timing {
            Some(elapsed.as_secs_f64() * 1000.0)
        } else {
            None
        };

        let envelope = OutputEnvelope::new(input_name.to_string(), token_infos, timing_ms);
        println!("{}", serde_json::to_string_pretty(&envelope).unwrap());
    } else {
        let mut offset = 0usize;
        for (kind, text) in &tokens {
            let len = text.len();
            println!("{kind:?} {offset}..{} {text:?}", offset + len);
            offset += len;
        }
        if timing {
            eprintln!("Lexed in {:.2}ms", elapsed.as_secs_f64() * 1000.0);
        }
    }

    0
}
