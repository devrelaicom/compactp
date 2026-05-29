use compactp_syntax::SyntaxKind;

#[derive(Debug)]
pub(crate) enum Event {
    StartNode {
        kind: SyntaxKind,
        forward_parent: Option<u32>,
    },
    Token {
        kind: SyntaxKind,
        n_raw_tokens: u8,
    },
    FinishNode,
    Error {
        message: String,
    },
    /// Tombstone for abandoned markers — skipped by the sink.
    Placeholder,
}
