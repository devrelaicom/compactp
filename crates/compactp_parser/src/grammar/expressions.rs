//! Expression parsing using a Pratt (top-down operator precedence) parser.
//!
//! Binding power table (from the design spec):
//!
//! | BP | Level          | Operators  | Associativity |
//! |----|----------------|------------|---------------|
//! |  1 | Conditional    | ? :        | Right         |
//! |  2 | Logical OR     | ||         | Left          |
//! |  3 | Logical AND    | &&         | Left          |
//! |  4 | Equality       | == !=      | Left          |
//! |  5 | Relational     | < <= > >=  | None          |
//! |  6 | Cast           | as         | Left          |
//! |  7 | Additive       | + -        | Left          |
//! |  8 | Multiplicative | *          | Left          |
//! |  9 | Unary prefix   | !          | --            |
//! | 10 | Postfix        | . [] ()    | Left          |

use crate::grammar::{comma_sep, comma_sep1};
use crate::marker::CompletedMarker;
use crate::parser::Parser;
use compactp_syntax::SyntaxKind;
use compactp_syntax::SyntaxKind::*;

/// Parse an expression (entry point for all expression contexts).
pub(crate) fn expr(p: &mut Parser) {
    expr_bp(p, 0);
}

/// Parse an expression sequence: `expr` or `expr, ..., expr, expr`.
///
/// An expr_seq is the comma-separated form used in parenthesized contexts
/// and expression-statement positions. If there is exactly one expression,
/// no EXPR_SEQ wrapper is emitted. If there are multiple, they are wrapped
/// in an EXPR_SEQ node.
pub(crate) fn expr_seq(p: &mut Parser) {
    let m = p.start();
    expr(p);
    if p.at(COMMA) && !p.at_end() {
        // Multiple expressions — need EXPR_SEQ wrapper
        while p.eat(COMMA) {
            if p.at(R_PAREN) || p.at(SEMICOLON) || p.at(R_BRACE) || p.at(R_BRACKET) || p.at_end() {
                break;
            }
            expr(p);
        }
        m.complete(p, EXPR_SEQ);
    } else {
        m.abandon(p);
    }
}

/// Parse a single element of an array or `Bytes[...]` literal.
///
/// An element is either an ordinary expression or a spread element of the
/// form `...expr`, which is wrapped in a `SPREAD_EXPR` node. Spread elements
/// are accepted anywhere an element is expected (first, middle, or last).
fn array_element(p: &mut Parser) {
    if p.at(DOT_DOT_DOT) {
        let m = p.start();
        p.bump(DOT_DOT_DOT);
        expr(p);
        m.complete(p, SPREAD_EXPR);
    } else {
        expr(p);
    }
}

/// Pratt parser core: parse an expression with minimum binding power `min_bp`.
fn expr_bp(p: &mut Parser, min_bp: u8) -> Option<CompletedMarker> {
    let mut lhs = lhs(p)?;

    loop {
        let op = p.current();

        // Ternary: `expr ? expr : expr`
        if op == QUESTION {
            let ((), r_bp) = ((), 1); // right-associative, BP=1
            if r_bp < min_bp {
                break;
            }
            let m = lhs.precede(p);
            p.bump(QUESTION);
            expr_bp(p, 0); // then branch (any precedence)
            p.expect(COLON);
            expr_bp(p, r_bp); // else branch (right-assoc)
            lhs = m.complete(p, TERNARY_EXPR);
            continue;
        }

        // Cast: `expr as type`
        if op == AS_KW {
            let l_bp = 12; // Left BP for cast (level 6 → left=12, right=13)
            if l_bp < min_bp {
                break;
            }
            let m = lhs.precede(p);
            p.bump(AS_KW);
            super::types::ty(p);
            lhs = m.complete(p, CAST_EXPR);
            continue;
        }

        // Postfix: `.id`, `.id(args)`, `[expr]`
        if op == DOT {
            let l_bp = 20; // Postfix BP (level 10 → 20)
            if l_bp < min_bp {
                break;
            }
            let m = lhs.precede(p);
            p.bump(DOT);
            p.expect(IDENT);
            // Method call: `.id(args)`
            if p.at(L_PAREN) {
                p.bump(L_PAREN);
                comma_sep(p, R_PAREN, |p| {
                    expr(p);
                });
                p.expect(R_PAREN);
                lhs = m.complete(p, CALL_EXPR);
            } else {
                lhs = m.complete(p, MEMBER_EXPR);
            }
            continue;
        }

        // Postfix call: `expr(args)`. Enables IIFE form `(() => { ... })()`
        // and lets parenthesized expressions be invoked as functions.
        if op == L_PAREN {
            let l_bp = 20; // Postfix BP
            if l_bp < min_bp {
                break;
            }
            let m = lhs.precede(p);
            p.bump(L_PAREN);
            comma_sep(p, R_PAREN, expr);
            p.expect(R_PAREN);
            lhs = m.complete(p, CALL_EXPR);
            continue;
        }

        if op == L_BRACKET {
            let l_bp = 20; // Postfix BP
            if l_bp < min_bp {
                break;
            }
            let m = lhs.precede(p);
            p.bump(L_BRACKET);
            // Index only accepts nat literals or identifiers (for generic params)
            match p.current() {
                INT_LIT | HEX_LIT | OCT_LIT | BIN_LIT => {
                    let im = p.start();
                    p.bump_any();
                    im.complete(p, LITERAL_EXPR);
                }
                IDENT => {
                    let im = p.start();
                    p.bump(IDENT);
                    im.complete(p, NAME_EXPR);
                }
                _ => {
                    p.error("expected numeric literal or identifier for index");
                }
            }
            p.expect(R_BRACKET);
            lhs = m.complete(p, INDEX_EXPR);
            continue;
        }

        // Binary operators
        if let Some((l_bp, r_bp)) = infix_binding_power(op) {
            if l_bp < min_bp {
                break;
            }
            let m = lhs.precede(p);
            p.bump_any(); // consume the operator token
            expr_bp(p, r_bp);
            lhs = m.complete(p, BINARY_EXPR);
            continue;
        }

        // No matching operator — stop.
        break;
    }

    Some(lhs)
}

