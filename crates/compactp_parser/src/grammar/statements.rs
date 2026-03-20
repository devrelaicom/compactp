//! Statement parsing.
//!
//! Grammar:
//! ```text
//! stmt → expr = expr ;
//!      → expr += expr ;
//!      → expr -= expr ;
//!      → expr-seq ;
//!      → return expr-seq? ;
//!      → if ( expr-seq ) stmt (else stmt)?
//!      → for ( const id of nat..nat | expr-seq ) stmt
//!      → assert expr str ;       (tree-sitter form)
//!      → assert(cond, "msg") ;   (corpus form — handled as call expression)
//!      → const pattern : type? = expr ;
//!      → block
//! ```

use crate::parser::Parser;
use compactp_syntax::SyntaxKind::*;

/// Parse a statement.
pub(crate) fn stmt(p: &mut Parser) {
    match p.current() {
        L_BRACE => block(p),
        RETURN_KW => return_stmt(p),
        IF_KW => if_stmt(p),
        FOR_KW => for_stmt(p),
        ASSERT_KW => assert_stmt(p),
        CONST_KW => const_stmt(p),
        _ => expr_or_assign_stmt(p),
    }
}

/// `{ stmt ... stmt }`
pub(crate) fn block(p: &mut Parser) {
    let m = p.start();
    p.expect(L_BRACE);
    while !p.at(R_BRACE) && !p.at_end() {
        if p.errors_exhausted() {
            break;
        }
        stmt(p);
    }
    p.expect(R_BRACE);
    m.complete(p, BLOCK);
}

/// `return expr-seq? ;`
fn return_stmt(p: &mut Parser) {
    let m = p.start();
    p.bump(RETURN_KW);
    if !p.at(SEMICOLON) && !p.at(R_BRACE) && !p.at_end() {
        super::expressions::expr_seq(p);
    }
    p.expect(SEMICOLON);
    m.complete(p, RETURN_STMT);
}

/// `if ( expr-seq ) stmt (else stmt)?`
fn if_stmt(p: &mut Parser) {
    let m = p.start();
    p.bump(IF_KW);
    p.expect(L_PAREN);
    super::expressions::expr_seq(p);
    p.expect(R_PAREN);
    stmt(p);
    if p.eat(ELSE_KW) {
        stmt(p);
    }
    m.complete(p, IF_STMT);
}

/// `for ( const id of nat..nat | expr-seq ) stmt`
fn for_stmt(p: &mut Parser) {
    let m = p.start();
    p.bump(FOR_KW);
    p.expect(L_PAREN);
    p.expect(CONST_KW);
    p.expect(IDENT);
    p.expect(OF_KW);

    // Range or expression
    if is_range_start(p) {
        let rm = p.start();
        p.bump_any(); // nat
        p.expect(DOT_DOT);
        super::expressions::expr(p);
        rm.complete(p, RANGE_EXPR);
    } else {
        super::expressions::expr_seq(p);
    }

    p.expect(R_PAREN);
    stmt(p);
    m.complete(p, FOR_STMT);
}

/// Check if we're at the start of a range: `nat ..`
fn is_range_start(p: &Parser) -> bool {
    matches!(p.current(), INT_LIT | HEX_LIT | OCT_LIT | BIN_LIT) && p.nth(1) == DOT_DOT
}

/// `assert expr str ;` (tree-sitter form)
/// This is the keyword-based assert syntax. The call-form `assert(cond, "msg")`
/// is handled as an expression statement via ident_expr → call_expr.
fn assert_stmt(p: &mut Parser) {
    let m = p.start();
    p.bump(ASSERT_KW);

    // Distinguish: `assert(expr, str)` call form vs `assert expr str ;` keyword form.
    if p.at(L_PAREN) {
        // Call form: `assert(cond, "msg");`
        p.bump(L_PAREN);
        super::expressions::expr(p);
        p.expect(COMMA);
        super::expressions::expr(p);
        p.expect(R_PAREN);
        p.expect(SEMICOLON);
        m.complete(p, ASSERT_STMT);
    } else {
        // Keyword form: `assert expr str ;`
        super::expressions::expr(p);
        p.expect(STRING_LIT);
        p.expect(SEMICOLON);
        m.complete(p, ASSERT_STMT);
    }
}

