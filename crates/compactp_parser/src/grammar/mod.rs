use crate::parser::Parser;
use compactp_syntax::SyntaxKind::*;

pub(crate) fn source_file(p: &mut Parser) {
    let m = p.start();
    // For now, just consume all tokens
    while !p.at_end() {
        p.bump_any();
    }
    // Eat trailing trivia so it's inside the SOURCE_FILE node
    p.eat_trivia();
    m.complete(p, SOURCE_FILE);
}
