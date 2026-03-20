//! Pattern parsing.
//!
//! Grammar:
//! ```text
//! pattern → id
//!         → [ pattern-tuple-elt , ... , pattern-tuple-elt ]
//!         → { pattern-struct-elt , ... , pattern-struct-elt }
//! pattern-tuple-elt → pattern
//! pattern-struct-elt → id | id : pattern
//! ```

use crate::grammar::comma_sep;
use crate::parser::Parser;
use compactp_syntax::SyntaxKind::*;

/// Parse a pattern.
pub(crate) fn pattern(p: &mut Parser) {
    match p.current() {
        L_BRACKET => tuple_pattern(p),
        L_BRACE => struct_pattern(p),
        IDENT => {
            let m = p.start();
            p.bump(IDENT);
            m.complete(p, IDENT_PAT);
        }
        _ => {
            p.error("expected pattern");
        }
    }
}

/// `[ pat, ..., pat ]`
fn tuple_pattern(p: &mut Parser) {
    let m = p.start();
    p.bump(L_BRACKET);
    comma_sep(p, R_BRACKET, tuple_pat_elt);
    p.expect(R_BRACKET);
    m.complete(p, TUPLE_PAT);
}

fn tuple_pat_elt(p: &mut Parser) {
    let m = p.start();
    pattern(p);
    m.complete(p, TUPLE_PAT_ELT);
}

/// `{ id , ... }` or `{ id: pat, ... }`
fn struct_pattern(p: &mut Parser) {
    let m = p.start();
    p.bump(L_BRACE);
    comma_sep(p, R_BRACE, struct_pat_field);
    p.expect(R_BRACE);
    m.complete(p, STRUCT_PAT);
}

fn struct_pat_field(p: &mut Parser) {
    let m = p.start();
    p.expect(IDENT);
    if p.eat(COLON) {
        pattern(p);
    }
    m.complete(p, STRUCT_PAT_FIELD);
}

/// Parse a pattern-or-parg (used in lambda parameters).
/// Returns the completed marker. This handles:
/// - `pattern : type` → PARAM (typed parameter)
/// - `pattern` alone → the pattern node itself
pub(crate) fn pattern_or_parg(p: &mut Parser) {
    // In all cases, the first thing we see is a pattern.
    // After the pattern, if we see `:`, it becomes a typed parameter.
    match p.current() {
        L_BRACKET => {
            // Tuple pattern, possibly typed.
            let m = p.start();
            tuple_pattern(p);
            if p.eat(COLON) {
                super::types::ty(p);
                m.complete(p, PARAM);
            } else {
                m.abandon(p);
            }
        }
        L_BRACE => {
            // Struct pattern, possibly typed.
            let m = p.start();
            struct_pattern(p);
            if p.eat(COLON) {
                super::types::ty(p);
                m.complete(p, PARAM);
            } else {
                m.abandon(p);
            }
        }
        IDENT => {
            // Could be `id`, `id : type`, or just id pattern.
            let m = p.start();
            p.bump(IDENT);
            if p.at(COLON) {
                // This is `id : type` → a typed parameter.
                p.bump(COLON);
                super::types::ty(p);
                m.complete(p, PARAM);
            } else {
                m.complete(p, IDENT_PAT);
            }
        }
        _ => {
            p.error("expected pattern or parameter");
        }
    }
}

/// Parse a parameter: `pattern : type`
pub(crate) fn param(p: &mut Parser) {
    let m = p.start();
    pattern(p);
    p.expect(COLON);
    super::types::ty(p);
    m.complete(p, PARAM);
}

/// Parse a simple argument: `id : type` (used in struct fields, witness args, etc.)
pub(crate) fn arg(p: &mut Parser) {
    let m = p.start();
    p.expect(IDENT);
    p.expect(COLON);
    super::types::ty(p);
    m.complete(p, STRUCT_FIELD);
}

#[cfg(test)]
mod tests {
    use crate::grammar::tests::check;
    use expect_test::expect;

