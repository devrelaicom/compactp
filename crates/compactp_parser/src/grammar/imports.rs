//! Import, export, and module parsing.
//!
//! Grammar:
//! ```text
//! import → import import-name gargs? prefix? ;
//! import-name → id | str
//! prefix → prefix id
//! export-list → export { id, ..., id } ;?
//! module → export? module name gparams? { pelts... }
//! ```

use crate::grammar::{comma_sep1, declarations};
use crate::parser::Parser;
use compactp_syntax::SyntaxKind::*;

/// `import import-name gargs? prefix? ;`
/// `import { specifier, ... } from import-name gargs? prefix? ;`
pub(crate) fn import(p: &mut Parser) {
    let m = p.start();
    p.bump(IMPORT_KW);

    if p.at(L_BRACE) {
        // Selective import: `import { id (as id)?, ... } from name gargs? prefix? ;`
        let list_m = p.start();
        p.bump(L_BRACE);
        super::comma_sep(p, R_BRACE, |p| {
            let spec_m = p.start();
            p.expect(IDENT);
            if p.eat(AS_KW) {
                p.expect(IDENT);
            }
            spec_m.complete(p, IMPORT_SPECIFIER);
        });
        p.expect(R_BRACE);
        list_m.complete(p, IMPORT_SPECIFIER_LIST);

        p.expect(FROM_KW);
    }

    // import-name: id or string
    match p.current() {
        IDENT => p.bump(IDENT),
        STRING_LIT => p.bump(STRING_LIT),
        _ => p.error("expected module name or string path"),
    }

    // Optional generic args
    if p.at(LT) {
        super::types::generic_arg_list(p);
    }

    // Optional prefix
    if p.at(PREFIX_KW) {
        let pm = p.start();
        p.bump(PREFIX_KW);
        p.expect(IDENT);
        pm.complete(p, PREFIX_DECL);
    }

    p.expect(SEMICOLON);
    m.complete(p, IMPORT);
}

/// `export { id, ..., id } ;?`
///
/// Called when we've already determined this is an export list (not an export modifier).
pub(crate) fn export_list(p: &mut Parser) {
    let m = p.start();
    p.bump(EXPORT_KW);
    p.expect(L_BRACE);
    comma_sep1(p, R_BRACE, |p| {
        p.expect(IDENT);
    });
    p.expect(R_BRACE);
    p.eat(SEMICOLON);
    m.complete(p, EXPORT_LIST);
}

/// `export? module name gparams? { pelts... }`
pub(crate) fn module_def(p: &mut Parser, has_export: bool) {
    let m = p.start();
    if has_export {
        p.bump(EXPORT_KW);
    }
    p.bump(MODULE_KW);
    p.expect(IDENT);

    // Optional generic params
    if p.at(LT) {
        super::types::generic_param_list(p);
    }

    p.expect(L_BRACE);
    while !p.at(R_BRACE) && !p.at_end() {
        if p.errors_exhausted() {
            break;
        }
        declarations::declaration(p);
    }
    p.expect(R_BRACE);
    m.complete(p, MODULE_DEF);
}

#[cfg(test)]
mod tests {
    use crate::grammar::tests::check;
    use expect_test::expect;

