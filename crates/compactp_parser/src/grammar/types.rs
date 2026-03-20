//! Type parsing.
//!
//! Grammar:
//! ```text
//! type → Boolean | Field | Uint<size> | Uint<size..size> | Bytes<size>
//!      | Opaque<str> | Vector<size, type> | [type, ..., type] | tref
//! tref → id gargs?
//! gargs → < garg, ..., garg >
//! garg → nat | type
//! gparams → < generic-param, ..., generic-param >
//! generic-param → # tvar | tvar
//! tsize → nat | id
//! ```

use crate::grammar::{comma_sep, comma_sep1};
use crate::parser::Parser;
use compactp_syntax::SyntaxKind::*;

/// Parse a type.
pub(crate) fn ty(p: &mut Parser) {
    match p.current() {
        BOOLEAN_KW => {
            let m = p.start();
            p.bump(BOOLEAN_KW);
            m.complete(p, BOOLEAN_TYPE);
        }
        FIELD_KW => {
            let m = p.start();
            p.bump(FIELD_KW);
            m.complete(p, FIELD_TYPE);
        }
        UINT_KW => uint_type(p),
        BYTES_KW => bytes_type(p),
        OPAQUE_KW => opaque_type(p),
        VECTOR_KW => vector_type(p),
        L_BRACKET => tuple_type(p),
        IDENT => type_ref(p),
        _ => {
            p.error("expected type");
        }
    }
}

/// `Uint < tsize (.. tsize)? >`
fn uint_type(p: &mut Parser) {
    let m = p.start();
    p.bump(UINT_KW);
    p.expect(LT);
    type_size(p);
    if p.eat(DOT_DOT) {
        type_size(p);
    }
    p.expect(GT);
    m.complete(p, UINT_TYPE);
}

/// `Bytes < tsize >`
fn bytes_type(p: &mut Parser) {
    let m = p.start();
    p.bump(BYTES_KW);
    p.expect(LT);
    type_size(p);
    p.expect(GT);
    m.complete(p, BYTES_TYPE);
}

/// `Opaque < str >`
fn opaque_type(p: &mut Parser) {
    let m = p.start();
    p.bump(OPAQUE_KW);
    p.expect(LT);
    p.expect(STRING_LIT);
    p.expect(GT);
    m.complete(p, OPAQUE_TYPE);
}

/// `Vector < tsize , type >`
fn vector_type(p: &mut Parser) {
    let m = p.start();
    p.bump(VECTOR_KW);
    p.expect(LT);
    type_size(p);
    p.expect(COMMA);
    ty(p);
    p.expect(GT);
    m.complete(p, VECTOR_TYPE);
}

/// `[ type , ... , type ]`
fn tuple_type(p: &mut Parser) {
    let m = p.start();
    p.bump(L_BRACKET);
    comma_sep(p, R_BRACKET, ty);
    p.expect(R_BRACKET);
    m.complete(p, TUPLE_TYPE);
}

/// `id gargs?` — a type reference (named type with optional generic args).
fn type_ref(p: &mut Parser) {
    let m = p.start();
    p.expect(IDENT);
    if p.at(LT) {
        generic_arg_list(p);
    }
    m.complete(p, TYPE_REF);
}

/// `tsize → nat | id`
fn type_size(p: &mut Parser) {
    let m = p.start();
    match p.current() {
        INT_LIT | HEX_LIT | OCT_LIT | BIN_LIT => {
            p.bump_any();
            m.complete(p, TYPE_SIZE);
        }
        IDENT => {
            p.bump(IDENT);
            m.complete(p, TYPE_SIZE);
        }
        _ => {
            p.error("expected type size (number or identifier)");
            m.abandon(p);
        }
    }
}

/// `< garg , ... , garg >`
pub(crate) fn generic_arg_list(p: &mut Parser) {
    let m = p.start();
    p.expect(LT);
    comma_sep1(p, GT, generic_arg);
    p.expect(GT);
    m.complete(p, GENERIC_ARG_LIST);
}

/// `garg → nat | type`
fn generic_arg(p: &mut Parser) {
    let m = p.start();
    match p.current() {
        // Numeric literals are always treated as generic arg values (not types).
        INT_LIT | HEX_LIT | OCT_LIT | BIN_LIT => {
            p.bump_any();
            m.complete(p, GENERIC_ARG);
        }
        _ => {
            // Otherwise try to parse as a type.
            ty(p);
            m.complete(p, GENERIC_ARG);
        }
    }
}