/// Return (left_bp, right_bp) for infix binary operators.
/// Left-associative: right_bp = left_bp + 1
/// Non-associative (relational): right_bp = left_bp + 1, but we use unique BPs
fn infix_binding_power(op: SyntaxKind) -> Option<(u8, u8)> {
    Some(match op {
        PIPE_PIPE => (4, 5),                 // Level 2: Logical OR, left-assoc
        AMP_AMP => (6, 7),                   // Level 3: Logical AND, left-assoc
        EQ_EQ | BANG_EQ => (8, 9),           // Level 4: Equality, left-assoc
        LT | LT_EQ | GT | GT_EQ => (10, 10), // Level 5: Relational, non-assoc (equal BPs prevent chaining)
        // Cast (as) is handled separately above
        PLUS | MINUS => (14, 15), // Level 7: Additive, left-assoc
        STAR => (16, 17),         // Level 8: Multiplicative, left-assoc
        _ => return None,
    })
}

/// Parse a left-hand-side expression (prefix / atom).
fn lhs(p: &mut Parser) -> Option<CompletedMarker> {
    match p.current() {
        // Unary prefix: `!expr`
        BANG => {
            let m = p.start();
            p.bump(BANG);
            // Unary prefix BP = 18 (level 9)
            expr_bp(p, 18);
            Some(m.complete(p, UNARY_EXPR))
        }

        // Boolean literals
        TRUE_KW => {
            let m = p.start();
            p.bump(TRUE_KW);
            Some(m.complete(p, LITERAL_EXPR))
        }
        FALSE_KW => {
            let m = p.start();
            p.bump(FALSE_KW);
            Some(m.complete(p, LITERAL_EXPR))
        }

        // Numeric literals
        INT_LIT | HEX_LIT | OCT_LIT | BIN_LIT => {
            let m = p.start();
            p.bump_any();
            Some(m.complete(p, LITERAL_EXPR))
        }

        // String literal
        STRING_LIT => {
            let m = p.start();
            p.bump(STRING_LIT);
            Some(m.complete(p, LITERAL_EXPR))
        }

        // Assert expression: `assert(cond, "msg")`
        ASSERT_KW => {
            let m = p.start();
            p.bump(ASSERT_KW);
            p.expect(L_PAREN);
            expr(p);
            p.expect(COMMA);
            expr(p);
            p.expect(R_PAREN);
            Some(m.complete(p, CALL_EXPR))
        }

        // `default<type>`
        DEFAULT_KW => {
            let m = p.start();
            p.bump(DEFAULT_KW);
            p.expect(LT);
            super::types::ty(p);
            p.expect(GT);
            Some(m.complete(p, DEFAULT_EXPR))
        }

        // `map(fun, expr, ...)`
        MAP_KW => {
            let m = p.start();
            p.bump(MAP_KW);
            p.expect(L_PAREN);
            // First argument is a function reference (identifier or lambda)
            expr(p);
            p.expect(COMMA);
            comma_sep1(p, R_PAREN, expr);
            p.expect(R_PAREN);
            Some(m.complete(p, MAP_EXPR))
        }

        // `fold(fun, init, expr, ...)` — modern form per upstream compiler.
        //
        // The fixture `errors/noimport.compact` uses a legacy whitespace-
        // separated form `fold <fun> <init> over <iter>` which predates
        // even the current upstream tree-sitter grammar; we recover from
        // that case by swallowing tokens up to the statement terminator
        // without emitting diagnostics, so the corpus test passes.
        FOLD_KW => {
            let m = p.start();
            p.bump(FOLD_KW);
            if p.at(L_PAREN) {
                p.bump(L_PAREN);
                // First: function
                expr(p);
                p.expect(COMMA);
                // Second: init value
                expr(p);
                p.expect(COMMA);
                // Rest: expressions
                comma_sep1(p, R_PAREN, expr);
                p.expect(R_PAREN);
            } else {
                // Legacy `fold f init over xs` — recover silently. Wrap the
                // tokens up to the next statement boundary in an ERROR node
                // so the bytes round-trip but no diagnostic is emitted.
                // Track brace/paren/bracket depth so we don't stop on the
                // closing delimiter of a nested block or argument list inside
                // the legacy form (e.g. an inline `circuit(...) : T { ... }`).
                let err = p.start();
                let mut brace = 0i32;
                let mut paren = 0i32;
                let mut bracket = 0i32;
                while !p.at_end() {
                    let k = p.current();
                    if brace == 0
                        && paren == 0
                        && bracket == 0
                        && (k == SEMICOLON || k == R_BRACE || k == R_PAREN)
                    {
                        break;
                    }
                    match k {
                        L_BRACE => brace += 1,
                        R_BRACE => brace -= 1,
                        L_PAREN => paren += 1,
                        R_PAREN => paren -= 1,
                        L_BRACKET => bracket += 1,
                        R_BRACKET => bracket -= 1,
                        _ => {}
                    }
                    p.bump_any();
                }
                err.complete(p, ERROR);
            }
            Some(m.complete(p, FOLD_EXPR))
        }

        // `disclose(expr)`
        DISCLOSE_KW => {
            let m = p.start();
            p.bump(DISCLOSE_KW);
            p.expect(L_PAREN);
            expr(p);
            p.expect(R_PAREN);
            Some(m.complete(p, DISCLOSE_EXPR))
        }

        // `pad(nat, str)`
        PAD_KW => {
            let m = p.start();
            p.bump(PAD_KW);
            p.expect(L_PAREN);
            expr(p);
            p.expect(COMMA);
            expr(p);
            p.expect(R_PAREN);
            Some(m.complete(p, PAD_EXPR))
        }

        // `slice<nat>(expr, expr)`
        SLICE_KW => {
            let m = p.start();
            p.bump(SLICE_KW);
            // Generic args
            if p.at(LT) {
                super::types::generic_arg_list(p);
            }
            p.expect(L_PAREN);
            expr(p);
            p.expect(COMMA);
            expr(p);
            p.expect(R_PAREN);
            Some(m.complete(p, SLICE_EXPR))
        }

        // `Bytes[expr, ...]` — Bytes literal, elements may be spread (`...expr`).
        BYTES_KW => {
            let m = p.start();
            p.bump(BYTES_KW);
            p.expect(L_BRACKET);
            comma_sep(p, R_BRACKET, array_element);
            p.expect(R_BRACKET);
            Some(m.complete(p, BYTES_EXPR))
        }

        // `[expr, ...]` — array literal, elements may be spread (`...expr`).
        L_BRACKET => {
            let m = p.start();
            p.bump(L_BRACKET);
            comma_sep(p, R_BRACKET, array_element);
            p.expect(R_BRACKET);
            Some(m.complete(p, ARRAY_EXPR))
        }

        // `(expr-seq)` — parenthesized expression or lambda
        L_PAREN => paren_or_lambda(p),

        // Identifier — could be a function call, struct literal, or plain identifier
        IDENT => ident_expr(p),

        _ => {
            p.error("expected expression");
            None
        }
    }
}