    #[test]
    fn import_ident() {
        check(
            "import Foo;",
            expect![[r#"
                SOURCE_FILE@0..11
                  IMPORT@0..11
                    IMPORT_KW@0..6 "import"
                    WHITESPACE@6..7 " "
                    IDENT@7..10 "Foo"
                    SEMICOLON@10..11 ";"
            "#]],
        );
    }

    #[test]
    fn import_string() {
        check(
            r#"import "path/to/module";"#,
            expect![[r#"
                SOURCE_FILE@0..24
                  IMPORT@0..24
                    IMPORT_KW@0..6 "import"
                    WHITESPACE@6..7 " "
                    STRING_LIT@7..23 "\"path/to/module\""
                    SEMICOLON@23..24 ";"
            "#]],
        );
    }

    #[test]
    fn import_with_generics() {
        check(
            "import Foo<10, Field>;",
            expect![[r#"
                SOURCE_FILE@0..22
                  IMPORT@0..22
                    IMPORT_KW@0..6 "import"
                    WHITESPACE@6..7 " "
                    IDENT@7..10 "Foo"
                    GENERIC_ARG_LIST@10..21
                      LT@10..11 "<"
                      GENERIC_ARG@11..13
                        INT_LIT@11..13 "10"
                      COMMA@13..14 ","
                      GENERIC_ARG@14..20
                        FIELD_TYPE@14..20
                          WHITESPACE@14..15 " "
                          FIELD_KW@15..20 "Field"
                      GT@20..21 ">"
                    SEMICOLON@21..22 ";"
            "#]],
        );
    }

    #[test]
    fn import_with_prefix() {
        check(
            "import Foo prefix bar;",
            expect![[r#"
                SOURCE_FILE@0..22
                  IMPORT@0..22
                    IMPORT_KW@0..6 "import"
                    WHITESPACE@6..7 " "
                    IDENT@7..10 "Foo"
                    PREFIX_DECL@10..21
                      WHITESPACE@10..11 " "
                      PREFIX_KW@11..17 "prefix"
                      WHITESPACE@17..18 " "
                      IDENT@18..21 "bar"
                    SEMICOLON@21..22 ";"
            "#]],
        );
    }

    #[test]
    fn export_list_simple() {
        check(
            "export { foo, bar };",
            expect![[r#"
                SOURCE_FILE@0..20
                  EXPORT_LIST@0..20
                    EXPORT_KW@0..6 "export"
                    WHITESPACE@6..7 " "
                    L_BRACE@7..8 "{"
                    WHITESPACE@8..9 " "
                    IDENT@9..12 "foo"
                    COMMA@12..13 ","
                    WHITESPACE@13..14 " "
                    IDENT@14..17 "bar"
                    WHITESPACE@17..18 " "
                    R_BRACE@18..19 "}"
                    SEMICOLON@19..20 ";"
            "#]],
        );
    }

    #[test]
    fn export_list_no_semicolon() {
        check(
            "export { foo, bar }",
            expect![[r#"
                SOURCE_FILE@0..19
                  EXPORT_LIST@0..19
                    EXPORT_KW@0..6 "export"
                    WHITESPACE@6..7 " "
                    L_BRACE@7..8 "{"
                    WHITESPACE@8..9 " "
                    IDENT@9..12 "foo"
                    COMMA@12..13 ","
                    WHITESPACE@13..14 " "
                    IDENT@14..17 "bar"
                    WHITESPACE@17..18 " "
                    R_BRACE@18..19 "}"
            "#]],
        );
    }

    #[test]
    fn module_simple() {
        check(
            "module Foo { }",
            expect![[r#"
                SOURCE_FILE@0..14
                  MODULE_DEF@0..14
                    MODULE_KW@0..6 "module"
                    WHITESPACE@6..7 " "
                    IDENT@7..10 "Foo"
                    WHITESPACE@10..11 " "
                    L_BRACE@11..12 "{"
                    WHITESPACE@12..13 " "
                    R_BRACE@13..14 "}"
            "#]],
        );
    }

    #[test]
    fn module_with_generics() {
        check(
            "module Foo<T> { }",
            expect![[r#"
                SOURCE_FILE@0..17
                  MODULE_DEF@0..17
                    MODULE_KW@0..6 "module"
                    WHITESPACE@6..7 " "
                    IDENT@7..10 "Foo"
                    GENERIC_PARAM_LIST@10..13
                      LT@10..11 "<"
                      GENERIC_PARAM@11..12
                        IDENT@11..12 "T"
                      GT@12..13 ">"
                    WHITESPACE@13..14 " "
                    L_BRACE@14..15 "{"
                    WHITESPACE@15..16 " "
                    R_BRACE@16..17 "}"
            "#]],
        );
    }

    #[test]
    fn module_exported() {
        check(
            "export module Foo { }",
            expect![[r#"
                SOURCE_FILE@0..21
                  MODULE_DEF@0..21
                    EXPORT_KW@0..6 "export"
                    WHITESPACE@6..7 " "
                    MODULE_KW@7..13 "module"
                    WHITESPACE@13..14 " "
                    IDENT@14..17 "Foo"
                    WHITESPACE@17..18 " "
                    L_BRACE@18..19 "{"
                    WHITESPACE@19..20 " "
                    R_BRACE@20..21 "}"
            "#]],
        );
    }

    #[test]
    fn module_with_contents() {
        check(
            "module Foo { circuit bar() : Field { } }",
            expect![[r#"
                SOURCE_FILE@0..40
                  MODULE_DEF@0..40
                    MODULE_KW@0..6 "module"
                    WHITESPACE@6..7 " "
                    IDENT@7..10 "Foo"
                    WHITESPACE@10..11 " "
                    L_BRACE@11..12 "{"
                    CIRCUIT_DEF@12..38
                      WHITESPACE@12..13 " "
                      CIRCUIT_KW@13..20 "circuit"
                      WHITESPACE@20..21 " "
                      IDENT@21..24 "bar"
                      L_PAREN@24..25 "("
                      R_PAREN@25..26 ")"
                      WHITESPACE@26..27 " "
                      COLON@27..28 ":"
                      FIELD_TYPE@28..34
                        WHITESPACE@28..29 " "
                        FIELD_KW@29..34 "Field"
                      BLOCK@34..38
                        WHITESPACE@34..35 " "
                        L_BRACE@35..36 "{"
                        WHITESPACE@36..37 " "
                        R_BRACE@37..38 "}"
                    WHITESPACE@38..39 " "
                    R_BRACE@39..40 "}"
            "#]],
        );
    }

    #[test]
    fn import_recovery() {
        check(
            "import ;",
            expect![[r#"
                SOURCE_FILE@0..8
                  IMPORT@0..8
                    IMPORT_KW@0..6 "import"
                    WHITESPACE@6..7 " "
                    SEMICOLON@7..8 ";"
                errors:
                  expected module name or string path
            "#]],
        );
    }
}
