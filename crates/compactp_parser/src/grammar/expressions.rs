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

/// Parse an expression that may optionally end in an assignment or
/// compound-assignment tail (`= rhs`, `+= rhs`, `-= rhs`).
///
/// Use this in positions where Compact accepts an assignment as a
/// sub-expression — `const x = (y = 1)`, `return field = x`,
/// `(y) => field = y`, parenthesized contexts — but where the
/// statement-level [`super::statements::ASSIGN_STMT`] form is not
/// being parsed.
pub(crate) fn expr_or_assign(p: &mut Parser) {
    let m = p.start();
    expr(p);
    match p.current() {
        EQ => {
            p.bump(EQ);
            expr(p);
            m.complete(p, ASSIGN_EXPR);
        }
        PLUS_EQ | MINUS_EQ => {
            p.bump_any();
            expr(p);
            m.complete(p, COMPOUND_ASSIGN_EXPR);
        }
        _ => {
            m.abandon(p);
        }
    }
}

/// Parse a single call-expression argument.
///
/// Accepts the ordinary positional form `expr` and the named form
/// `IDENT = expr`. The named form is selected only when the current token
/// is an identifier and the immediately following token is `=` — this is
/// unambiguous inside an argument list because plain assignment is a
/// statement, not an expression, in Compact.
fn call_arg(p: &mut Parser) {
    if p.at(IDENT) && p.nth(1) == EQ {
        let m = p.start();
        p.bump(IDENT);
        p.bump(EQ);
        expr(p);
        m.complete(p, NAMED_ARG);
    } else {
        expr(p);
    }
}

/// Parse an expression sequence: `expr` or `expr, ..., expr, expr`.
///
/// Used in parenthesized contexts (`( ... )`, `if ( ... )`,
/// `for ( const id of ... )`) and in `return` statements. If there
/// is exactly one element, no `EXPR_SEQ` wrapper is emitted; if there
/// are multiple, they are wrapped in an `EXPR_SEQ` node.
///
/// Each element may itself be an assignment or compound-assignment
/// form (`lhs = rhs`, `lhs += rhs`, `lhs -= rhs`), wrapped in
/// `ASSIGN_EXPR` or `COMPOUND_ASSIGN_EXPR`. This matches Compact's
/// treatment of assignment as an expression in these contexts
/// (e.g. `(field = x)`, `return counter += y, x * y`).
pub(crate) fn expr_seq(p: &mut Parser) {
    let m = p.start();
    expr_or_assign(p);
    if p.at(COMMA) && !p.at_end() {
        while p.eat(COMMA) {
            if p.at(R_PAREN) || p.at(SEMICOLON) || p.at(R_BRACE) || p.at(R_BRACKET) || p.at_end() {
                break;
            }
            expr_or_assign(p);
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
            expr_bp_or_error(p, 0); // then branch (any precedence)
            p.expect(COLON);
            expr_bp_or_error(p, r_bp); // else branch (right-assoc)
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
                comma_sep(p, R_PAREN, call_arg);
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
            comma_sep(p, R_PAREN, call_arg);
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
            // Index accepts any expression (literals, idents, arithmetic, etc.)
            expr(p);
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
            expr_bp_or_error(p, r_bp);
            lhs = m.complete(p, BINARY_EXPR);
            continue;
        }

        // No matching operator — stop.
        break;
    }

    Some(lhs)
}

