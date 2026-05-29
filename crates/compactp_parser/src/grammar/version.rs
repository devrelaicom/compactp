//! Version expression parsing for pragma declarations.
//!
//! Grammar:
//! ```text
//! version-expr  → version-expr || version-expr0
//!               → version-expr0
//! version-expr0 → version-expr0 && version-term
//!               → version-term
//! version-term  → version-atom
//!               → op version-atom
//!               → ! version-term
//!               → ( version-expr )
//! version-atom  → nat | version-literal
//! ```

use crate::parser::Parser;
use compactp_syntax::SyntaxKind::*;

/// Parse a version expression (top-level entry point for pragma version constraints).
pub(crate) fn version_expr(p: &mut Parser) {
    version_or_expr(p);
}

/// version-expr → version-expr0 (|| version-expr0)*
fn version_or_expr(p: &mut Parser) {
    let m = p.start();
    version_and_expr(p);
    if p.at(PIPE_PIPE) {
        while p.eat(PIPE_PIPE) {
            version_and_expr(p);
        }
        m.complete(p, VERSION_OR_EXPR);
    } else {
        m.abandon(p);
    }
}

/// version-expr0 → version-term (&& version-term)*
fn version_and_expr(p: &mut Parser) {
    let m = p.start();
    version_term(p);
    if p.at(AMP_AMP) {
        while p.eat(AMP_AMP) {
            version_term(p);
        }
        m.complete(p, VERSION_AND_EXPR);
    } else {
        m.abandon(p);
    }
}

/// version-term → version-atom | op version-atom | ! version-term | ( version-expr )
fn version_term(p: &mut Parser) {
    match p.current() {
        BANG => {
            let m = p.start();
            p.bump(BANG);
            version_term(p);
            m.complete(p, VERSION_UNARY_EXPR);
        }
        LT | LT_EQ | GT | GT_EQ => {
            let m = p.start();
            p.bump_any(); // consume the operator
            version_atom(p);
            m.complete(p, VERSION_UNARY_EXPR);
        }
        L_PAREN => {
            let m = p.start();
            p.bump(L_PAREN);
            version_or_expr(p);
            p.expect(R_PAREN);
            m.complete(p, VERSION_PAREN_EXPR);
        }
        _ => {
            version_atom(p);
        }
    }
}

/// version-atom → nat | version-literal
fn version_atom(p: &mut Parser) {
    let m = p.start();
    match p.current() {
        INT_LIT | VERSION_LIT => {
            p.bump_any();
            m.complete(p, VERSION_EXPR);
        }
        _ => {
            p.error("expected version number");
            m.abandon(p);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::grammar::tests::check;
    use expect_test::expect;

    #[test]
    fn pragma_simple_version() {
        check(
            "pragma compact 0.15.0;",
            expect![[r#"
                SOURCE_FILE@0..22
                  PRAGMA@0..22
                    PRAGMA_KW@0..6 "pragma"
                    WHITESPACE@6..7 " "
                    IDENT@7..14 "compact"
                    VERSION_EXPR@14..21
                      WHITESPACE@14..15 " "
                      VERSION_LIT@15..21 "0.15.0"
                    SEMICOLON@21..22 ";"
            "#]],
        );
    }

    #[test]
    fn pragma_version_or() {
        check(
            "pragma compact >= 0.15 || >= 1.0;",
            expect![[r#"
                SOURCE_FILE@0..33
                  PRAGMA@0..33
                    PRAGMA_KW@0..6 "pragma"
                    WHITESPACE@6..7 " "
                    IDENT@7..14 "compact"
                    VERSION_OR_EXPR@14..32
                      VERSION_UNARY_EXPR@14..22
                        WHITESPACE@14..15 " "
                        GT_EQ@15..17 ">="
                        VERSION_EXPR@17..22
                          WHITESPACE@17..18 " "
                          VERSION_LIT@18..22 "0.15"
                      WHITESPACE@22..23 " "
                      PIPE_PIPE@23..25 "||"
                      VERSION_UNARY_EXPR@25..32
                        WHITESPACE@25..26 " "
                        GT_EQ@26..28 ">="
                        VERSION_EXPR@28..32
                          WHITESPACE@28..29 " "
                          VERSION_LIT@29..32 "1.0"
                    SEMICOLON@32..33 ";"
            "#]],
        );
    }

    #[test]
    fn pragma_version_and() {
        check(
            "pragma compact >= 0.15 && < 1.0;",
            expect![[r#"
                SOURCE_FILE@0..32
                  PRAGMA@0..32
                    PRAGMA_KW@0..6 "pragma"
                    WHITESPACE@6..7 " "
                    IDENT@7..14 "compact"
                    VERSION_AND_EXPR@14..31
                      VERSION_UNARY_EXPR@14..22
                        WHITESPACE@14..15 " "
                        GT_EQ@15..17 ">="
                        VERSION_EXPR@17..22
                          WHITESPACE@17..18 " "
                          VERSION_LIT@18..22 "0.15"
                      WHITESPACE@22..23 " "
                      AMP_AMP@23..25 "&&"
                      VERSION_UNARY_EXPR@25..31
                        WHITESPACE@25..26 " "
                        LT@26..27 "<"
                        VERSION_EXPR@27..31
                          WHITESPACE@27..28 " "
                          VERSION_LIT@28..31 "1.0"
                    SEMICOLON@31..32 ";"
            "#]],
        );
    }

    #[test]
    fn pragma_version_not() {
        check(
            "pragma compact !0.14;",
            expect![[r#"
                SOURCE_FILE@0..21
                  PRAGMA@0..21
                    PRAGMA_KW@0..6 "pragma"
                    WHITESPACE@6..7 " "
                    IDENT@7..14 "compact"
                    VERSION_UNARY_EXPR@14..20
                      WHITESPACE@14..15 " "
                      BANG@15..16 "!"
                      VERSION_EXPR@16..20
                        VERSION_LIT@16..20 "0.14"
                    SEMICOLON@20..21 ";"
            "#]],
        );
    }

    #[test]
    fn pragma_version_paren() {
        check(
            "pragma compact (>= 1.0);",
            expect![[r#"
                SOURCE_FILE@0..24
                  PRAGMA@0..24
                    PRAGMA_KW@0..6 "pragma"
                    WHITESPACE@6..7 " "
                    IDENT@7..14 "compact"
                    VERSION_PAREN_EXPR@14..23
                      WHITESPACE@14..15 " "
                      L_PAREN@15..16 "("
                      VERSION_UNARY_EXPR@16..22
                        GT_EQ@16..18 ">="
                        VERSION_EXPR@18..22
                          WHITESPACE@18..19 " "
                          VERSION_LIT@19..22 "1.0"
                      R_PAREN@22..23 ")"
                    SEMICOLON@23..24 ";"
            "#]],
        );
    }

    #[test]
    fn pragma_int_version() {
        check(
            "pragma compact 1;",
            expect![[r#"
                SOURCE_FILE@0..17
                  PRAGMA@0..17
                    PRAGMA_KW@0..6 "pragma"
                    WHITESPACE@6..7 " "
                    IDENT@7..14 "compact"
                    VERSION_EXPR@14..16
                      WHITESPACE@14..15 " "
                      INT_LIT@15..16 "1"
                    SEMICOLON@16..17 ";"
            "#]],
        );
    }
}
