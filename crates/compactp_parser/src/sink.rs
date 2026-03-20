use crate::event::Event;
use compactp_diagnostics::{Diagnostic, DiagnosticCode};
use compactp_syntax::SyntaxKind;
use rowan::GreenNode;

pub(crate) struct Sink<'src> {
    events: Vec<Event>,
    tokens: Vec<(SyntaxKind, &'src str)>,
    token_pos: usize,
    builder: rowan::GreenNodeBuilder<'static>,
    errors: Vec<Diagnostic>,
    byte_offset: usize,
}

impl<'src> Sink<'src> {
    pub(crate) fn new(events: Vec<Event>, tokens: Vec<(SyntaxKind, &'src str)>) -> Self {
        Self {
            events,
            tokens,
            token_pos: 0,
            builder: rowan::GreenNodeBuilder::new(),
            errors: Vec::new(),
            byte_offset: 0,
        }
    }

    pub(crate) fn finish(mut self) -> (GreenNode, Vec<Diagnostic>) {
        // Resolve forward parents. For each event, we need to figure out the
        // chain of forward_parent pointers and process them in reverse order
        // (outermost parent first).
        let mut forward_parents = Vec::new();

        for i in 0..self.events.len() {
            match std::mem::replace(&mut self.events[i], Event::Placeholder) {
                Event::StartNode {
                    kind,
                    forward_parent,
                } => {
                    // Walk the forward_parent chain to collect all ancestors
                    forward_parents.push(kind);
                    let mut fp = forward_parent;
                    while let Some(parent_idx) = fp {
                        let pidx = parent_idx as usize;
                        match std::mem::replace(&mut self.events[pidx], Event::Placeholder) {
                            Event::StartNode {
                                kind,
                                forward_parent,
                            } => {
                                forward_parents.push(kind);
                                fp = forward_parent;
                            }
                            _ => unreachable!(),
                        }
                    }

                    // Process in reverse order (outermost first)
                    for kind in forward_parents.drain(..).rev() {
                        self.builder
                            .start_node(compactp_syntax::CompactLanguage::kind_to_raw(kind));
                    }
                }
                Event::Token { kind, n_raw_tokens } => {
                    self.token(kind, n_raw_tokens);
                }
                Event::FinishNode => {
                    self.builder.finish_node();
                }
                Event::Error { message } => {
                    let offset = self.byte_offset as u32;
                    let span = rowan::TextRange::new(offset.into(), offset.into());
                    self.errors.push(Diagnostic::error(
                        DiagnosticCode::new("E", 1),
                        message,
                        span,
                    ));
                }
                Event::Placeholder => {}
            }
        }

        // Consume any remaining trailing trivia
        self.eat_remaining_trivia();

        (self.builder.finish(), self.errors)
    }

    fn token(&mut self, _kind: SyntaxKind, n_raw_tokens: u8) {
        // Emit the token(s)
        for _ in 0..n_raw_tokens {
            if self.token_pos < self.tokens.len() {
                let (tk, text) = self.tokens[self.token_pos];
                self.builder
                    .token(compactp_syntax::CompactLanguage::kind_to_raw(tk), text);
                self.byte_offset += text.len();
                self.token_pos += 1;
            }
        }
    }

    fn eat_remaining_trivia(&mut self) {
        while self.token_pos < self.tokens.len() {
            let (kind, text) = self.tokens[self.token_pos];
            if kind.is_trivia() {
                self.builder
                    .token(compactp_syntax::CompactLanguage::kind_to_raw(kind), text);
                self.byte_offset += text.len();
                self.token_pos += 1;
            } else {
                break;
            }
        }
    }
}

use rowan::Language;
