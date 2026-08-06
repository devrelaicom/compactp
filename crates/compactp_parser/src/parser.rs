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
    pub(crate) current_depth: u32,
    pub(crate) max_depth: u32,
    depth_watermark: u32,
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
            current_depth: 0,
            max_depth: 256,
            depth_watermark: 0,
        }
    }

    pub(crate) fn set_options(&mut self, opts: &crate::ParseOptions) {
        self.recover = opts.recover;
        self.max_errors = opts.max_errors;
        self.max_depth = opts.max_depth;
    }

    /// Enter a recursive grammar function. Returns `true` if depth is
    /// within limit (caller proceeds); `false` if overflow (caller
    /// emits a diagnostic + ERROR node and returns).
    pub(crate) fn enter_depth(&mut self) -> bool {
        if self.current_depth >= self.max_depth {
            return false;
        }
        self.current_depth += 1;
        if self.current_depth > self.depth_watermark {
            self.depth_watermark = self.current_depth;
        }
        true
    }

    /// Pair with `enter_depth`; decrement on function exit.
    ///
    /// Note this restores the *path* depth but deliberately leaves
    /// `depth_watermark` alone — see [`Parser::begin_subtree`].
    pub(crate) fn exit_depth(&mut self) {
        self.current_depth = self.current_depth.saturating_sub(1);
    }

    /// Deepest `current_depth` reached inside the subtree currently being
    /// measured. Its distance above `current_depth` is the *height* of
    /// what has been built so far.
    ///
    /// `current_depth` alone cannot answer that: it is a path counter, so
    /// it returns to its starting value once a child subtree finishes,
    /// forgetting how tall the child was. That is fine for plain
    /// recursive descent, where a node is only ever built above the path
    /// currently on the stack, but not for `CompletedMarker::precede`,
    /// which inserts a parent *above an already-finished subtree* and so
    /// pushes all of it one level deeper.
    pub(crate) fn depth_watermark(&self) -> u32 {
        self.depth_watermark
    }

    /// Begin measuring a fresh subtree's height, returning the enclosing
    /// watermark to hand back to [`Parser::end_subtree`].
    ///
    /// Without this reset the watermark would be a running maximum over
    /// the whole parse, and one deep expression would exhaust the budget
    /// for every later sibling.
    pub(crate) fn begin_subtree(&mut self) -> u32 {
        std::mem::replace(&mut self.depth_watermark, self.current_depth)
    }

    /// Finish measuring a subtree, folding its height back into the
    /// enclosing one — which contains it, and so is at least as deep.
    pub(crate) fn end_subtree(&mut self, enclosing: u32) {
        self.depth_watermark = self.depth_watermark.max(enclosing);
    }

    /// Index of the next raw token to be consumed.
    ///
    /// Only meaningful as a *progress* probe: a recovery loop that runs
    /// a production and finds this unchanged knows the production
    /// consumed nothing, so iterating again would repeat the same work
    /// on the same token. See [`crate::grammar::step_ensuring_progress`].
    pub(crate) fn tok_pos(&self) -> usize {
        self.pos
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

    /// The kinds of the upcoming non-trivia tokens, in order.
    ///
    /// [`Parser::nth`] restarts from the current position on every call,
    /// so a scan that reads `nth(0)`, `nth(1)`, … `nth(k)` walks the
    /// token list `k` times over — quadratic in the lookahead distance,
    /// and it re-skips the same trivia each pass. Use this instead
    /// whenever a heuristic needs to look at a *run* of upcoming tokens
    /// rather than one at a fixed offset.
    pub(crate) fn lookahead(&self) -> impl Iterator<Item = SyntaxKind> + '_ {
        self.tokens
            .get(self.pos..)
            .unwrap_or(&[])
            .iter()
            .map(|&(kind, _)| kind)
            .filter(|kind| !kind.is_trivia())
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