/// Like `expr_bp` but emits an ERROR placeholder at the current position
/// if no LHS could be produced.
///
/// Use this in grammatical contexts that *require* an expression on the
/// right-hand side (binary operators, ternary branches, unary prefix). The
/// bare `expr_bp` returns `Option<CompletedMarker>`; if callers ignored that
/// `None`, the outer marker would be completed with a zero-width hole, and
/// the Pratt loop could spin without consuming a token (latent infinite
/// loop / empty-marker bug, FU1).
///
/// The ERROR placeholder preserves the lossless round-trip (parent marker
/// has a real child where the missing RHS would have been) and lets outer
/// recovery resume on the next token.
fn expr_bp_or_error(p: &mut Parser, min_bp: u8) {
    if expr_bp(p, min_bp).is_none() {
        // `lhs()` already emitted a diagnostic when it returned None
        // (see the wildcard arm in `lhs`), so we only need the placeholder
        // node here — no duplicate `p.error(...)` call.
        let m = p.start();
        m.complete(p, ERROR);
    }
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
            expr_bp_or_error(p, 18);
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

        FOLD_KW => {
            let m = p.start();
            p.bump(FOLD_KW);
            // `fold(fn, init, ...args)` — strictly require the `(`.
            // Legacy `fold f init over xs` is not accepted by any current
            // Compact compiler; corpus fixtures using the legacy form live
            // under `tests/corpus/errors/negative/`.
            p.expect(L_PAREN);
            // First: function
            expr(p);
            p.expect(COMMA);
            // Second: init value
            expr(p);
            p.expect(COMMA);
            // Rest: expressions
            comma_sep1(p, R_PAREN, expr);
            p.expect(R_PAREN);
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

        // `ledger` — bare-name reference to the ledger context.
        // Compact lets circuit bodies write `ledger.field.method(...)`
        // using `ledger` as an implicit name expression. The keyword
        // itself produces a `NAME_EXPR`; subsequent `.field` access
        // is parsed by the normal postfix-operator loop.
        LEDGER_KW => {
            let m = p.start();
            p.bump(LEDGER_KW);
            Some(m.complete(p, NAME_EXPR))
        }

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
                comma_sep(p, R_PAREN, call_arg);
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
        comma_sep(p, R_PAREN, call_arg);
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

    // Parenthesized expression. Inside a `(...)` we allow assignment
    // and compound-assignment as expressions (handled inside
    // `expr_seq`'s element parser), since Compact treats `(x = e)`
    // and `(x += e)` as valid expression forms when explicitly
    // parenthesized. The statement forms are still parsed by
    // `expr_or_assign_stmt`.
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

    // Body: block or expression. The expression form may be an
    // assignment expression — e.g. `(y) => field = y` is a valid
    // Compact lambda whose body assigns its argument to a ledger
    // field.
    if p.at(L_BRACE) {
        super::statements::block(p);
    } else {
        expr_or_assign(p);
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
    fn expr_call_named_arg() {
        check(
            "circuit f() : Field { obj.insert(1, field = 42); return 1 as Field; }",
            expect![[r#"
                SOURCE_FILE@0..69
                  CIRCUIT_DEF@0..69
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
                    BLOCK@19..69
                      WHITESPACE@19..20 " "
                      L_BRACE@20..21 "{"
                      EXPR_STMT@21..48
                        CALL_EXPR@21..47
                          NAME_EXPR@21..25
                            WHITESPACE@21..22 " "
                            IDENT@22..25 "obj"
                          DOT@25..26 "."
                          IDENT@26..32 "insert"
                          L_PAREN@32..33 "("
                          LITERAL_EXPR@33..34
                            INT_LIT@33..34 "1"
                          COMMA@34..35 ","
                          NAMED_ARG@35..46
                            WHITESPACE@35..36 " "
                            IDENT@36..41 "field"
                            WHITESPACE@41..42 " "
                            EQ@42..43 "="
                            LITERAL_EXPR@43..46
                              WHITESPACE@43..44 " "
                              INT_LIT@44..46 "42"
                          R_PAREN@46..47 ")"
                        SEMICOLON@47..48 ";"
                      RETURN_STMT@48..67
                        WHITESPACE@48..49 " "
                        RETURN_KW@49..55 "return"
                        CAST_EXPR@55..66
                          LITERAL_EXPR@55..57
                            WHITESPACE@55..56 " "
                            INT_LIT@56..57 "1"
                          WHITESPACE@57..58 " "
                          AS_KW@58..60 "as"
                          FIELD_TYPE@60..66
                            WHITESPACE@60..61 " "
                            FIELD_KW@61..66 "Field"
                        SEMICOLON@66..67 ";"
                      WHITESPACE@67..68 " "
                      R_BRACE@68..69 "}"
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
    fn expr_vector_index_with_expr() {
        check(
            "circuit f() : Field { return v[i - 1]; }",
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
                      RETURN_STMT@21..38
                        WHITESPACE@21..22 " "
                        RETURN_KW@22..28 "return"
                        INDEX_EXPR@28..37
                          NAME_EXPR@28..30
                            WHITESPACE@28..29 " "
                            IDENT@29..30 "v"
                          L_BRACKET@30..31 "["
                          BINARY_EXPR@31..36
                            NAME_EXPR@31..32
                              IDENT@31..32 "i"
                            WHITESPACE@32..33 " "
                            MINUS@33..34 "-"
                            LITERAL_EXPR@34..36
                              WHITESPACE@34..35 " "
                              INT_LIT@35..36 "1"
                          R_BRACKET@36..37 "]"
                        SEMICOLON@37..38 ";"
                      WHITESPACE@38..39 " "
                      R_BRACE@39..40 "}"
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

    // --- FU1 regression tests --------------------------------------------
    //
    // These exercise the `expr_bp_or_error` helper added in FU1.
    // Previously, when an inner `expr_bp` call returned `None` (no LHS),
    // the surrounding Pratt context would complete its marker with a
    // zero-width hole — a latent infinite-loop / empty-marker bug that
    // would have been the first thing the WS3 fuzzer hit. The fix inserts
    // an ERROR placeholder so the CST round-trips and the parser advances.

    #[test]
    fn expr_binary_missing_rhs_emits_error_no_infinite_loop() {
        check(
            "circuit f() : Field { return a + ; }",
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
                        BINARY_EXPR@28..32
                          NAME_EXPR@28..30
                            WHITESPACE@28..29 " "
                            IDENT@29..30 "a"
                          WHITESPACE@30..31 " "
                          PLUS@31..32 "+"
                          ERROR@32..32
                        WHITESPACE@32..33 " "
                        SEMICOLON@33..34 ";"
                      WHITESPACE@34..35 " "
                      R_BRACE@35..36 "}"
                errors:
                  expected expression
            "#]],
        );
    }

    #[test]
    fn expr_ternary_missing_then_emits_error() {
        check(
            "circuit f() : Field { return a ? : b; }",
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
                        TERNARY_EXPR@28..36
                          NAME_EXPR@28..30
                            WHITESPACE@28..29 " "
                            IDENT@29..30 "a"
                          WHITESPACE@30..31 " "
                          QUESTION@31..32 "?"
                          ERROR@32..32
                          WHITESPACE@32..33 " "
                          COLON@33..34 ":"
                          NAME_EXPR@34..36
                            WHITESPACE@34..35 " "
                            IDENT@35..36 "b"
                        SEMICOLON@36..37 ";"
                      WHITESPACE@37..38 " "
                      R_BRACE@38..39 "}"
                errors:
                  expected expression
            "#]],
        );
    }

    #[test]
    fn expr_assign_in_paren() {
        check("circuit f() : [] { return (x = 42); }", expect![[r#"
            SOURCE_FILE@0..37
              CIRCUIT_DEF@0..37
                CIRCUIT_KW@0..7 "circuit"
                WHITESPACE@7..8 " "
                IDENT@8..9 "f"
                L_PAREN@9..10 "("
                R_PAREN@10..11 ")"
                WHITESPACE@11..12 " "
                COLON@12..13 ":"
                TUPLE_TYPE@13..16
                  WHITESPACE@13..14 " "
                  L_BRACKET@14..15 "["
                  R_BRACKET@15..16 "]"
                BLOCK@16..37
                  WHITESPACE@16..17 " "
                  L_BRACE@17..18 "{"
                  RETURN_STMT@18..35
                    WHITESPACE@18..19 " "
                    RETURN_KW@19..25 "return"
                    PAREN_EXPR@25..34
                      WHITESPACE@25..26 " "
                      L_PAREN@26..27 "("
                      ASSIGN_EXPR@27..33
                        NAME_EXPR@27..28
                          IDENT@27..28 "x"
                        WHITESPACE@28..29 " "
                        EQ@29..30 "="
                        LITERAL_EXPR@30..33
                          WHITESPACE@30..31 " "
                          INT_LIT@31..33 "42"
                      R_PAREN@33..34 ")"
                    SEMICOLON@34..35 ";"
                  WHITESPACE@35..36 " "
                  R_BRACE@36..37 "}"
        "#]]);
    }

    #[test]
    fn expr_compound_assign_in_paren() {
        check(
            "circuit f() : [] { assert((x += 2) == 0, \"ok\"); }",
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
                    TUPLE_TYPE@13..16
                      WHITESPACE@13..14 " "
                      L_BRACKET@14..15 "["
                      R_BRACKET@15..16 "]"
                    BLOCK@16..49
                      WHITESPACE@16..17 " "
                      L_BRACE@17..18 "{"
                      EXPR_STMT@18..47
                        CALL_EXPR@18..46
                          WHITESPACE@18..19 " "
                          ASSERT_KW@19..25 "assert"
                          L_PAREN@25..26 "("
                          BINARY_EXPR@26..39
                            PAREN_EXPR@26..34
                              L_PAREN@26..27 "("
                              COMPOUND_ASSIGN_EXPR@27..33
                                NAME_EXPR@27..28
                                  IDENT@27..28 "x"
                                WHITESPACE@28..29 " "
                                PLUS_EQ@29..31 "+="
                                LITERAL_EXPR@31..33
                                  WHITESPACE@31..32 " "
                                  INT_LIT@32..33 "2"
                              R_PAREN@33..34 ")"
                            WHITESPACE@34..35 " "
                            EQ_EQ@35..37 "=="
                            LITERAL_EXPR@37..39
                              WHITESPACE@37..38 " "
                              INT_LIT@38..39 "0"
                          COMMA@39..40 ","
                          LITERAL_EXPR@40..45
                            WHITESPACE@40..41 " "
                            STRING_LIT@41..45 "\"ok\""
                          R_PAREN@45..46 ")"
                        SEMICOLON@46..47 ";"
                      WHITESPACE@47..48 " "
                      R_BRACE@48..49 "}"
            "#]],
        );
    }

    #[test]
    fn expr_assign_in_lambda_body() {
        check(
            "circuit f() : [] { const g = (y) => field = y; }",
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
                    TUPLE_TYPE@13..16
                      WHITESPACE@13..14 " "
                      L_BRACKET@14..15 "["
                      R_BRACKET@15..16 "]"
                    BLOCK@16..48
                      WHITESPACE@16..17 " "
                      L_BRACE@17..18 "{"
                      CONST_STMT@18..46
                        WHITESPACE@18..19 " "
                        CONST_KW@19..24 "const"
                        IDENT_PAT@24..26
                          WHITESPACE@24..25 " "
                          IDENT@25..26 "g"
                        WHITESPACE@26..27 " "
                        EQ@27..28 "="
                        LAMBDA_EXPR@28..45
                          PARAM_LIST@28..32
                            WHITESPACE@28..29 " "
                            L_PAREN@29..30 "("
                            IDENT_PAT@30..31
                              IDENT@30..31 "y"
                            R_PAREN@31..32 ")"
                          WHITESPACE@32..33 " "
                          FAT_ARROW@33..35 "=>"
                          ASSIGN_EXPR@35..45
                            NAME_EXPR@35..41
                              WHITESPACE@35..36 " "
                              IDENT@36..41 "field"
                            WHITESPACE@41..42 " "
                            EQ@42..43 "="
                            NAME_EXPR@43..45
                              WHITESPACE@43..44 " "
                              IDENT@44..45 "y"
                        SEMICOLON@45..46 ";"
                      WHITESPACE@46..47 " "
                      R_BRACE@47..48 "}"
            "#]],
        );
    }

    #[test]
    fn expr_assign_in_return() {
        check("circuit f() : [] { return field = x; }", expect![[r#"
            SOURCE_FILE@0..38
              CIRCUIT_DEF@0..38
                CIRCUIT_KW@0..7 "circuit"
                WHITESPACE@7..8 " "
                IDENT@8..9 "f"
                L_PAREN@9..10 "("
                R_PAREN@10..11 ")"
                WHITESPACE@11..12 " "
                COLON@12..13 ":"
                TUPLE_TYPE@13..16
                  WHITESPACE@13..14 " "
                  L_BRACKET@14..15 "["
                  R_BRACKET@15..16 "]"
                BLOCK@16..38
                  WHITESPACE@16..17 " "
                  L_BRACE@17..18 "{"
                  RETURN_STMT@18..36
                    WHITESPACE@18..19 " "
                    RETURN_KW@19..25 "return"
                    ASSIGN_EXPR@25..35
                      NAME_EXPR@25..31
                        WHITESPACE@25..26 " "
                        IDENT@26..31 "field"
                      WHITESPACE@31..32 " "
                      EQ@32..33 "="
                      NAME_EXPR@33..35
                        WHITESPACE@33..34 " "
                        IDENT@34..35 "x"
                    SEMICOLON@35..36 ";"
                  WHITESPACE@36..37 " "
                  R_BRACE@37..38 "}"
        "#]]);
    }

    #[test]
    fn expr_compound_assign_in_const_rhs() {
        check(
            "circuit f() : [] { const p = counter += disclose(x); }",
            expect![[r#"
                SOURCE_FILE@0..54
                  CIRCUIT_DEF@0..54
                    CIRCUIT_KW@0..7 "circuit"
                    WHITESPACE@7..8 " "
                    IDENT@8..9 "f"
                    L_PAREN@9..10 "("
                    R_PAREN@10..11 ")"
                    WHITESPACE@11..12 " "
                    COLON@12..13 ":"
                    TUPLE_TYPE@13..16
                      WHITESPACE@13..14 " "
                      L_BRACKET@14..15 "["
                      R_BRACKET@15..16 "]"
                    BLOCK@16..54
                      WHITESPACE@16..17 " "
                      L_BRACE@17..18 "{"
                      CONST_STMT@18..52
                        WHITESPACE@18..19 " "
                        CONST_KW@19..24 "const"
                        IDENT_PAT@24..26
                          WHITESPACE@24..25 " "
                          IDENT@25..26 "p"
                        WHITESPACE@26..27 " "
                        EQ@27..28 "="
                        COMPOUND_ASSIGN_EXPR@28..51
                          NAME_EXPR@28..36
                            WHITESPACE@28..29 " "
                            IDENT@29..36 "counter"
                          WHITESPACE@36..37 " "
                          PLUS_EQ@37..39 "+="
                          DISCLOSE_EXPR@39..51
                            WHITESPACE@39..40 " "
                            DISCLOSE_KW@40..48 "disclose"
                            L_PAREN@48..49 "("
                            NAME_EXPR@49..50
                              IDENT@49..50 "x"
                            R_PAREN@50..51 ")"
                        SEMICOLON@51..52 ";"
                      WHITESPACE@52..53 " "
                      R_BRACE@53..54 "}"
            "#]],
        );
    }

    #[test]
    fn expr_compound_assign_in_for_iter() {
        check(
            "circuit f() : [] { for (const y of counter -= u, counter += u) { x; } }",
            expect![[r#"
                SOURCE_FILE@0..71
                  CIRCUIT_DEF@0..71
                    CIRCUIT_KW@0..7 "circuit"
                    WHITESPACE@7..8 " "
                    IDENT@8..9 "f"
                    L_PAREN@9..10 "("
                    R_PAREN@10..11 ")"
                    WHITESPACE@11..12 " "
                    COLON@12..13 ":"
                    TUPLE_TYPE@13..16
                      WHITESPACE@13..14 " "
                      L_BRACKET@14..15 "["
                      R_BRACKET@15..16 "]"
                    BLOCK@16..71
                      WHITESPACE@16..17 " "
                      L_BRACE@17..18 "{"
                      FOR_STMT@18..69
                        WHITESPACE@18..19 " "
                        FOR_KW@19..22 "for"
                        WHITESPACE@22..23 " "
                        L_PAREN@23..24 "("
                        CONST_KW@24..29 "const"
                        WHITESPACE@29..30 " "
                        IDENT@30..31 "y"
                        WHITESPACE@31..32 " "
                        OF_KW@32..34 "of"
                        EXPR_SEQ@34..61
                          COMPOUND_ASSIGN_EXPR@34..47
                            NAME_EXPR@34..42
                              WHITESPACE@34..35 " "
                              IDENT@35..42 "counter"
                            WHITESPACE@42..43 " "
                            MINUS_EQ@43..45 "-="
                            NAME_EXPR@45..47
                              WHITESPACE@45..46 " "
                              IDENT@46..47 "u"
                          COMMA@47..48 ","
                          COMPOUND_ASSIGN_EXPR@48..61
                            NAME_EXPR@48..56
                              WHITESPACE@48..49 " "
                              IDENT@49..56 "counter"
                            WHITESPACE@56..57 " "
                            PLUS_EQ@57..59 "+="
                            NAME_EXPR@59..61
                              WHITESPACE@59..60 " "
                              IDENT@60..61 "u"
                        R_PAREN@61..62 ")"
                        BLOCK@62..69
                          WHITESPACE@62..63 " "
                          L_BRACE@63..64 "{"
                          EXPR_STMT@64..67
                            NAME_EXPR@64..66
                              WHITESPACE@64..65 " "
                              IDENT@65..66 "x"
                            SEMICOLON@66..67 ";"
                          WHITESPACE@67..68 " "
                          R_BRACE@68..69 "}"
                      WHITESPACE@69..70 " "
                      R_BRACE@70..71 "}"
            "#]],
        );
    }

    #[test]
    fn expr_ledger_keyword_as_name() {
        check(
            "circuit f() : [] { ledger.field.write(true); }",
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
                    TUPLE_TYPE@13..16
                      WHITESPACE@13..14 " "
                      L_BRACKET@14..15 "["
                      R_BRACKET@15..16 "]"
                    BLOCK@16..46
                      WHITESPACE@16..17 " "
                      L_BRACE@17..18 "{"
                      EXPR_STMT@18..44
                        CALL_EXPR@18..43
                          MEMBER_EXPR@18..31
                            NAME_EXPR@18..25
                              WHITESPACE@18..19 " "
                              LEDGER_KW@19..25 "ledger"
                            DOT@25..26 "."
                            IDENT@26..31 "field"
                          DOT@31..32 "."
                          IDENT@32..37 "write"
                          L_PAREN@37..38 "("
                          LITERAL_EXPR@38..42
                            TRUE_KW@38..42 "true"
                          R_PAREN@42..43 ")"
                        SEMICOLON@43..44 ";"
                      WHITESPACE@44..45 " "
                      R_BRACE@45..46 "}"
            "#]],
        );
    }
}
