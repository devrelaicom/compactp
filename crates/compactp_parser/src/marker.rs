use crate::event::Event;
use crate::parser::Parser;
use compactp_syntax::SyntaxKind;
use drop_bomb::DropBomb;

/// An open marker in the event stream. Must be completed or abandoned before being dropped.
pub(crate) struct Marker {
    pub(crate) pos: u32,
    bomb: DropBomb,
}

impl Marker {
    pub(crate) fn new(pos: u32) -> Self {
        Self {
            pos,
            bomb: DropBomb::new("Marker must be either completed or abandoned"),
        }
    }

    /// Complete this marker, wrapping all tokens since it was opened in a node of the given kind.
    pub(crate) fn complete(mut self, p: &mut Parser, kind: SyntaxKind) -> CompletedMarker {
        self.bomb.defuse();
        match &mut p.events[self.pos as usize] {
            Event::StartNode { kind: slot, .. } => *slot = kind,
            _ => unreachable!(),
        }
        p.push_event(Event::FinishNode);
        CompletedMarker { pos: self.pos }
    }

    /// Abandon this marker, removing it from the event stream.
    pub(crate) fn abandon(mut self, p: &mut Parser) {
        self.bomb.defuse();
        if self.pos as usize == p.events.len() - 1 {
            match p.events.pop() {
                Some(Event::StartNode {
                    kind: SyntaxKind::ERROR,
                    forward_parent: None,
                }) => {
                    // Event removed cleanly
                }
                _ => unreachable!(),
            }
        } else {
            // Replace with a placeholder tombstone
            p.events[self.pos as usize] = Event::Placeholder;
        }
    }
}

/// A marker that has been completed. Can be used to wrap the completed node in a parent.
pub(crate) struct CompletedMarker {
    pub(crate) pos: u32,
}

impl CompletedMarker {
    /// Wrap this completed node in a new parent node (for Pratt parsing).
    ///
    /// Creates a new marker at the current position in the event stream and sets
    /// the COMPLETED marker's forward_parent to point to the new marker. This way,
    /// when the sink processes events in order and encounters the original StartNode,
    /// it follows the forward_parent chain to discover the new outer parent.
    pub(crate) fn precede(self, p: &mut Parser) -> Marker {
        let new_m = p.start();
        match &mut p.events[self.pos as usize] {
            Event::StartNode { forward_parent, .. } => {
                *forward_parent = Some(new_m.pos);
            }
            _ => unreachable!(),
        }
        new_m
    }
}