/// `< generic-param , ... , generic-param >`
pub(crate) fn generic_param_list(p: &mut Parser) {
    let m = p.start();
    p.expect(LT);
    comma_sep1(p, GT, generic_param);
    p.expect(GT);
    m.complete(p, GENERIC_PARAM_LIST);
}

/// `generic-param → # tvar | tvar`
fn generic_param(p: &mut Parser) {
    let m = p.start();
    // Optional `#` prefix for numeric type variables
    p.eat(HASH);
    p.expect(IDENT);
    m.complete(p, GENERIC_PARAM);
}

#[cfg(test)]
mod tests {
    use crate::grammar::tests::check;
    use expect_test::expect;

    // We test types through circuit declarations that use them.

    #[test]
    fn type_boolean() {
        check(
            "circuit f() : Boolean { }",
            expect![[r#"
                SOURCE_FILE@0..25
                  CIRCUIT_DEF@0..25
                    CIRCUIT_KW@0..7 "circuit"
                    WHITESPACE@7..8 " "
                    IDENT@8..9 "f"
                    L_PAREN@9..10 "("
                    R_PAREN@10..11 ")"
                    WHITESPACE@11..12 " "
                    COLON@12..13 ":"
                    BOOLEAN_TYPE@13..21
                      WHITESPACE@13..14 " "
                      BOOLEAN_KW@14..21 "Boolean"
                    BLOCK@21..25
                      WHITESPACE@21..22 " "
                      L_BRACE@22..23 "{"
                      WHITESPACE@23..24 " "
                      R_BRACE@24..25 "}"
            "#]],
        );
    }

    #[test]
    fn type_field() {
        check(
            "circuit f() : Field { }",
            expect![[r#"
                SOURCE_FILE@0..23
                  CIRCUIT_DEF@0..23
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
                    BLOCK@19..23
                      WHITESPACE@19..20 " "
                      L_BRACE@20..21 "{"
                      WHITESPACE@21..22 " "
                      R_BRACE@22..23 "}"
            "#]],
        );
    }

    #[test]
    fn type_uint() {
        check(
            "circuit f() : Uint<8> { }",
            expect![[r#"
                SOURCE_FILE@0..25
                  CIRCUIT_DEF@0..25
                    CIRCUIT_KW@0..7 "circuit"
                    WHITESPACE@7..8 " "
                    IDENT@8..9 "f"
                    L_PAREN@9..10 "("
                    R_PAREN@10..11 ")"
                    WHITESPACE@11..12 " "
                    COLON@12..13 ":"
                    UINT_TYPE@13..21
                      WHITESPACE@13..14 " "
                      UINT_KW@14..18 "Uint"
                      LT@18..19 "<"
                      TYPE_SIZE@19..20
                        INT_LIT@19..20 "8"
                      GT@20..21 ">"
                    BLOCK@21..25
                      WHITESPACE@21..22 " "
                      L_BRACE@22..23 "{"
                      WHITESPACE@23..24 " "
                      R_BRACE@24..25 "}"
            "#]],
        );
    }

    #[test]
    fn type_uint_range() {
        check(
            "circuit f() : Uint<1..8> { }",
            expect![[r#"
                SOURCE_FILE@0..28
                  CIRCUIT_DEF@0..28
                    CIRCUIT_KW@0..7 "circuit"
                    WHITESPACE@7..8 " "
                    IDENT@8..9 "f"
                    L_PAREN@9..10 "("
                    R_PAREN@10..11 ")"
                    WHITESPACE@11..12 " "
                    COLON@12..13 ":"
                    UINT_TYPE@13..24
                      WHITESPACE@13..14 " "
                      UINT_KW@14..18 "Uint"
                      LT@18..19 "<"
                      TYPE_SIZE@19..20
                        INT_LIT@19..20 "1"
                      DOT_DOT@20..22 ".."
                      TYPE_SIZE@22..23
                        INT_LIT@22..23 "8"
                      GT@23..24 ">"
                    BLOCK@24..28
                      WHITESPACE@24..25 " "
                      L_BRACE@25..26 "{"
                      WHITESPACE@26..27 " "
                      R_BRACE@27..28 "}"
            "#]],
        );
    }

    #[test]
    fn type_bytes() {
        check(
            "circuit f() : Bytes<32> { }",
            expect![[r#"
                SOURCE_FILE@0..27
                  CIRCUIT_DEF@0..27
                    CIRCUIT_KW@0..7 "circuit"
                    WHITESPACE@7..8 " "
                    IDENT@8..9 "f"
                    L_PAREN@9..10 "("
                    R_PAREN@10..11 ")"
                    WHITESPACE@11..12 " "
                    COLON@12..13 ":"
                    BYTES_TYPE@13..23
                      WHITESPACE@13..14 " "
                      BYTES_KW@14..19 "Bytes"
                      LT@19..20 "<"
                      TYPE_SIZE@20..22
                        INT_LIT@20..22 "32"
                      GT@22..23 ">"
                    BLOCK@23..27
                      WHITESPACE@23..24 " "
                      L_BRACE@24..25 "{"
                      WHITESPACE@25..26 " "
                      R_BRACE@26..27 "}"
            "#]],
        );
    }

    #[test]
    fn type_opaque() {
        check(
            r#"circuit f() : Opaque<"foo"> { }"#,
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
                    OPAQUE_TYPE@13..27
                      WHITESPACE@13..14 " "
                      OPAQUE_KW@14..20 "Opaque"
                      LT@20..21 "<"
                      STRING_LIT@21..26 "\"foo\""
                      GT@26..27 ">"
                    BLOCK@27..31
                      WHITESPACE@27..28 " "
                      L_BRACE@28..29 "{"
                      WHITESPACE@29..30 " "
                      R_BRACE@30..31 "}"
            "#]],
        );
    }

    #[test]
    fn type_vector() {
        check(
            "circuit f() : Vector<10, Field> { }",
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
                    VECTOR_TYPE@13..31
                      WHITESPACE@13..14 " "
                      VECTOR_KW@14..20 "Vector"
                      LT@20..21 "<"
                      TYPE_SIZE@21..23
                        INT_LIT@21..23 "10"
                      COMMA@23..24 ","
                      FIELD_TYPE@24..30
                        WHITESPACE@24..25 " "
                        FIELD_KW@25..30 "Field"
                      GT@30..31 ">"
                    BLOCK@31..35
                      WHITESPACE@31..32 " "
                      L_BRACE@32..33 "{"
                      WHITESPACE@33..34 " "
                      R_BRACE@34..35 "}"
            "#]],
        );
    }

    #[test]
    fn type_tuple() {
        check(
            "circuit f() : [Field, Boolean] { }",
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
                    TUPLE_TYPE@13..30
                      WHITESPACE@13..14 " "
                      L_BRACKET@14..15 "["
                      FIELD_TYPE@15..20
                        FIELD_KW@15..20 "Field"
                      COMMA@20..21 ","
                      BOOLEAN_TYPE@21..29
                        WHITESPACE@21..22 " "
                        BOOLEAN_KW@22..29 "Boolean"
                      R_BRACKET@29..30 "]"
                    BLOCK@30..34
                      WHITESPACE@30..31 " "
                      L_BRACE@31..32 "{"
                      WHITESPACE@32..33 " "
                      R_BRACE@33..34 "}"
            "#]],
        );
    }

    #[test]
    fn type_ref_with_generics() {
        check(
            "circuit f() : MyType<Field, 10> { }",
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
                    TYPE_REF@13..31
                      WHITESPACE@13..14 " "
                      IDENT@14..20 "MyType"
                      GENERIC_ARG_LIST@20..31
                        LT@20..21 "<"
                        GENERIC_ARG@21..26
                          FIELD_TYPE@21..26
                            FIELD_KW@21..26 "Field"
                        COMMA@26..27 ","
                        GENERIC_ARG@27..30
                          WHITESPACE@27..28 " "
                          INT_LIT@28..30 "10"
                        GT@30..31 ">"
                    BLOCK@31..35
                      WHITESPACE@31..32 " "
                      L_BRACE@32..33 "{"
                      WHITESPACE@33..34 " "
                      R_BRACE@34..35 "}"
            "#]],
        );
    }

    #[test]
    fn type_ref_simple() {
        check(
            "circuit f() : MyType { }",
            expect![[r#"
                SOURCE_FILE@0..24
                  CIRCUIT_DEF@0..24
                    CIRCUIT_KW@0..7 "circuit"
                    WHITESPACE@7..8 " "
                    IDENT@8..9 "f"
                    L_PAREN@9..10 "("
                    R_PAREN@10..11 ")"
                    WHITESPACE@11..12 " "
                    COLON@12..13 ":"
                    TYPE_REF@13..20
                      WHITESPACE@13..14 " "
                      IDENT@14..20 "MyType"
                    BLOCK@20..24
                      WHITESPACE@20..21 " "
                      L_BRACE@21..22 "{"
                      WHITESPACE@22..23 " "
                      R_BRACE@23..24 "}"
            "#]],
        );
    }
}
