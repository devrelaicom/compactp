use crate::event::Event;
use crate::marker::Marker;
use compactp_syntax::SyntaxKind;

pub(crate) struct Parser<'src> {
    tokens: Vec<(SyntaxKind, &'src str)>,
    pos: usize,
    pub(crate) events: Vec<Event>,
    pub(crate) recover: bool,
    pub(crate) max_errors: usize,
    pub(crate) error_count: usize,
}

impl<'src> Parser<'src> {
    pub(crate) fn new(tokens: Vec<(SyntaxKind, &'src str)>) -> Self {
        Self {
            tokens,
            pos: 0,
            events: Vec::new(),
            recover: true,
            max_errors: 256,
            error_count: 0,
        }
    }

    pub(crate) fn set_options(&mut self, opts: &crate::ParseOptions) {
        self.recover = opts.recover;
        self.max_errors = opts.max_errors;
    }

    /// Peek at the current non-trivia token kind.
    pub(crate) fn current(&self) -> SyntaxKind {
        self.nth(0)
    }

    /// Lookahead n non-trivia tokens.
    pub(crate) fn nth(&self, n: usize) -> SyntaxKind {
        let mut i = self.pos;
        let mut non_trivia = 0;
        while i < self.tokens.len() {
            let kind = self.tokens[i].0;
            if !kind.is_trivia() {
                if non_trivia == n {
                    return kind;
                }
                non_trivia += 1;
            }
            i += 1;
        }
        SyntaxKind::EOF
    }

    /// Check if the current non-trivia token matches.
    pub(crate) fn at(&self, kind: SyntaxKind) -> bool {
        self.current() == kind
    }

    /// Consume the current token if it matches, returning true. Otherwise false.
    pub(crate) fn eat(&mut self, kind: SyntaxKind) -> bool {
        if self.at(kind) {
            self.bump(kind);
            true
        } else {
            false
        }
    }

    /// Consume the current token, or emit an error if it doesn't match.
    pub(crate) fn expect(&mut self, kind: SyntaxKind) {
        if !self.eat(kind) {
            self.error(format!("expected {kind:?}"));
        }
    }

    /// Unconditionally consume the current token (eating leading trivia first).
    pub(crate) fn bump(&mut self, kind: SyntaxKind) {
        self.eat_trivia();
        assert!(
            self.pos < self.tokens.len(),
            "bump past end of tokens, expected {kind:?}"
        );
        assert_eq!(
            self.tokens[self.pos].0, kind,
            "expected {kind:?}, got {:?}",
            self.tokens[self.pos].0
        );
        self.push_event(Event::Token {
            kind,
            n_raw_tokens: 1,
        });
        self.pos += 1;
    }

    /// Consume any token regardless of kind.
    pub(crate) fn bump_any(&mut self) {
        self.eat_trivia();
        if self.pos < self.tokens.len() {
            let kind = self.tokens[self.pos].0;
            self.push_event(Event::Token {
                kind,
                n_raw_tokens: 1,
            });
            self.pos += 1;
        }
    }

    /// Open a new marker in the event stream.
    pub(crate) fn start(&mut self) -> Marker {
        let pos = self.events.len() as u32;
        self.push_event(Event::StartNode {
            kind: SyntaxKind::ERROR, // placeholder, overwritten by complete()
            forward_parent: None,
        });
        Marker::new(pos)
    }

    /// Emit a parse error.
    pub(crate) fn error(&mut self, message: impl Into<String>) {
        self.error_count += 1;
        self.push_event(Event::Error {
            message: message.into(),
        });
    }

    pub(crate) fn push_event(&mut self, event: Event) {
        self.events.push(event);
    }

    /// Consume leading trivia tokens (whitespace, comments).
    pub(crate) fn eat_trivia(&mut self) {
        while self.pos < self.tokens.len() && self.tokens[self.pos].0.is_trivia() {
            let kind = self.tokens[self.pos].0;
            self.push_event(Event::Token {
                kind,
                n_raw_tokens: 1,
            });
            self.pos += 1;
        }
    }

    /// Check if we've reached the end of input.
    pub(crate) fn at_end(&self) -> bool {
        self.current() == SyntaxKind::EOF
    }

    /// Get the text of the current non-trivia token (for diagnostics).
    #[allow(dead_code)]
    pub(crate) fn current_text(&self) -> &str {
        let mut i = self.pos;
        while i < self.tokens.len() {
            if !self.tokens[i].0.is_trivia() {
                return self.tokens[i].1;
            }
            i += 1;
        }
        ""
    }

    /// Check if the error budget has been exhausted.
    pub(crate) fn errors_exhausted(&self) -> bool {
        self.error_count >= self.max_errors
    }
}