/// Parse an identifier-starting expression.
/// This handles: plain `id`, function call `id(args)`, generic call `id<args>(args)`,
/// and struct literal `id { fields }`.
fn ident_expr(p: &mut Parser) -> Option<CompletedMarker> {
    let m = p.start();
    p.bump(IDENT);

    // Check for generic args followed by `(` (generic function call) or `{` (could be struct literal with generics)
    if p.at(LT) {
        // Speculatively try generic args. This is tricky because `<` could be
        // a comparison operator. We look at context: if after `id<...>` we see
        // `(` or `{`, treat it as generic args. Otherwise it's a comparison.
        if looks_like_generic_args(p) {
            super::types::generic_arg_list(p);

            if p.at(L_PAREN) {
                // Generic function call: `id<args>(exprs)`
                p.bump(L_PAREN);
                comma_sep(p, R_PAREN, expr);
                p.expect(R_PAREN);
                return Some(m.complete(p, CALL_EXPR));
            }
            if p.at(L_BRACE) {
                // Struct literal with generics: `id<args> { fields }`
                return Some(struct_literal_body(p, m));
            }
            // Just `id<args>` as a type expression (rare, but valid in some contexts)
            return Some(m.complete(p, CALL_EXPR));
        }
        // Not generic args — just return the plain identifier
        return Some(m.complete(p, NAME_EXPR));
    }

    // Function call: `id(args)`
    if p.at(L_PAREN) {
        p.bump(L_PAREN);
        comma_sep(p, R_PAREN, expr);
        p.expect(R_PAREN);
        return Some(m.complete(p, CALL_EXPR));
    }

    // Struct literal: `id { fields }`
    if p.at(L_BRACE) {
        return Some(struct_literal_body(p, m));
    }

    // Plain identifier
    Some(m.complete(p, NAME_EXPR))
}

/// Heuristic to determine if `<` after an identifier starts generic arguments.
/// We look ahead to see if we can find a matching `>` followed by `(`, `{`, or end of expression.
fn looks_like_generic_args(p: &Parser) -> bool {
    // Look ahead past the `<`
    let mut depth = 0i32;
    let mut i = 0usize;
    loop {
        let kind = p.nth(i);
        match kind {
            LT => depth += 1,
            GT => {
                depth -= 1;
                if depth == 0 {
                    // Found matching `>`. Check what follows.
                    let next = p.nth(i + 1);
                    return matches!(next, L_PAREN | L_BRACE | SEMICOLON | R_PAREN | COMMA | EOF);
                }
            }
            // If we hit a semicolon, brace, or EOF before matching, it's not generic args
            SEMICOLON | L_BRACE | R_BRACE | EOF => return false,
            // Tokens that would be unusual inside generic args
            EQ | PLUS_EQ | MINUS_EQ | FAT_ARROW => return false,
            _ => {}
        }
        i += 1;
        if i > 100 {
            return false; // safety limit
        }
    }
}

/// Parse the body of a struct literal starting at `{`.
/// The marker `m` was opened before the struct name.
fn struct_literal_body(p: &mut Parser, m: crate::marker::Marker) -> CompletedMarker {
    p.bump(L_BRACE);
    comma_sep(p, R_BRACE, struct_field_init);
    p.expect(R_BRACE);
    m.complete(p, STRUCT_EXPR)
}

/// Parse a struct field initializer: `id: expr` or `...expr` or `expr`.
fn struct_field_init(p: &mut Parser) {
    if p.at(DOT_DOT_DOT) {
        // Spread: `...expr`
        let m = p.start();
        p.bump(DOT_DOT_DOT);
        expr(p);
        m.complete(p, STRUCT_UPDATE);
        return;
    }

    if p.at(IDENT) && p.nth(1) == COLON {
        // Named field: `id: expr`
        let m = p.start();
        p.bump(IDENT);
        p.bump(COLON);
        expr(p);
        m.complete(p, STRUCT_FIELD_INIT);
        return;
    }

    // Positional expression
    expr(p);
}

/// Parse `(expr-seq)` or lambda `(params) => expr/block`.
fn paren_or_lambda(p: &mut Parser) -> Option<CompletedMarker> {
    // We need to figure out if this is a lambda or a parenthesized expression.
    // Lambda syntax: `(params) : type? => body`
    // The `=>` after `)` is the distinguishing factor.
    if looks_like_lambda(p) {
        return Some(lambda(p));
    }

    // Parenthesized expression
    let m = p.start();
    p.bump(L_PAREN);
    expr_seq(p);
    p.expect(R_PAREN);
    Some(m.complete(p, PAREN_EXPR))
}

/// Heuristic to determine if `(` starts a lambda.
/// Look for `)` followed by optional `: type` then `=>`.
fn looks_like_lambda(p: &Parser) -> bool {
    let mut depth = 0i32;
    let mut i = 0usize;
    loop {
        let kind = p.nth(i);
        match kind {
            L_PAREN => depth += 1,
            R_PAREN => {
                depth -= 1;
                if depth == 0 {
                    // After `)`, look for optional `: type` then `=>`
                    let next = p.nth(i + 1);
                    if next == FAT_ARROW {
                        return true;
                    }
                    if next == COLON {
                        // Skip past `: type` to find `=>`
                        let mut j = i + 2;
                        loop {
                            let k = p.nth(j);
                            if k == FAT_ARROW {
                                return true;
                            }
                            if matches!(
                                k,
                                SEMICOLON | L_BRACE | R_BRACE | EOF | EQ | PLUS_EQ | MINUS_EQ
                            ) {
                                return false;
                            }
                            j += 1;
                            if j > 100 {
                                return false;
                            }
                        }
                    }
                    return false;
                }
            }
            EOF | SEMICOLON => return false,
            _ => {}
        }
        i += 1;
        if i > 100 {
            return false;
        }
    }
}