    #[test]
    fn pattern_ident() {
        check(
            "circuit f(x: Field) : Field { }",
            expect![[r#"
                SOURCE_FILE@0..31
                  CIRCUIT_DEF@0..31
                    CIRCUIT_KW@0..7 "circuit"
                    WHITESPACE@7..8 " "
                    IDENT@8..9 "f"
                    L_PAREN@9..10 "("
                    PARAM@10..18
                      IDENT_PAT@10..11
                        IDENT@10..11 "x"
                      COLON@11..12 ":"
                      FIELD_TYPE@12..18
                        WHITESPACE@12..13 " "
                        FIELD_KW@13..18 "Field"
                    R_PAREN@18..19 ")"
                    WHITESPACE@19..20 " "
                    COLON@20..21 ":"
                    FIELD_TYPE@21..27
                      WHITESPACE@21..22 " "
                      FIELD_KW@22..27 "Field"
                    BLOCK@27..31
                      WHITESPACE@27..28 " "
                      L_BRACE@28..29 "{"
                      WHITESPACE@29..30 " "
                      R_BRACE@30..31 "}"
            "#]],
        );
    }

    #[test]
    fn pattern_tuple_in_const() {
        check(
            "circuit f() : Field { const [a, b] = x; }",
            expect![[r#"
                SOURCE_FILE@0..41
                  CIRCUIT_DEF@0..41
                    CIRCUIT_KW@0..7 "circuit"
                    WHITESPACE@7..8 " "
                    IDENT@8..9 "f"
                    L_PAREN@9..10 "("
                    R_PAREN@10..11 ")"
                    WHITESPACE@11..12 " "
                    COLON@12..13 ":"
                    FIELD_TYPE@13..19
                      WHITESPACE@13..14 " "
                      FIELD_KW@14..19 "Field"
                    BLOCK@19..41
                      WHITESPACE@19..20 " "
                      L_BRACE@20..21 "{"
                      CONST_STMT@21..39
                        WHITESPACE@21..22 " "
                        CONST_KW@22..27 "const"
                        TUPLE_PAT@27..34
                          WHITESPACE@27..28 " "
                          L_BRACKET@28..29 "["
                          TUPLE_PAT_ELT@29..30
                            IDENT_PAT@29..30
                              IDENT@29..30 "a"
                          COMMA@30..31 ","
                          TUPLE_PAT_ELT@31..33
                            IDENT_PAT@31..33
                              WHITESPACE@31..32 " "
                              IDENT@32..33 "b"
                          R_BRACKET@33..34 "]"
                        WHITESPACE@34..35 " "
                        EQ@35..36 "="
                        EXPR_STMT@36..38
                          WHITESPACE@36..37 " "
                          IDENT@37..38 "x"
                        SEMICOLON@38..39 ";"
                      WHITESPACE@39..40 " "
                      R_BRACE@40..41 "}"
            "#]],
        );
    }

    #[test]
    fn pattern_struct_in_const() {
        check(
            "circuit f() : Field { const {a, b: c} = x; }",
            expect![[r#"
                SOURCE_FILE@0..44
                  CIRCUIT_DEF@0..44
                    CIRCUIT_KW@0..7 "circuit"
                    WHITESPACE@7..8 " "
                    IDENT@8..9 "f"
                    L_PAREN@9..10 "("
                    R_PAREN@10..11 ")"
                    WHITESPACE@11..12 " "
                    COLON@12..13 ":"
                    FIELD_TYPE@13..19
                      WHITESPACE@13..14 " "
                      FIELD_KW@14..19 "Field"
                    BLOCK@19..44
                      WHITESPACE@19..20 " "
                      L_BRACE@20..21 "{"
                      CONST_STMT@21..42
                        WHITESPACE@21..22 " "
                        CONST_KW@22..27 "const"
                        STRUCT_PAT@27..37
                          WHITESPACE@27..28 " "
                          L_BRACE@28..29 "{"
                          STRUCT_PAT_FIELD@29..30
                            IDENT@29..30 "a"
                          COMMA@30..31 ","
                          STRUCT_PAT_FIELD@31..36
                            WHITESPACE@31..32 " "
                            IDENT@32..33 "b"
                            COLON@33..34 ":"
                            IDENT_PAT@34..36
                              WHITESPACE@34..35 " "
                              IDENT@35..36 "c"
                          R_BRACE@36..37 "}"
                        WHITESPACE@37..38 " "
                        EQ@38..39 "="
                        EXPR_STMT@39..41
                          WHITESPACE@39..40 " "
                          IDENT@40..41 "x"
                        SEMICOLON@41..42 ";"
                      WHITESPACE@42..43 " "
                      R_BRACE@43..44 "}"
            "#]],
        );
    }
}