/// `const pattern : type? = expr ;`
fn const_stmt(p: &mut Parser) {
    let m = p.start();
    p.bump(CONST_KW);
    super::patterns::pattern(p);
    // Optional type annotation
    if p.eat(COLON) {
        super::types::ty(p);
    }
    p.expect(EQ);
    super::expressions::expr(p);
    p.expect(SEMICOLON);
    m.complete(p, CONST_STMT);
}

/// Parse an expression statement or assignment statement.
/// `expr-seq ;` or `expr op= expr ;`
fn expr_or_assign_stmt(p: &mut Parser) {
    let m = p.start();
    super::expressions::expr(p);

    match p.current() {
        EQ | PLUS_EQ | MINUS_EQ => {
            p.bump_any(); // consume the assignment operator
            super::expressions::expr(p);
            p.expect(SEMICOLON);
            m.complete(p, ASSIGN_STMT);
        }
        COMMA => {
            // Expression sequence: `expr, ..., expr ;`
            while p.eat(COMMA) {
                if p.at(SEMICOLON) || p.at(R_BRACE) || p.at_end() {
                    break;
                }
                super::expressions::expr(p);
            }
            p.expect(SEMICOLON);
            m.complete(p, EXPR_STMT);
        }
        _ => {
            p.expect(SEMICOLON);
            m.complete(p, EXPR_STMT);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::grammar::tests::check;
    use expect_test::expect;

    #[test]
    fn stmt_return_simple() {
        check(
            "circuit f() : Field { return 42; }",
            expect![[r#"
                SOURCE_FILE@0..34
                  CIRCUIT_DEF@0..34
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
                    BLOCK@19..34
                      WHITESPACE@19..20 " "
                      L_BRACE@20..21 "{"
                      RETURN_STMT@21..32
                        WHITESPACE@21..22 " "
                        RETURN_KW@22..28 "return"
                        EXPR_STMT@28..31
                          WHITESPACE@28..29 " "
                          INT_LIT@29..31 "42"
                        SEMICOLON@31..32 ";"
                      WHITESPACE@32..33 " "
                      R_BRACE@33..34 "}"
            "#]],
        );
    }

    #[test]
    fn stmt_return_bare() {
        check(
            "circuit f() : Field { return; }",
            expect![[r#"
                SOURCE_FILE@0..31
                  CIRCUIT_DEF@0..31
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
                    BLOCK@19..31
                      WHITESPACE@19..20 " "
                      L_BRACE@20..21 "{"
                      RETURN_STMT@21..29
                        WHITESPACE@21..22 " "
                        RETURN_KW@22..28 "return"
                        SEMICOLON@28..29 ";"
                      WHITESPACE@29..30 " "
                      R_BRACE@30..31 "}"
            "#]],
        );
    }

    #[test]
    fn stmt_if_else() {
        check(
            "circuit f() : Field { if (x) { return 1; } else { return 2; } }",
            expect![[r#"
                SOURCE_FILE@0..63
                  CIRCUIT_DEF@0..63
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
                    BLOCK@19..63
                      WHITESPACE@19..20 " "
                      L_BRACE@20..21 "{"
                      IF_STMT@21..61
                        WHITESPACE@21..22 " "
                        IF_KW@22..24 "if"
                        WHITESPACE@24..25 " "
                        L_PAREN@25..26 "("
                        EXPR_STMT@26..27
                          IDENT@26..27 "x"
                        R_PAREN@27..28 ")"
                        BLOCK@28..42
                          WHITESPACE@28..29 " "
                          L_BRACE@29..30 "{"
                          RETURN_STMT@30..40
                            WHITESPACE@30..31 " "
                            RETURN_KW@31..37 "return"
                            EXPR_STMT@37..39
                              WHITESPACE@37..38 " "
                              INT_LIT@38..39 "1"
                            SEMICOLON@39..40 ";"
                          WHITESPACE@40..41 " "
                          R_BRACE@41..42 "}"
                        WHITESPACE@42..43 " "
                        ELSE_KW@43..47 "else"
                        BLOCK@47..61
                          WHITESPACE@47..48 " "
                          L_BRACE@48..49 "{"
                          RETURN_STMT@49..59
                            WHITESPACE@49..50 " "
                            RETURN_KW@50..56 "return"
                            EXPR_STMT@56..58
                              WHITESPACE@56..57 " "
                              INT_LIT@57..58 "2"
                            SEMICOLON@58..59 ";"
                          WHITESPACE@59..60 " "
                          R_BRACE@60..61 "}"
                      WHITESPACE@61..62 " "
                      R_BRACE@62..63 "}"
            "#]],
        );
    }

    #[test]
    fn stmt_for_range() {
        check(
            "circuit f() : Field { for (const i of 0..10) { x; } }",
            expect![[r#"
                SOURCE_FILE@0..53
                  CIRCUIT_DEF@0..53
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
                    BLOCK@19..53
                      WHITESPACE@19..20 " "
                      L_BRACE@20..21 "{"
                      FOR_STMT@21..51
                        WHITESPACE@21..22 " "
                        FOR_KW@22..25 "for"
                        WHITESPACE@25..26 " "
                        L_PAREN@26..27 "("
                        CONST_KW@27..32 "const"
                        WHITESPACE@32..33 " "
                        IDENT@33..34 "i"
                        WHITESPACE@34..35 " "
                        OF_KW@35..37 "of"
                        RANGE_EXPR@37..43
                          WHITESPACE@37..38 " "
                          INT_LIT@38..39 "0"
                          DOT_DOT@39..41 ".."
                          EXPR_STMT@41..43
                            INT_LIT@41..43 "10"
                        R_PAREN@43..44 ")"
                        BLOCK@44..51
                          WHITESPACE@44..45 " "
                          L_BRACE@45..46 "{"
                          EXPR_STMT@46..49
                            EXPR_STMT@46..48
                              WHITESPACE@46..47 " "
                              IDENT@47..48 "x"
                            SEMICOLON@48..49 ";"
                          WHITESPACE@49..50 " "
                          R_BRACE@50..51 "}"
                      WHITESPACE@51..52 " "
                      R_BRACE@52..53 "}"
            "#]],
        );
    }

    #[test]
    fn stmt_assert_call_form() {
        check(
            r#"circuit f() : Field { assert(x, "fail"); }"#,
            expect![[r#"
                SOURCE_FILE@0..42
                  CIRCUIT_DEF@0..42
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
                    BLOCK@19..42
                      WHITESPACE@19..20 " "
                      L_BRACE@20..21 "{"
                      ASSERT_STMT@21..40
                        WHITESPACE@21..22 " "
                        ASSERT_KW@22..28 "assert"
                        L_PAREN@28..29 "("
                        EXPR_STMT@29..30
                          IDENT@29..30 "x"
                        COMMA@30..31 ","
                        EXPR_STMT@31..38
                          WHITESPACE@31..32 " "
                          STRING_LIT@32..38 "\"fail\""
                        R_PAREN@38..39 ")"
                        SEMICOLON@39..40 ";"
                      WHITESPACE@40..41 " "
                      R_BRACE@41..42 "}"
            "#]],
        );
    }

    #[test]
    fn stmt_assert_keyword_form() {
        check(
            r#"circuit f() : Field { assert x "fail"; }"#,
            expect![[r#"
                SOURCE_FILE@0..40
                  CIRCUIT_DEF@0..40
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
                    BLOCK@19..40
                      WHITESPACE@19..20 " "
                      L_BRACE@20..21 "{"
                      ASSERT_STMT@21..38
                        WHITESPACE@21..22 " "
                        ASSERT_KW@22..28 "assert"
                        EXPR_STMT@28..30
                          WHITESPACE@28..29 " "
                          IDENT@29..30 "x"
                        WHITESPACE@30..31 " "
                        STRING_LIT@31..37 "\"fail\""
                        SEMICOLON@37..38 ";"
                      WHITESPACE@38..39 " "
                      R_BRACE@39..40 "}"
            "#]],
        );
    }

    #[test]
    fn stmt_const_typed() {
        check(
            "circuit f() : Field { const x : Field = 1; }",
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
                        IDENT_PAT@27..29
                          WHITESPACE@27..28 " "
                          IDENT@28..29 "x"
                        WHITESPACE@29..30 " "
                        COLON@30..31 ":"
                        FIELD_TYPE@31..37
                          WHITESPACE@31..32 " "
                          FIELD_KW@32..37 "Field"
                        WHITESPACE@37..38 " "
                        EQ@38..39 "="
                        EXPR_STMT@39..41
                          WHITESPACE@39..40 " "
                          INT_LIT@40..41 "1"
                        SEMICOLON@41..42 ";"
                      WHITESPACE@42..43 " "
                      R_BRACE@43..44 "}"
            "#]],
        );
    }

    #[test]
    fn stmt_assign() {
        check(
            "circuit f() : Field { x = 1; }",
            expect![[r#"
                SOURCE_FILE@0..30
                  CIRCUIT_DEF@0..30
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
                    BLOCK@19..30
                      WHITESPACE@19..20 " "
                      L_BRACE@20..21 "{"
                      ASSIGN_STMT@21..28
                        EXPR_STMT@21..23
                          WHITESPACE@21..22 " "
                          IDENT@22..23 "x"
                        WHITESPACE@23..24 " "
                        EQ@24..25 "="
                        EXPR_STMT@25..27
                          WHITESPACE@25..26 " "
                          INT_LIT@26..27 "1"
                        SEMICOLON@27..28 ";"
                      WHITESPACE@28..29 " "
                      R_BRACE@29..30 "}"
            "#]],
        );
    }

    #[test]
    fn stmt_plus_assign() {
        check(
            "circuit f() : Field { x += 1; }",
            expect![[r#"
                SOURCE_FILE@0..31
                  CIRCUIT_DEF@0..31
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
                    BLOCK@19..31
                      WHITESPACE@19..20 " "
                      L_BRACE@20..21 "{"
                      ASSIGN_STMT@21..29
                        EXPR_STMT@21..23
                          WHITESPACE@21..22 " "
                          IDENT@22..23 "x"
                        WHITESPACE@23..24 " "
                        PLUS_EQ@24..26 "+="
                        EXPR_STMT@26..28
                          WHITESPACE@26..27 " "
                          INT_LIT@27..28 "1"
                        SEMICOLON@28..29 ";"
                      WHITESPACE@29..30 " "
                      R_BRACE@30..31 "}"
            "#]],
        );
    }

    #[test]
    fn stmt_block() {
        check(
            "circuit f() : Field { { return 1; } }",
            expect![[r#"
                SOURCE_FILE@0..37
                  CIRCUIT_DEF@0..37
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
                    BLOCK@19..37
                      WHITESPACE@19..20 " "
                      L_BRACE@20..21 "{"
                      BLOCK@21..35
                        WHITESPACE@21..22 " "
                        L_BRACE@22..23 "{"
                        RETURN_STMT@23..33
                          WHITESPACE@23..24 " "
                          RETURN_KW@24..30 "return"
                          EXPR_STMT@30..32
                            WHITESPACE@30..31 " "
                            INT_LIT@31..32 "1"
                          SEMICOLON@32..33 ";"
                        WHITESPACE@33..34 " "
                        R_BRACE@34..35 "}"
                      WHITESPACE@35..36 " "
                      R_BRACE@36..37 "}"
            "#]],
        );
    }

    #[test]
    fn stmt_for_expr() {
        check(
            "circuit f() : Field { for (const i of n) { x; } }",
            expect![[r#"
                SOURCE_FILE@0..49
                  CIRCUIT_DEF@0..49
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
                    BLOCK@19..49
                      WHITESPACE@19..20 " "
                      L_BRACE@20..21 "{"
                      FOR_STMT@21..47
                        WHITESPACE@21..22 " "
                        FOR_KW@22..25 "for"
                        WHITESPACE@25..26 " "
                        L_PAREN@26..27 "("
                        CONST_KW@27..32 "const"
                        WHITESPACE@32..33 " "
                        IDENT@33..34 "i"
                        WHITESPACE@34..35 " "
                        OF_KW@35..37 "of"
                        EXPR_STMT@37..39
                          WHITESPACE@37..38 " "
                          IDENT@38..39 "n"
                        R_PAREN@39..40 ")"
                        BLOCK@40..47
                          WHITESPACE@40..41 " "
                          L_BRACE@41..42 "{"
                          EXPR_STMT@42..45
                            EXPR_STMT@42..44
                              WHITESPACE@42..43 " "
                              IDENT@43..44 "x"
                            SEMICOLON@44..45 ";"
                          WHITESPACE@45..46 " "
                          R_BRACE@46..47 "}"
                      WHITESPACE@47..48 " "
                      R_BRACE@48..49 "}"
            "#]],
        );
    }
}