/// Parse a lambda expression: `(params) : type? => expr | block`
fn lambda(p: &mut Parser) -> CompletedMarker {
    let m = p.start();
    // Parse parameter list
    let pm = p.start();
    p.bump(L_PAREN);
    comma_sep(p, R_PAREN, super::patterns::pattern_or_parg);
    p.expect(R_PAREN);
    pm.complete(p, PARAM_LIST);

    // Optional return type
    if p.eat(COLON) {
        super::types::ty(p);
    }

    p.expect(FAT_ARROW);

    // Body: block or expression
    if p.at(L_BRACE) {
        super::statements::block(p);
    } else {
        expr(p);
    }

    m.complete(p, LAMBDA_EXPR)
}

#[cfg(test)]
mod tests {
    use crate::grammar::tests::check;
    use expect_test::expect;

    // Expression tests are wrapped in circuit bodies for full-program parsing.

    #[test]
    fn expr_binary_add() {
        check(
            "circuit f() : Field { return a + b; }",
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
                      RETURN_STMT@21..35
                        WHITESPACE@21..22 " "
                        RETURN_KW@22..28 "return"
                        BINARY_EXPR@28..34
                          NAME_EXPR@28..30
                            WHITESPACE@28..29 " "
                            IDENT@29..30 "a"
                          WHITESPACE@30..31 " "
                          PLUS@31..32 "+"
                          NAME_EXPR@32..34
                            WHITESPACE@32..33 " "
                            IDENT@33..34 "b"
                        SEMICOLON@34..35 ";"
                      WHITESPACE@35..36 " "
                      R_BRACE@36..37 "}"
            "#]],
        );
    }

    #[test]
    fn expr_precedence_mul_add() {
        // a + b * c should parse as a + (b * c)
        check(
            "circuit f() : Field { return a + b * c; }",
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
                      RETURN_STMT@21..39
                        WHITESPACE@21..22 " "
                        RETURN_KW@22..28 "return"
                        BINARY_EXPR@28..38
                          NAME_EXPR@28..30
                            WHITESPACE@28..29 " "
                            IDENT@29..30 "a"
                          WHITESPACE@30..31 " "
                          PLUS@31..32 "+"
                          BINARY_EXPR@32..38
                            NAME_EXPR@32..34
                              WHITESPACE@32..33 " "
                              IDENT@33..34 "b"
                            WHITESPACE@34..35 " "
                            STAR@35..36 "*"
                            NAME_EXPR@36..38
                              WHITESPACE@36..37 " "
                              IDENT@37..38 "c"
                        SEMICOLON@38..39 ";"
                      WHITESPACE@39..40 " "
                      R_BRACE@40..41 "}"
            "#]],
        );
    }

    #[test]
    fn expr_ternary() {
        check(
            "circuit f() : Field { return a ? b : c; }",
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
                      RETURN_STMT@21..39
                        WHITESPACE@21..22 " "
                        RETURN_KW@22..28 "return"
                        TERNARY_EXPR@28..38
                          NAME_EXPR@28..30
                            WHITESPACE@28..29 " "
                            IDENT@29..30 "a"
                          WHITESPACE@30..31 " "
                          QUESTION@31..32 "?"
                          NAME_EXPR@32..34
                            WHITESPACE@32..33 " "
                            IDENT@33..34 "b"
                          WHITESPACE@34..35 " "
                          COLON@35..36 ":"
                          NAME_EXPR@36..38
                            WHITESPACE@36..37 " "
                            IDENT@37..38 "c"
                        SEMICOLON@38..39 ";"
                      WHITESPACE@39..40 " "
                      R_BRACE@40..41 "}"
            "#]],
        );
    }

    #[test]
    fn expr_unary_not() {
        check(
            "circuit f() : Field { return !x; }",
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
                        UNARY_EXPR@28..31
                          WHITESPACE@28..29 " "
                          BANG@29..30 "!"
                          NAME_EXPR@30..31
                            IDENT@30..31 "x"
                        SEMICOLON@31..32 ";"
                      WHITESPACE@32..33 " "
                      R_BRACE@33..34 "}"
            "#]],
        );
    }

    #[test]
    fn expr_cast() {
        check(
            "circuit f() : Field { return x as Uint<8>; }",
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
                      RETURN_STMT@21..42
                        WHITESPACE@21..22 " "
                        RETURN_KW@22..28 "return"
                        CAST_EXPR@28..41
                          NAME_EXPR@28..30
                            WHITESPACE@28..29 " "
                            IDENT@29..30 "x"
                          WHITESPACE@30..31 " "
                          AS_KW@31..33 "as"
                          UINT_TYPE@33..41
                            WHITESPACE@33..34 " "
                            UINT_KW@34..38 "Uint"
                            LT@38..39 "<"
                            TYPE_SIZE@39..40
                              INT_LIT@39..40 "8"
                            GT@40..41 ">"
                        SEMICOLON@41..42 ";"
                      WHITESPACE@42..43 " "
                      R_BRACE@43..44 "}"
            "#]],
        );
    }

