#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(src) = std::str::from_utf8(data) else { return };

    // No panic on any UTF-8 input.
    let result = compactp_parser::parse(src);

    // Round-trip assertion: for syntactically-valid input (no errors),
    // the CST's text content must equal the input byte-for-byte. The
    // parser's lossless contract.
    if result.errors.is_empty() {
        let root = compactp_syntax::SyntaxNode::new_root(result.green.clone());
        let round_trip = root.text().to_string();
        assert_eq!(
            round_trip, src,
            "CST round-trip must equal input for accept-cases"
        );
    }
});
