#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Lexer takes &str; libfuzzer-sys gives &[u8]. UTF-8 validate
    // and skip non-UTF-8 inputs (the parser front-end also rejects them).
    let Ok(src) = std::str::from_utf8(data) else { return };

    // No panic on any UTF-8 input.
    let tokens = compactp_lexer::lex(src);

    // Sum the lengths of every token's text — they should cover the
    // entire input exactly. If they don't, we have a span bug.
    let covered: usize = tokens.iter().map(|(_, text)| text.len()).sum();
    assert_eq!(
        covered,
        src.len(),
        "token text lengths must cover input exactly (covered {} of {})",
        covered,
        src.len()
    );
});