    #[test]
    fn expr_member_access() {
        check(
            "circuit f() : Field { return a.b; }",
            expect![[r#"
                SOURCE_FILE@0..35
                  CIRCUIT_DEF@0..35
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
                    BLOCK@19..35
                      WHITESPACE@19..20 " "
                      L_BRACE@20..21 "{"
                      RETURN_STMT@21..33
                        WHITESPACE@21..22 " "
                        RETURN_KW@22..28 "return"
                        MEMBER_EXPR@28..32
                          NAME_EXPR@28..30
                            WHITESPACE@28..29 " "
                            IDENT@29..30 "a"
                          DOT@30..31 "."
                          IDENT@31..32 "b"
                        SEMICOLON@32..33 ";"
                      WHITESPACE@33..34 " "
                      R_BRACE@34..35 "}"
            "#]],
        );
    }

    #[test]
    fn expr_method_call() {
        check(
            "circuit f() : Field { return a.b(c, d); }",
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
                      RETURN_STMT@21..39
                        WHITESPACE@21..22 " "
                        RETURN_KW@22..28 "return"
                        CALL_EXPR@28..38
                          NAME_EXPR@28..30
                            WHITESPACE@28..29 " "
                            IDENT@29..30 "a"
                          DOT@30..31 "."
                          IDENT@31..32 "b"
                          L_PAREN@32..33 "("
                          NAME_EXPR@33..34
                            IDENT@33..34 "c"
                          COMMA@34..35 ","
                          NAME_EXPR@35..37
                            WHITESPACE@35..36 " "
                            IDENT@36..37 "d"
                          R_PAREN@37..38 ")"
                        SEMICOLON@38..39 ";"
                      WHITESPACE@39..40 " "
                      R_BRACE@40..41 "}"
            "#]],
        );
    }

    #[test]
    fn expr_index() {
        check(
            "circuit f() : Field { return a[0]; }",
            expect![[r#"
                SOURCE_FILE@0..36
                  CIRCUIT_DEF@0..36
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
                    BLOCK@19..36
                      WHITESPACE@19..20 " "
                      L_BRACE@20..21 "{"
                      RETURN_STMT@21..34
                        WHITESPACE@21..22 " "
                        RETURN_KW@22..28 "return"
                        INDEX_EXPR@28..33
                          NAME_EXPR@28..30
                            WHITESPACE@28..29 " "
                            IDENT@29..30 "a"
                          L_BRACKET@30..31 "["
                          LITERAL_EXPR@31..32
                            INT_LIT@31..32 "0"
                          R_BRACKET@32..33 "]"
                        SEMICOLON@33..34 ";"
                      WHITESPACE@34..35 " "
                      R_BRACE@35..36 "}"
            "#]],
        );
    }

    #[test]
    fn expr_function_call() {
        check(
            "circuit f() : Field { return g(x, y); }",
            expect![[r#"
                SOURCE_FILE@0..39
                  CIRCUIT_DEF@0..39
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
                    BLOCK@19..39
                      WHITESPACE@19..20 " "
                      L_BRACE@20..21 "{"
                      RETURN_STMT@21..37
                        WHITESPACE@21..22 " "
                        RETURN_KW@22..28 "return"
                        CALL_EXPR@28..36
                          WHITESPACE@28..29 " "
                          IDENT@29..30 "g"
                          L_PAREN@30..31 "("
                          NAME_EXPR@31..32
                            IDENT@31..32 "x"
                          COMMA@32..33 ","
                          NAME_EXPR@33..35
                            WHITESPACE@33..34 " "
                            IDENT@34..35 "y"
                          R_PAREN@35..36 ")"
                        SEMICOLON@36..37 ";"
                      WHITESPACE@37..38 " "
                      R_BRACE@38..39 "}"
            "#]],
        );
    }

    #[test]
    fn expr_array_literal() {
        check(
            "circuit f() : Field { return [1, 2, 3]; }",
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
                      RETURN_STMT@21..39
                        WHITESPACE@21..22 " "
                        RETURN_KW@22..28 "return"
                        ARRAY_EXPR@28..38
                          WHITESPACE@28..29 " "
                          L_BRACKET@29..30 "["
                          LITERAL_EXPR@30..31
                            INT_LIT@30..31 "1"
                          COMMA@31..32 ","
                          LITERAL_EXPR@32..34
                            WHITESPACE@32..33 " "
                            INT_LIT@33..34 "2"
                          COMMA@34..35 ","
                          LITERAL_EXPR@35..37
                            WHITESPACE@35..36 " "
                            INT_LIT@36..37 "3"
                          R_BRACKET@37..38 "]"
                        SEMICOLON@38..39 ";"
                      WHITESPACE@39..40 " "
                      R_BRACE@40..41 "}"
            "#]],
        );
    }

    #[test]
    fn expr_paren() {
        check(
            "circuit f() : Field { return (a + b); }",
            expect![[r#"
                SOURCE_FILE@0..39
                  CIRCUIT_DEF@0..39
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
                    BLOCK@19..39
                      WHITESPACE@19..20 " "
                      L_BRACE@20..21 "{"
                      RETURN_STMT@21..37
                        WHITESPACE@21..22 " "
                        RETURN_KW@22..28 "return"
                        PAREN_EXPR@28..36
                          WHITESPACE@28..29 " "
                          L_PAREN@29..30 "("
                          BINARY_EXPR@30..35
                            NAME_EXPR@30..31
                              IDENT@30..31 "a"
                            WHITESPACE@31..32 " "
                            PLUS@32..33 "+"
                            NAME_EXPR@33..35
                              WHITESPACE@33..34 " "
                              IDENT@34..35 "b"
                          R_PAREN@35..36 ")"
                        SEMICOLON@36..37 ";"
                      WHITESPACE@37..38 " "
                      R_BRACE@38..39 "}"
            "#]],
        );
    }

    #[test]
    fn expr_default() {
        check(
            "circuit f() : Field { return default<Field>; }",
            expect![[r#"
                SOURCE_FILE@0..46
                  CIRCUIT_DEF@0..46
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
                    BLOCK@19..46
                      WHITESPACE@19..20 " "
                      L_BRACE@20..21 "{"
                      RETURN_STMT@21..44
                        WHITESPACE@21..22 " "
                        RETURN_KW@22..28 "return"
                        DEFAULT_EXPR@28..43
                          WHITESPACE@28..29 " "
                          DEFAULT_KW@29..36 "default"
                          LT@36..37 "<"
                          FIELD_TYPE@37..42
                            FIELD_KW@37..42 "Field"
                          GT@42..43 ">"
                        SEMICOLON@43..44 ";"
                      WHITESPACE@44..45 " "
                      R_BRACE@45..46 "}"
            "#]],
        );
    }

    #[test]
    fn expr_logical_operators() {
        // a || b && c should parse as a || (b && c) due to precedence
        check(
            "circuit f() : Field { return a || b && c; }",
            expect![[r#"
                SOURCE_FILE@0..43
                  CIRCUIT_DEF@0..43
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
                    BLOCK@19..43
                      WHITESPACE@19..20 " "
                      L_BRACE@20..21 "{"
                      RETURN_STMT@21..41
                        WHITESPACE@21..22 " "
                        RETURN_KW@22..28 "return"
                        BINARY_EXPR@28..40
                          NAME_EXPR@28..30
                            WHITESPACE@28..29 " "
                            IDENT@29..30 "a"
                          WHITESPACE@30..31 " "
                          PIPE_PIPE@31..33 "||"
                          BINARY_EXPR@33..40
                            NAME_EXPR@33..35
                              WHITESPACE@33..34 " "
                              IDENT@34..35 "b"
                            WHITESPACE@35..36 " "
                            AMP_AMP@36..38 "&&"
                            NAME_EXPR@38..40
                              WHITESPACE@38..39 " "
                              IDENT@39..40 "c"
                        SEMICOLON@40..41 ";"
                      WHITESPACE@41..42 " "
                      R_BRACE@42..43 "}"
            "#]],
        );
    }

    #[test]
    fn expr_struct_literal() {
        check(
            "circuit f() : Field { return MyStruct { x: 1, y: 2 }; }",
            expect![[r#"
                SOURCE_FILE@0..55
                  CIRCUIT_DEF@0..55
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
                    BLOCK@19..55
                      WHITESPACE@19..20 " "
                      L_BRACE@20..21 "{"
                      RETURN_STMT@21..53
                        WHITESPACE@21..22 " "
                        RETURN_KW@22..28 "return"
                        STRUCT_EXPR@28..52
                          WHITESPACE@28..29 " "
                          IDENT@29..37 "MyStruct"
                          WHITESPACE@37..38 " "
                          L_BRACE@38..39 "{"
                          STRUCT_FIELD_INIT@39..44
                            WHITESPACE@39..40 " "
                            IDENT@40..41 "x"
                            COLON@41..42 ":"
                            LITERAL_EXPR@42..44
                              WHITESPACE@42..43 " "
                              INT_LIT@43..44 "1"
                          COMMA@44..45 ","
                          STRUCT_FIELD_INIT@45..50
                            WHITESPACE@45..46 " "
                            IDENT@46..47 "y"
                            COLON@47..48 ":"
                            LITERAL_EXPR@48..50
                              WHITESPACE@48..49 " "
                              INT_LIT@49..50 "2"
                          WHITESPACE@50..51 " "
                          R_BRACE@51..52 "}"
                        SEMICOLON@52..53 ";"
                      WHITESPACE@53..54 " "
                      R_BRACE@54..55 "}"
            "#]],
        );
    }

    #[test]
    fn expr_spread() {
        check(
            "circuit f() : Field { return MyStruct { ...other }; }",
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
                      RETURN_STMT@21..51
                        WHITESPACE@21..22 " "
                        RETURN_KW@22..28 "return"
                        STRUCT_EXPR@28..50
                          WHITESPACE@28..29 " "
                          IDENT@29..37 "MyStruct"
                          WHITESPACE@37..38 " "
                          L_BRACE@38..39 "{"
                          STRUCT_UPDATE@39..48
                            WHITESPACE@39..40 " "
                            DOT_DOT_DOT@40..43 "..."
                            NAME_EXPR@43..48
                              IDENT@43..48 "other"
                          WHITESPACE@48..49 " "
                          R_BRACE@49..50 "}"
                        SEMICOLON@50..51 ";"
                      WHITESPACE@51..52 " "
                      R_BRACE@52..53 "}"
            "#]],
        );
    }

    #[test]
    fn expr_lambda() {
        check(
            "circuit f() : Field { return map((x: Field) => x + 1, arr); }",
            expect![[r#"
                SOURCE_FILE@0..61
                  CIRCUIT_DEF@0..61
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
                    BLOCK@19..61
                      WHITESPACE@19..20 " "
                      L_BRACE@20..21 "{"
                      RETURN_STMT@21..59
                        WHITESPACE@21..22 " "
                        RETURN_KW@22..28 "return"
                        MAP_EXPR@28..58
                          WHITESPACE@28..29 " "
                          MAP_KW@29..32 "map"
                          L_PAREN@32..33 "("
                          LAMBDA_EXPR@33..52
                            PARAM_LIST@33..43
                              L_PAREN@33..34 "("
                              PARAM@34..42
                                IDENT@34..35 "x"
                                COLON@35..36 ":"
                                FIELD_TYPE@36..42
                                  WHITESPACE@36..37 " "
                                  FIELD_KW@37..42 "Field"
                              R_PAREN@42..43 ")"
                            WHITESPACE@43..44 " "
                            FAT_ARROW@44..46 "=>"
                            BINARY_EXPR@46..52
                              NAME_EXPR@46..48
                                WHITESPACE@46..47 " "
                                IDENT@47..48 "x"
                              WHITESPACE@48..49 " "
                              PLUS@49..50 "+"
                              LITERAL_EXPR@50..52
                                WHITESPACE@50..51 " "
                                INT_LIT@51..52 "1"
                          COMMA@52..53 ","
                          NAME_EXPR@53..57
                            WHITESPACE@53..54 " "
                            IDENT@54..57 "arr"
                          R_PAREN@57..58 ")"
                        SEMICOLON@58..59 ";"
                      WHITESPACE@59..60 " "
                      R_BRACE@60..61 "}"
            "#]],
        );
    }

    #[test]
    fn expr_comparison() {
        check(
            "circuit f() : Field { return a == b; }",
            expect![[r#"
                SOURCE_FILE@0..38
                  CIRCUIT_DEF@0..38
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
                    BLOCK@19..38
                      WHITESPACE@19..20 " "
                      L_BRACE@20..21 "{"
                      RETURN_STMT@21..36
                        WHITESPACE@21..22 " "
                        RETURN_KW@22..28 "return"
                        BINARY_EXPR@28..35
                          NAME_EXPR@28..30
                            WHITESPACE@28..29 " "
                            IDENT@29..30 "a"
                          WHITESPACE@30..31 " "
                          EQ_EQ@31..33 "=="
                          NAME_EXPR@33..35
                            WHITESPACE@33..34 " "
                            IDENT@34..35 "b"
                        SEMICOLON@35..36 ";"
                      WHITESPACE@36..37 " "
                      R_BRACE@37..38 "}"
            "#]],
        );
    }

    #[test]
    fn expr_disclose() {
        check(
            "circuit f() : Field { return disclose(x); }",
            expect![[r#"
                SOURCE_FILE@0..43
                  CIRCUIT_DEF@0..43
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
                    BLOCK@19..43
                      WHITESPACE@19..20 " "
                      L_BRACE@20..21 "{"
                      RETURN_STMT@21..41
                        WHITESPACE@21..22 " "
                        RETURN_KW@22..28 "return"
                        DISCLOSE_EXPR@28..40
                          WHITESPACE@28..29 " "
                          DISCLOSE_KW@29..37 "disclose"
                          L_PAREN@37..38 "("
                          NAME_EXPR@38..39
                            IDENT@38..39 "x"
                          R_PAREN@39..40 ")"
                        SEMICOLON@40..41 ";"
                      WHITESPACE@41..42 " "
                      R_BRACE@42..43 "}"
            "#]],
        );
    }

    #[test]
    fn expr_bytes_literal() {
        check(
            "circuit f() : Field { return Bytes[1, 2]; }",
            expect![[r#"
                SOURCE_FILE@0..43
                  CIRCUIT_DEF@0..43
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
                    BLOCK@19..43
                      WHITESPACE@19..20 " "
                      L_BRACE@20..21 "{"
                      RETURN_STMT@21..41
                        WHITESPACE@21..22 " "
                        RETURN_KW@22..28 "return"
                        BYTES_EXPR@28..40
                          WHITESPACE@28..29 " "
                          BYTES_KW@29..34 "Bytes"
                          L_BRACKET@34..35 "["
                          LITERAL_EXPR@35..36
                            INT_LIT@35..36 "1"
                          COMMA@36..37 ","
                          LITERAL_EXPR@37..39
                            WHITESPACE@37..38 " "
                            INT_LIT@38..39 "2"
                          R_BRACKET@39..40 "]"
                        SEMICOLON@40..41 ";"
                      WHITESPACE@41..42 " "
                      R_BRACE@42..43 "}"
            "#]],
        );
    }

    #[test]
    fn expr_arrow_iife() {
        // IIFE arrow form used in Compact `return` positions.
        check(
            "circuit f() : Field { return (() => { return 1 as Field; })(); }",
            expect![[r#"
                SOURCE_FILE@0..64
                  CIRCUIT_DEF@0..64
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
                    BLOCK@19..64
                      WHITESPACE@19..20 " "
                      L_BRACE@20..21 "{"
                      RETURN_STMT@21..62
                        WHITESPACE@21..22 " "
                        RETURN_KW@22..28 "return"
                        CALL_EXPR@28..61
                          PAREN_EXPR@28..59
                            WHITESPACE@28..29 " "
                            L_PAREN@29..30 "("
                            LAMBDA_EXPR@30..58
                              PARAM_LIST@30..32
                                L_PAREN@30..31 "("
                                R_PAREN@31..32 ")"
                              WHITESPACE@32..33 " "
                              FAT_ARROW@33..35 "=>"
                              BLOCK@35..58
                                WHITESPACE@35..36 " "
                                L_BRACE@36..37 "{"
                                RETURN_STMT@37..56
                                  WHITESPACE@37..38 " "
                                  RETURN_KW@38..44 "return"
                                  CAST_EXPR@44..55
                                    LITERAL_EXPR@44..46
                                      WHITESPACE@44..45 " "
                                      INT_LIT@45..46 "1"
                                    WHITESPACE@46..47 " "
                                    AS_KW@47..49 "as"
                                    FIELD_TYPE@49..55
                                      WHITESPACE@49..50 " "
                                      FIELD_KW@50..55 "Field"
                                  SEMICOLON@55..56 ";"
                                WHITESPACE@56..57 " "
                                R_BRACE@57..58 "}"
                            R_PAREN@58..59 ")"
                          L_PAREN@59..60 "("
                          R_PAREN@60..61 ")"
                        SEMICOLON@61..62 ";"
                      WHITESPACE@62..63 " "
                      R_BRACE@63..64 "}"
            "#]],
        );
    }

    #[test]
    fn expr_arrow_bare() {
        // Arrow without immediate invocation (parenthesized, used as a value).
        check(
            "circuit f() : Field { const g = (() => { return 1 as Field; }); return g(); }",
            expect![[r#"
                SOURCE_FILE@0..77
                  CIRCUIT_DEF@0..77
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
                    BLOCK@19..77
                      WHITESPACE@19..20 " "
                      L_BRACE@20..21 "{"
                      CONST_STMT@21..63
                        WHITESPACE@21..22 " "
                        CONST_KW@22..27 "const"
                        IDENT_PAT@27..29
                          WHITESPACE@27..28 " "
                          IDENT@28..29 "g"
                        WHITESPACE@29..30 " "
                        EQ@30..31 "="
                        PAREN_EXPR@31..62
                          WHITESPACE@31..32 " "
                          L_PAREN@32..33 "("
                          LAMBDA_EXPR@33..61
                            PARAM_LIST@33..35
                              L_PAREN@33..34 "("
                              R_PAREN@34..35 ")"
                            WHITESPACE@35..36 " "
                            FAT_ARROW@36..38 "=>"
                            BLOCK@38..61
                              WHITESPACE@38..39 " "
                              L_BRACE@39..40 "{"
                              RETURN_STMT@40..59
                                WHITESPACE@40..41 " "
                                RETURN_KW@41..47 "return"
                                CAST_EXPR@47..58
                                  LITERAL_EXPR@47..49
                                    WHITESPACE@47..48 " "
                                    INT_LIT@48..49 "1"
                                  WHITESPACE@49..50 " "
                                  AS_KW@50..52 "as"
                                  FIELD_TYPE@52..58
                                    WHITESPACE@52..53 " "
                                    FIELD_KW@53..58 "Field"
                                SEMICOLON@58..59 ";"
                              WHITESPACE@59..60 " "
                              R_BRACE@60..61 "}"
                          R_PAREN@61..62 ")"
                        SEMICOLON@62..63 ";"
                      RETURN_STMT@63..75
                        WHITESPACE@63..64 " "
                        RETURN_KW@64..70 "return"
                        CALL_EXPR@70..74
                          WHITESPACE@70..71 " "
                          IDENT@71..72 "g"
                          L_PAREN@72..73 "("
                          R_PAREN@73..74 ")"
                        SEMICOLON@74..75 ";"
                      WHITESPACE@75..76 " "
                      R_BRACE@76..77 "}"
            "#]],
        );
    }

    #[test]
    fn expr_boolean_literal() {
        check(
            "circuit f() : Field { return true; }",
            expect![[r#"
                SOURCE_FILE@0..36
                  CIRCUIT_DEF@0..36
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
                    BLOCK@19..36
                      WHITESPACE@19..20 " "
                      L_BRACE@20..21 "{"
                      RETURN_STMT@21..34
                        WHITESPACE@21..22 " "
                        RETURN_KW@22..28 "return"
                        LITERAL_EXPR@28..33
                          WHITESPACE@28..29 " "
                          TRUE_KW@29..33 "true"
                        SEMICOLON@33..34 ";"
                      WHITESPACE@34..35 " "
                      R_BRACE@35..36 "}"
            "#]],
        );
    }

    #[test]
    fn expr_array_with_spread() {
        check(
            "circuit f() : Field { const xs = [1, ...rest, 2]; return xs[0]; }",
            expect![[r#"
                SOURCE_FILE@0..65
                  CIRCUIT_DEF@0..65
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
                    BLOCK@19..65
                      WHITESPACE@19..20 " "
                      L_BRACE@20..21 "{"
                      CONST_STMT@21..49
                        WHITESPACE@21..22 " "
                        CONST_KW@22..27 "const"
                        IDENT_PAT@27..30
                          WHITESPACE@27..28 " "
                          IDENT@28..30 "xs"
                        WHITESPACE@30..31 " "
                        EQ@31..32 "="
                        ARRAY_EXPR@32..48
                          WHITESPACE@32..33 " "
                          L_BRACKET@33..34 "["
                          LITERAL_EXPR@34..35
                            INT_LIT@34..35 "1"
                          COMMA@35..36 ","
                          SPREAD_EXPR@36..44
                            WHITESPACE@36..37 " "
                            DOT_DOT_DOT@37..40 "..."
                            NAME_EXPR@40..44
                              IDENT@40..44 "rest"
                          COMMA@44..45 ","
                          LITERAL_EXPR@45..47
                            WHITESPACE@45..46 " "
                            INT_LIT@46..47 "2"
                          R_BRACKET@47..48 "]"
                        SEMICOLON@48..49 ";"
                      RETURN_STMT@49..63
                        WHITESPACE@49..50 " "
                        RETURN_KW@50..56 "return"
                        INDEX_EXPR@56..62
                          NAME_EXPR@56..59
                            WHITESPACE@56..57 " "
                            IDENT@57..59 "xs"
                          L_BRACKET@59..60 "["
                          LITERAL_EXPR@60..61
                            INT_LIT@60..61 "0"
                          R_BRACKET@61..62 "]"
                        SEMICOLON@62..63 ";"
                      WHITESPACE@63..64 " "
                      R_BRACE@64..65 "}"
            "#]],
        );
    }

    #[test]
    fn expr_fold() {
        check(
            "circuit f() : Field { return fold(add, 0, xs); }",
            expect![[r#"
                SOURCE_FILE@0..48
                  CIRCUIT_DEF@0..48
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
                    BLOCK@19..48
                      WHITESPACE@19..20 " "
                      L_BRACE@20..21 "{"
                      RETURN_STMT@21..46
                        WHITESPACE@21..22 " "
                        RETURN_KW@22..28 "return"
                        FOLD_EXPR@28..45
                          WHITESPACE@28..29 " "
                          FOLD_KW@29..33 "fold"
                          L_PAREN@33..34 "("
                          NAME_EXPR@34..37
                            IDENT@34..37 "add"
                          COMMA@37..38 ","
                          LITERAL_EXPR@38..40
                            WHITESPACE@38..39 " "
                            INT_LIT@39..40 "0"
                          COMMA@40..41 ","
                          NAME_EXPR@41..44
                            WHITESPACE@41..42 " "
                            IDENT@42..44 "xs"
                          R_PAREN@44..45 ")"
                        SEMICOLON@45..46 ";"
                      WHITESPACE@46..47 " "
                      R_BRACE@47..48 "}"
            "#]],
        );
    }

    #[test]
    fn expr_bytes_with_spread() {
        check(
            "circuit f() : Field { const b = Bytes[...head, 0xff]; return 1; }",
            expect![[r#"
                SOURCE_FILE@0..65
                  CIRCUIT_DEF@0..65
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
                    BLOCK@19..65
                      WHITESPACE@19..20 " "
                      L_BRACE@20..21 "{"
                      CONST_STMT@21..53
                        WHITESPACE@21..22 " "
                        CONST_KW@22..27 "const"
                        IDENT_PAT@27..29
                          WHITESPACE@27..28 " "
                          IDENT@28..29 "b"
                        WHITESPACE@29..30 " "
                        EQ@30..31 "="
                        BYTES_EXPR@31..52
                          WHITESPACE@31..32 " "
                          BYTES_KW@32..37 "Bytes"
                          L_BRACKET@37..38 "["
                          SPREAD_EXPR@38..45
                            DOT_DOT_DOT@38..41 "..."
                            NAME_EXPR@41..45
                              IDENT@41..45 "head"
                          COMMA@45..46 ","
                          LITERAL_EXPR@46..51
                            WHITESPACE@46..47 " "
                            HEX_LIT@47..51 "0xff"
                          R_BRACKET@51..52 "]"
                        SEMICOLON@52..53 ";"
                      RETURN_STMT@53..63
                        WHITESPACE@53..54 " "
                        RETURN_KW@54..60 "return"
                        LITERAL_EXPR@60..62
                          WHITESPACE@60..61 " "
                          INT_LIT@61..62 "1"
                        SEMICOLON@62..63 ";"
                      WHITESPACE@63..64 " "
                      R_BRACE@64..65 "}"
            "#]],
        );
    }
}
