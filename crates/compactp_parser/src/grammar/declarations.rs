//! Top-level declaration parsing.
//!
//! This module dispatches to the appropriate parser for each top-level
//! declaration form: pragma, include, import, export, module, ledger,
//! constructor, circuit, witness, contract, struct, enum.

use crate::grammar::{comma_sep, comma_sep1};
use crate::parser::Parser;
use compactp_syntax::SyntaxKind::*;

/// Parse a single top-level declaration (program element / pelt).
pub(crate) fn declaration(p: &mut Parser) {
    match p.current() {
        PRAGMA_KW => pragma(p),
        INCLUDE_KW => include(p),
        IMPORT_KW => super::imports::import(p),
        EXPORT_KW => export_prefixed(p),
        MODULE_KW => super::imports::module_def(p, false),
        LEDGER_KW => ledger(p, false, false),
        SEALED_KW => sealed_prefixed(p),
        CONSTRUCTOR_KW => constructor(p),
        CIRCUIT_KW => circuit_or_decl(p, false, false),
        WITNESS_KW => witness(p, false),
        CONTRACT_KW => contract(p, false),
        STRUCT_KW => struct_def(p, false),
        ENUM_KW => enum_def(p, false),
        TYPE_KW => type_alias(p),
        NEW_KW if p.nth(1) == TYPE_KW => type_alias(p),
        PURE_KW => pure_prefixed(p, false),
        _ => super::error_recover_to_declaration(p),
    }
}

/// `pragma id version-expr ;`
fn pragma(p: &mut Parser) {
    let m = p.start();
    p.bump(PRAGMA_KW);
    p.expect(IDENT);
    super::version::version_expr(p);
    p.expect(SEMICOLON);
    m.complete(p, PRAGMA);
}

/// `include str ;`
fn include(p: &mut Parser) {
    let m = p.start();
    p.bump(INCLUDE_KW);
    p.expect(STRING_LIT);
    p.expect(SEMICOLON);
    m.complete(p, INCLUDE);
}

/// Handle `export` at top level — could be:
/// - `export { id, ... }` — export list
/// - `export module ...` — exported module
/// - `export circuit ...` — exported circuit
/// - `export witness ...` — exported witness
/// - `export contract ...` — exported contract
/// - `export struct ...` — exported struct
/// - `export enum ...` — exported enum
/// - `export ledger ...` — exported ledger
/// - `export pure circuit ...` — exported pure circuit
/// - `export sealed ledger ...` — exported sealed ledger
fn export_prefixed(p: &mut Parser) {
    match p.nth(1) {
        L_BRACE => super::imports::export_list(p),
        MODULE_KW => super::imports::module_def(p, true),
        CIRCUIT_KW => circuit_or_decl(p, true, false),
        WITNESS_KW => witness(p, true),
        CONTRACT_KW => contract(p, true),
        STRUCT_KW => struct_def(p, true),
        ENUM_KW => enum_def(p, true),
        LEDGER_KW => ledger(p, true, false),
        TYPE_KW => type_alias_exported(p),
        NEW_KW if p.nth(2) == TYPE_KW => type_alias_exported(p),
        PURE_KW => pure_prefixed(p, true),
        SEALED_KW => {
            // export sealed ledger ...
            sealed_prefixed_exported(p);
        }
        _ => {
            // Unknown export form — error recovery
            super::error_recover_to_declaration(p);
        }
    }
}

/// Handle `sealed` at top level: `sealed ledger ...`
fn sealed_prefixed(p: &mut Parser) {
    if p.nth(1) == LEDGER_KW {
        ledger(p, false, true);
    } else {
        super::error_recover_to_declaration(p);
    }
}

/// Handle `export sealed ledger ...`
fn sealed_prefixed_exported(p: &mut Parser) {
    // We know p.current() is EXPORT_KW and p.nth(1) is SEALED_KW
    if p.nth(2) == LEDGER_KW {
        ledger(p, true, true);
    } else {
        super::error_recover_to_declaration(p);
    }
}

/// Handle `pure circuit ...` (possibly after `export`)
fn pure_prefixed(p: &mut Parser, has_export: bool) {
    if has_export {
        // Current is EXPORT_KW, nth(1) is PURE_KW, nth(2) should be CIRCUIT_KW
        if p.nth(2) == CIRCUIT_KW {
            circuit_def(p, true, true);
        } else {
            super::error_recover_to_declaration(p);
        }
    } else {
        // Current is PURE_KW
        if p.nth(1) == CIRCUIT_KW {
            circuit_def(p, false, true);
        } else {
            super::error_recover_to_declaration(p);
        }
    }
}

/// `export? sealed? ledger id : type ;`
fn ledger(p: &mut Parser, has_export: bool, has_sealed: bool) {
    let m = p.start();
    if has_export {
        p.bump(EXPORT_KW);
    }
    if has_sealed {
        p.bump(SEALED_KW);
    }
    p.bump(LEDGER_KW);
    p.expect(IDENT);
    p.expect(COLON);
    super::types::ty(p);
    p.expect(SEMICOLON);
    m.complete(p, LEDGER_DECL);
}

/// `constructor ( parg , ... ) block ;?`
fn constructor(p: &mut Parser) {
    let m = p.start();
    p.bump(CONSTRUCTOR_KW);
    p.expect(L_PAREN);
    comma_sep(p, R_PAREN, super::patterns::param);
    p.expect(R_PAREN);
    super::statements::block(p);
    p.eat(SEMICOLON);
    m.complete(p, CONSTRUCTOR_DEF);
}

/// Determine if a `circuit` declaration is a definition (with body) or a
/// declaration (without body, ending in `;`).
/// `export? circuit id gparams? ( arg... ) : type ;`  → CIRCUIT_DECL
/// `export? pure? circuit id gparams? ( parg... ) : type block` → CIRCUIT_DEF
fn circuit_or_decl(p: &mut Parser, has_export: bool, has_pure: bool) {
    // We'll parse far enough to see if there's a block or semicolon after the return type.
    // The simple heuristic: if `pure` is involved, it's always a definition.
    // If no `pure` but there's a body `{`, it's a definition. If `;`, it's a declaration.
    if has_pure {
        circuit_def(p, has_export, true);
    } else {
        // We need to distinguish circuit-def from circuit-decl.
        // Parse shared prefix up to the return type, then check what follows.
        circuit_shared(p, has_export);
    }
}

/// Parse circuit definition: `export? pure? circuit name gparams? ( parg, ... ) : type block`
fn circuit_def(p: &mut Parser, has_export: bool, has_pure: bool) {
    let m = p.start();
    if has_export {
        p.bump(EXPORT_KW);
    }
    if has_pure {
        p.bump(PURE_KW);
    }
    p.bump(CIRCUIT_KW);
    p.expect(IDENT);

    // Optional generic params
    if p.at(LT) {
        super::types::generic_param_list(p);
    }

    p.expect(L_PAREN);
    comma_sep(p, R_PAREN, super::patterns::param);
    p.expect(R_PAREN);

    // Return type — optional in the composable-contracts forward-looking
    // syntax (e.g., `export circuit foo() { ... }` with no `:` clause).
    if p.eat(COLON) {
        super::types::ty(p);
    }

    // Body
    super::statements::block(p);

    m.complete(p, CIRCUIT_DEF);
}

/// Parse a circuit where we don't know yet if it's a def or decl.
/// After the return type, `;` means CIRCUIT_DECL, `{` means CIRCUIT_DEF.
fn circuit_shared(p: &mut Parser, has_export: bool) {
    let m = p.start();
    if has_export {
        p.bump(EXPORT_KW);
    }
    p.bump(CIRCUIT_KW);
    p.expect(IDENT);

    // Optional generic params
    if p.at(LT) {
        super::types::generic_param_list(p);
    }

    p.expect(L_PAREN);
    // In declarations, params are `id : type`. In definitions, they are `pattern : type`.
    // Since `pattern` includes simple `id`, we parse as patterns (superset).
    comma_sep(p, R_PAREN, super::patterns::param);
    p.expect(R_PAREN);

    // Return type — optional for the composable-contracts forward-looking
    // syntax (a definition body may follow directly with no `: type`).
    if p.eat(COLON) {
        super::types::ty(p);
    }

    // Now decide: body or semicolon?
    if p.at(L_BRACE) {
        super::statements::block(p);
        m.complete(p, CIRCUIT_DEF);
    } else {
        p.expect(SEMICOLON);
        m.complete(p, CIRCUIT_DECL);
    }
}

/// `export? witness name gparams? ( arg, ... ) : type ;`
fn witness(p: &mut Parser, has_export: bool) {
    let m = p.start();
    if has_export {
        p.bump(EXPORT_KW);
    }
    p.bump(WITNESS_KW);
    p.expect(IDENT);

    // Optional generic params
    if p.at(LT) {
        super::types::generic_param_list(p);
    }

    p.expect(L_PAREN);
    comma_sep(p, R_PAREN, super::patterns::arg);
    p.expect(R_PAREN);

    p.expect(COLON);
    super::types::ty(p);
    p.expect(SEMICOLON);
    m.complete(p, WITNESS_DECL);
}

/// `export? contract name { member... } ;?`
///
/// Members are typically `pure? circuit ... ;` (`CONTRACT_CIRCUIT`),
/// but the composable-contracts forward-looking syntax also permits
/// other declaration forms inside the body (e.g. `export ledger a : T ;`).
/// We dispatch by the current token: contract-circuit forms go through
/// `contract_circuit` (preserving the existing CST shape); anything else
/// falls back to the general `declaration` dispatcher.
fn contract(p: &mut Parser, has_export: bool) {
    let m = p.start();
    if has_export {
        p.bump(EXPORT_KW);
    }
    p.bump(CONTRACT_KW);
    p.expect(IDENT);
    p.expect(L_BRACE);

    while !p.at(R_BRACE) && !p.at_end() {
        if p.errors_exhausted() {
            break;
        }
        match p.current() {
            CIRCUIT_KW => contract_circuit(p),
            PURE_KW if p.nth(1) == CIRCUIT_KW => contract_circuit(p),
            _ => declaration(p),
        }
    }

    p.expect(R_BRACE);
    p.eat(SEMICOLON);
    m.complete(p, CONTRACT_DECL);
}

/// `pure? circuit id ( arg, ... ) : type ;`
fn contract_circuit(p: &mut Parser) {
    let m = p.start();
    if p.eat(PURE_KW) {
        // pure circuit
    }
    p.expect(CIRCUIT_KW);
    p.expect(IDENT);
    p.expect(L_PAREN);
    comma_sep(p, R_PAREN, super::patterns::arg);
    p.expect(R_PAREN);
    p.expect(COLON);
    super::types::ty(p);
    p.expect(SEMICOLON);
    m.complete(p, CONTRACT_CIRCUIT);
}

/// `export? struct name gparams? { field; ... } ;?`
/// or `export? struct name gparams? { field, ... } ;?`
fn struct_def(p: &mut Parser, has_export: bool) {
    let m = p.start();
    if has_export {
        p.bump(EXPORT_KW);
    }
    p.bump(STRUCT_KW);
    p.expect(IDENT);

    // Optional generic params
    if p.at(LT) {
        super::types::generic_param_list(p);
    }

    p.expect(L_BRACE);

    // Parse fields — they can be separated by `;` or `,`
    while !p.at(R_BRACE) && !p.at_end() {
        if p.errors_exhausted() {
            break;
        }
        super::patterns::arg(p); // arg is `id : type`
        // Eat separator (comma or semicolon)
        if !p.eat(SEMICOLON) && !p.eat(COMMA) {
            // No separator — if not at closing brace, that's an error
            if !p.at(R_BRACE) {
                p.error("expected `;`, `,`, or `}`");
                break;
            }
        }
    }

    p.expect(R_BRACE);
    p.eat(SEMICOLON);
    m.complete(p, STRUCT_DEF);
}

/// `export? enum name { variant, ..., variant } ;?`
fn enum_def(p: &mut Parser, has_export: bool) {
    let m = p.start();
    if has_export {
        p.bump(EXPORT_KW);
    }
    p.bump(ENUM_KW);
    p.expect(IDENT);
    p.expect(L_BRACE);
    comma_sep1(p, R_BRACE, |p| {
        let vm = p.start();
        p.expect(IDENT);
        vm.complete(p, ENUM_VARIANT);
    });
    p.expect(R_BRACE);
    p.eat(SEMICOLON);
    m.complete(p, ENUM_DEF);
}

/// `export? new? type id gparams? = type ;`
fn type_alias(p: &mut Parser) {
    let m = p.start();
    p.eat(NEW_KW);
    p.bump(TYPE_KW);
    p.expect(IDENT);
    if p.at(LT) {
        super::types::generic_param_list(p);
    }
    p.expect(EQ);
    super::types::ty(p);
    p.expect(SEMICOLON);
    m.complete(p, TYPE_DECL);
}

/// `export new? type id gparams? = type ;`
fn type_alias_exported(p: &mut Parser) {
    let m = p.start();
    p.bump(EXPORT_KW);
    p.eat(NEW_KW);
    p.bump(TYPE_KW);
    p.expect(IDENT);
    if p.at(LT) {
        super::types::generic_param_list(p);
    }
    p.expect(EQ);
    super::types::ty(p);
    p.expect(SEMICOLON);
    m.complete(p, TYPE_DECL);
}

#[cfg(test)]
mod tests {
    use crate::grammar::tests::check;
    use expect_test::expect;

    #[test]
    fn decl_pragma() {
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
    fn decl_include() {
        check(
            r#"include "std/lib.compact";"#,
            expect![[r#"
                SOURCE_FILE@0..26
                  INCLUDE@0..26
                    INCLUDE_KW@0..7 "include"
                    WHITESPACE@7..8 " "
                    STRING_LIT@8..25 "\"std/lib.compact\""
                    SEMICOLON@25..26 ";"
            "#]],
        );
    }

    #[test]
    fn decl_ledger() {
        check(
            "ledger myLedger : Field;",
            expect![[r#"
                SOURCE_FILE@0..24
                  LEDGER_DECL@0..24
                    LEDGER_KW@0..6 "ledger"
                    WHITESPACE@6..7 " "
                    IDENT@7..15 "myLedger"
                    WHITESPACE@15..16 " "
                    COLON@16..17 ":"
                    FIELD_TYPE@17..23
                      WHITESPACE@17..18 " "
                      FIELD_KW@18..23 "Field"
                    SEMICOLON@23..24 ";"
            "#]],
        );
    }

    #[test]
    fn decl_sealed_ledger() {
        check(
            "sealed ledger myLedger : Field;",
            expect![[r#"
                SOURCE_FILE@0..31
                  LEDGER_DECL@0..31
                    SEALED_KW@0..6 "sealed"
                    WHITESPACE@6..7 " "
                    LEDGER_KW@7..13 "ledger"
                    WHITESPACE@13..14 " "
                    IDENT@14..22 "myLedger"
                    WHITESPACE@22..23 " "
                    COLON@23..24 ":"
                    FIELD_TYPE@24..30
                      WHITESPACE@24..25 " "
                      FIELD_KW@25..30 "Field"
                    SEMICOLON@30..31 ";"
            "#]],
        );
    }

    #[test]
    fn decl_export_sealed_ledger() {
        check(
            "export sealed ledger myLedger : Field;",
            expect![[r#"
                SOURCE_FILE@0..38
                  LEDGER_DECL@0..38
                    EXPORT_KW@0..6 "export"
                    WHITESPACE@6..7 " "
                    SEALED_KW@7..13 "sealed"
                    WHITESPACE@13..14 " "
                    LEDGER_KW@14..20 "ledger"
                    WHITESPACE@20..21 " "
                    IDENT@21..29 "myLedger"
                    WHITESPACE@29..30 " "
                    COLON@30..31 ":"
                    FIELD_TYPE@31..37
                      WHITESPACE@31..32 " "
                      FIELD_KW@32..37 "Field"
                    SEMICOLON@37..38 ";"
            "#]],
        );
    }

    #[test]
    fn decl_constructor() {
        check(
            "constructor(x: Field) { }",
            expect![[r#"
                SOURCE_FILE@0..25
                  CONSTRUCTOR_DEF@0..25
                    CONSTRUCTOR_KW@0..11 "constructor"
                    L_PAREN@11..12 "("
                    PARAM@12..20
                      IDENT_PAT@12..13
                        IDENT@12..13 "x"
                      COLON@13..14 ":"
                      FIELD_TYPE@14..20
                        WHITESPACE@14..15 " "
                        FIELD_KW@15..20 "Field"
                    R_PAREN@20..21 ")"
                    BLOCK@21..25
                      WHITESPACE@21..22 " "
                      L_BRACE@22..23 "{"
                      WHITESPACE@23..24 " "
                      R_BRACE@24..25 "}"
            "#]],
        );
    }

    #[test]
    fn decl_circuit_def() {
        check(
            "circuit foo(x: Field) : Field { return x; }",
            expect![[r#"
                SOURCE_FILE@0..43
                  CIRCUIT_DEF@0..43
                    CIRCUIT_KW@0..7 "circuit"
                    WHITESPACE@7..8 " "
                    IDENT@8..11 "foo"
                    L_PAREN@11..12 "("
                    PARAM@12..20
                      IDENT_PAT@12..13
                        IDENT@12..13 "x"
                      COLON@13..14 ":"
                      FIELD_TYPE@14..20
                        WHITESPACE@14..15 " "
                        FIELD_KW@15..20 "Field"
                    R_PAREN@20..21 ")"
                    WHITESPACE@21..22 " "
                    COLON@22..23 ":"
                    FIELD_TYPE@23..29
                      WHITESPACE@23..24 " "
                      FIELD_KW@24..29 "Field"
                    BLOCK@29..43
                      WHITESPACE@29..30 " "
                      L_BRACE@30..31 "{"
                      RETURN_STMT@31..41
                        WHITESPACE@31..32 " "
                        RETURN_KW@32..38 "return"
                        NAME_EXPR@38..40
                          WHITESPACE@38..39 " "
                          IDENT@39..40 "x"
                        SEMICOLON@40..41 ";"
                      WHITESPACE@41..42 " "
                      R_BRACE@42..43 "}"
            "#]],
        );
    }

    #[test]
    fn decl_circuit_decl() {
        check(
            "circuit foo(x: Field) : Field;",
            expect![[r#"
                SOURCE_FILE@0..30
                  CIRCUIT_DECL@0..30
                    CIRCUIT_KW@0..7 "circuit"
                    WHITESPACE@7..8 " "
                    IDENT@8..11 "foo"
                    L_PAREN@11..12 "("
                    PARAM@12..20
                      IDENT_PAT@12..13
                        IDENT@12..13 "x"
                      COLON@13..14 ":"
                      FIELD_TYPE@14..20
                        WHITESPACE@14..15 " "
                        FIELD_KW@15..20 "Field"
                    R_PAREN@20..21 ")"
                    WHITESPACE@21..22 " "
                    COLON@22..23 ":"
                    FIELD_TYPE@23..29
                      WHITESPACE@23..24 " "
                      FIELD_KW@24..29 "Field"
                    SEMICOLON@29..30 ";"
            "#]],
        );
    }

    #[test]
    fn decl_export_pure_circuit() {
        check(
            "export pure circuit foo() : Field { }",
            expect![[r#"
                SOURCE_FILE@0..37
                  CIRCUIT_DEF@0..37
                    EXPORT_KW@0..6 "export"
                    WHITESPACE@6..7 " "
                    PURE_KW@7..11 "pure"
                    WHITESPACE@11..12 " "
                    CIRCUIT_KW@12..19 "circuit"
                    WHITESPACE@19..20 " "
                    IDENT@20..23 "foo"
                    L_PAREN@23..24 "("
                    R_PAREN@24..25 ")"
                    WHITESPACE@25..26 " "
                    COLON@26..27 ":"
                    FIELD_TYPE@27..33
                      WHITESPACE@27..28 " "
                      FIELD_KW@28..33 "Field"
                    BLOCK@33..37
                      WHITESPACE@33..34 " "
                      L_BRACE@34..35 "{"
                      WHITESPACE@35..36 " "
                      R_BRACE@36..37 "}"
            "#]],
        );
    }

    #[test]
    fn decl_circuit_generics() {
        check(
            "circuit foo<T>(x: T) : T { return x; }",
            expect![[r#"
                SOURCE_FILE@0..38
                  CIRCUIT_DEF@0..38
                    CIRCUIT_KW@0..7 "circuit"
                    WHITESPACE@7..8 " "
                    IDENT@8..11 "foo"
                    GENERIC_PARAM_LIST@11..14
                      LT@11..12 "<"
                      GENERIC_PARAM@12..13
                        IDENT@12..13 "T"
                      GT@13..14 ">"
                    L_PAREN@14..15 "("
                    PARAM@15..19
                      IDENT_PAT@15..16
                        IDENT@15..16 "x"
                      COLON@16..17 ":"
                      TYPE_REF@17..19
                        WHITESPACE@17..18 " "
                        IDENT@18..19 "T"
                    R_PAREN@19..20 ")"
                    WHITESPACE@20..21 " "
                    COLON@21..22 ":"
                    TYPE_REF@22..24
                      WHITESPACE@22..23 " "
                      IDENT@23..24 "T"
                    BLOCK@24..38
                      WHITESPACE@24..25 " "
                      L_BRACE@25..26 "{"
                      RETURN_STMT@26..36
                        WHITESPACE@26..27 " "
                        RETURN_KW@27..33 "return"
                        NAME_EXPR@33..35
                          WHITESPACE@33..34 " "
                          IDENT@34..35 "x"
                        SEMICOLON@35..36 ";"
                      WHITESPACE@36..37 " "
                      R_BRACE@37..38 "}"
            "#]],
        );
    }

    #[test]
    fn decl_witness() {
        check(
            "witness myWitness(x: Field) : Boolean;",
            expect![[r#"
                SOURCE_FILE@0..38
                  WITNESS_DECL@0..38
                    WITNESS_KW@0..7 "witness"
                    WHITESPACE@7..8 " "
                    IDENT@8..17 "myWitness"
                    L_PAREN@17..18 "("
                    STRUCT_FIELD@18..26
                      IDENT@18..19 "x"
                      COLON@19..20 ":"
                      FIELD_TYPE@20..26
                        WHITESPACE@20..21 " "
                        FIELD_KW@21..26 "Field"
                    R_PAREN@26..27 ")"
                    WHITESPACE@27..28 " "
                    COLON@28..29 ":"
                    BOOLEAN_TYPE@29..37
                      WHITESPACE@29..30 " "
                      BOOLEAN_KW@30..37 "Boolean"
                    SEMICOLON@37..38 ";"
            "#]],
        );
    }

    #[test]
    fn decl_contract() {
        check(
            "contract MyContract { circuit foo(x: Field) : Field; }",
            expect![[r#"
                SOURCE_FILE@0..54
                  CONTRACT_DECL@0..54
                    CONTRACT_KW@0..8 "contract"
                    WHITESPACE@8..9 " "
                    IDENT@9..19 "MyContract"
                    WHITESPACE@19..20 " "
                    L_BRACE@20..21 "{"
                    CONTRACT_CIRCUIT@21..52
                      WHITESPACE@21..22 " "
                      CIRCUIT_KW@22..29 "circuit"
                      WHITESPACE@29..30 " "
                      IDENT@30..33 "foo"
                      L_PAREN@33..34 "("
                      STRUCT_FIELD@34..42
                        IDENT@34..35 "x"
                        COLON@35..36 ":"
                        FIELD_TYPE@36..42
                          WHITESPACE@36..37 " "
                          FIELD_KW@37..42 "Field"
                      R_PAREN@42..43 ")"
                      WHITESPACE@43..44 " "
                      COLON@44..45 ":"
                      FIELD_TYPE@45..51
                        WHITESPACE@45..46 " "
                        FIELD_KW@46..51 "Field"
                      SEMICOLON@51..52 ";"
                    WHITESPACE@52..53 " "
                    R_BRACE@53..54 "}"
            "#]],
        );
    }

    #[test]
    fn decl_struct_semicolons() {
        check(
            "struct Foo { x: Field; y: Boolean; }",
            expect![[r#"
                SOURCE_FILE@0..36
                  STRUCT_DEF@0..36
                    STRUCT_KW@0..6 "struct"
                    WHITESPACE@6..7 " "
                    IDENT@7..10 "Foo"
                    WHITESPACE@10..11 " "
                    L_BRACE@11..12 "{"
                    STRUCT_FIELD@12..21
                      WHITESPACE@12..13 " "
                      IDENT@13..14 "x"
                      COLON@14..15 ":"
                      FIELD_TYPE@15..21
                        WHITESPACE@15..16 " "
                        FIELD_KW@16..21 "Field"
                    SEMICOLON@21..22 ";"
                    STRUCT_FIELD@22..33
                      WHITESPACE@22..23 " "
                      IDENT@23..24 "y"
                      COLON@24..25 ":"
                      BOOLEAN_TYPE@25..33
                        WHITESPACE@25..26 " "
                        BOOLEAN_KW@26..33 "Boolean"
                    SEMICOLON@33..34 ";"
                    WHITESPACE@34..35 " "
                    R_BRACE@35..36 "}"
            "#]],
        );
    }

    #[test]
    fn decl_struct_commas() {
        check(
            "struct Foo { x: Field, y: Boolean }",
            expect![[r#"
                SOURCE_FILE@0..35
                  STRUCT_DEF@0..35
                    STRUCT_KW@0..6 "struct"
                    WHITESPACE@6..7 " "
                    IDENT@7..10 "Foo"
                    WHITESPACE@10..11 " "
                    L_BRACE@11..12 "{"
                    STRUCT_FIELD@12..21
                      WHITESPACE@12..13 " "
                      IDENT@13..14 "x"
                      COLON@14..15 ":"
                      FIELD_TYPE@15..21
                        WHITESPACE@15..16 " "
                        FIELD_KW@16..21 "Field"
                    COMMA@21..22 ","
                    STRUCT_FIELD@22..33
                      WHITESPACE@22..23 " "
                      IDENT@23..24 "y"
                      COLON@24..25 ":"
                      BOOLEAN_TYPE@25..33
                        WHITESPACE@25..26 " "
                        BOOLEAN_KW@26..33 "Boolean"
                    WHITESPACE@33..34 " "
                    R_BRACE@34..35 "}"
            "#]],
        );
    }

    #[test]
    fn decl_enum() {
        check(
            "enum Color { Red, Green, Blue }",
            expect![[r#"
                SOURCE_FILE@0..31
                  ENUM_DEF@0..31
                    ENUM_KW@0..4 "enum"
                    WHITESPACE@4..5 " "
                    IDENT@5..10 "Color"
                    WHITESPACE@10..11 " "
                    L_BRACE@11..12 "{"
                    ENUM_VARIANT@12..16
                      WHITESPACE@12..13 " "
                      IDENT@13..16 "Red"
                    COMMA@16..17 ","
                    ENUM_VARIANT@17..23
                      WHITESPACE@17..18 " "
                      IDENT@18..23 "Green"
                    COMMA@23..24 ","
                    ENUM_VARIANT@24..29
                      WHITESPACE@24..25 " "
                      IDENT@25..29 "Blue"
                    WHITESPACE@29..30 " "
                    R_BRACE@30..31 "}"
            "#]],
        );
    }

    #[test]
    fn decl_export_enum() {
        check(
            "export enum Color { Red, Green };",
            expect![[r#"
                SOURCE_FILE@0..33
                  ENUM_DEF@0..33
                    EXPORT_KW@0..6 "export"
                    WHITESPACE@6..7 " "
                    ENUM_KW@7..11 "enum"
                    WHITESPACE@11..12 " "
                    IDENT@12..17 "Color"
                    WHITESPACE@17..18 " "
                    L_BRACE@18..19 "{"
                    ENUM_VARIANT@19..23
                      WHITESPACE@19..20 " "
                      IDENT@20..23 "Red"
                    COMMA@23..24 ","
                    ENUM_VARIANT@24..30
                      WHITESPACE@24..25 " "
                      IDENT@25..30 "Green"
                    WHITESPACE@30..31 " "
                    R_BRACE@31..32 "}"
                    SEMICOLON@32..33 ";"
            "#]],
        );
    }

    #[test]
    fn decl_export_struct() {
        check(
            "export struct Point { x: Field, y: Field }",
            expect![[r#"
                SOURCE_FILE@0..42
                  STRUCT_DEF@0..42
                    EXPORT_KW@0..6 "export"
                    WHITESPACE@6..7 " "
                    STRUCT_KW@7..13 "struct"
                    WHITESPACE@13..14 " "
                    IDENT@14..19 "Point"
                    WHITESPACE@19..20 " "
                    L_BRACE@20..21 "{"
                    STRUCT_FIELD@21..30
                      WHITESPACE@21..22 " "
                      IDENT@22..23 "x"
                      COLON@23..24 ":"
                      FIELD_TYPE@24..30
                        WHITESPACE@24..25 " "
                        FIELD_KW@25..30 "Field"
                    COMMA@30..31 ","
                    STRUCT_FIELD@31..40
                      WHITESPACE@31..32 " "
                      IDENT@32..33 "y"
                      COLON@33..34 ":"
                      FIELD_TYPE@34..40
                        WHITESPACE@34..35 " "
                        FIELD_KW@35..40 "Field"
                    WHITESPACE@40..41 " "
                    R_BRACE@41..42 "}"
            "#]],
        );
    }

    #[test]
    fn decl_struct_with_generics() {
        check(
            "struct Pair<A, B> { first: A; second: B; }",
            expect![[r#"
                SOURCE_FILE@0..42
                  STRUCT_DEF@0..42
                    STRUCT_KW@0..6 "struct"
                    WHITESPACE@6..7 " "
                    IDENT@7..11 "Pair"
                    GENERIC_PARAM_LIST@11..17
                      LT@11..12 "<"
                      GENERIC_PARAM@12..13
                        IDENT@12..13 "A"
                      COMMA@13..14 ","
                      GENERIC_PARAM@14..16
                        WHITESPACE@14..15 " "
                        IDENT@15..16 "B"
                      GT@16..17 ">"
                    WHITESPACE@17..18 " "
                    L_BRACE@18..19 "{"
                    STRUCT_FIELD@19..28
                      WHITESPACE@19..20 " "
                      IDENT@20..25 "first"
                      COLON@25..26 ":"
                      TYPE_REF@26..28
                        WHITESPACE@26..27 " "
                        IDENT@27..28 "A"
                    SEMICOLON@28..29 ";"
                    STRUCT_FIELD@29..39
                      WHITESPACE@29..30 " "
                      IDENT@30..36 "second"
                      COLON@36..37 ":"
                      TYPE_REF@37..39
                        WHITESPACE@37..38 " "
                        IDENT@38..39 "B"
                    SEMICOLON@39..40 ";"
                    WHITESPACE@40..41 " "
                    R_BRACE@41..42 "}"
            "#]],
        );
    }

    #[test]
    fn decl_export_witness() {
        check(
            "export witness myW(x: Field) : Boolean;",
            expect![[r#"
                SOURCE_FILE@0..39
                  WITNESS_DECL@0..39
                    EXPORT_KW@0..6 "export"
                    WHITESPACE@6..7 " "
                    WITNESS_KW@7..14 "witness"
                    WHITESPACE@14..15 " "
                    IDENT@15..18 "myW"
                    L_PAREN@18..19 "("
                    STRUCT_FIELD@19..27
                      IDENT@19..20 "x"
                      COLON@20..21 ":"
                      FIELD_TYPE@21..27
                        WHITESPACE@21..22 " "
                        FIELD_KW@22..27 "Field"
                    R_PAREN@27..28 ")"
                    WHITESPACE@28..29 " "
                    COLON@29..30 ":"
                    BOOLEAN_TYPE@30..38
                      WHITESPACE@30..31 " "
                      BOOLEAN_KW@31..38 "Boolean"
                    SEMICOLON@38..39 ";"
            "#]],
        );
    }

    #[test]
    fn decl_export_contract() {
        check(
            "export contract MyC { circuit f() : Field; }",
            expect![[r#"
                SOURCE_FILE@0..44
                  CONTRACT_DECL@0..44
                    EXPORT_KW@0..6 "export"
                    WHITESPACE@6..7 " "
                    CONTRACT_KW@7..15 "contract"
                    WHITESPACE@15..16 " "
                    IDENT@16..19 "MyC"
                    WHITESPACE@19..20 " "
                    L_BRACE@20..21 "{"
                    CONTRACT_CIRCUIT@21..42
                      WHITESPACE@21..22 " "
                      CIRCUIT_KW@22..29 "circuit"
                      WHITESPACE@29..30 " "
                      IDENT@30..31 "f"
                      L_PAREN@31..32 "("
                      R_PAREN@32..33 ")"
                      WHITESPACE@33..34 " "
                      COLON@34..35 ":"
                      FIELD_TYPE@35..41
                        WHITESPACE@35..36 " "
                        FIELD_KW@36..41 "Field"
                      SEMICOLON@41..42 ";"
                    WHITESPACE@42..43 " "
                    R_BRACE@43..44 "}"
            "#]],
        );
    }

    #[test]
    fn decl_contract_pure_circuit() {
        check(
            "contract MyC { pure circuit f() : Field; }",
            expect![[r#"
                SOURCE_FILE@0..42
                  CONTRACT_DECL@0..42
                    CONTRACT_KW@0..8 "contract"
                    WHITESPACE@8..9 " "
                    IDENT@9..12 "MyC"
                    WHITESPACE@12..13 " "
                    L_BRACE@13..14 "{"
                    CONTRACT_CIRCUIT@14..40
                      WHITESPACE@14..15 " "
                      PURE_KW@15..19 "pure"
                      WHITESPACE@19..20 " "
                      CIRCUIT_KW@20..27 "circuit"
                      WHITESPACE@27..28 " "
                      IDENT@28..29 "f"
                      L_PAREN@29..30 "("
                      R_PAREN@30..31 ")"
                      WHITESPACE@31..32 " "
                      COLON@32..33 ":"
                      FIELD_TYPE@33..39
                        WHITESPACE@33..34 " "
                        FIELD_KW@34..39 "Field"
                      SEMICOLON@39..40 ";"
                    WHITESPACE@40..41 " "
                    R_BRACE@41..42 "}"
            "#]],
        );
    }

    #[test]
    fn decl_generic_param_hash() {
        check(
            "struct Foo<#N> { x: Uint<N>; }",
            expect![[r##"
                SOURCE_FILE@0..30
                  STRUCT_DEF@0..30
                    STRUCT_KW@0..6 "struct"
                    WHITESPACE@6..7 " "
                    IDENT@7..10 "Foo"
                    GENERIC_PARAM_LIST@10..14
                      LT@10..11 "<"
                      GENERIC_PARAM@11..13
                        HASH@11..12 "#"
                        IDENT@12..13 "N"
                      GT@13..14 ">"
                    WHITESPACE@14..15 " "
                    L_BRACE@15..16 "{"
                    STRUCT_FIELD@16..27
                      WHITESPACE@16..17 " "
                      IDENT@17..18 "x"
                      COLON@18..19 ":"
                      UINT_TYPE@19..27
                        WHITESPACE@19..20 " "
                        UINT_KW@20..24 "Uint"
                        LT@24..25 "<"
                        TYPE_SIZE@25..26
                          IDENT@25..26 "N"
                        GT@26..27 ">"
                    SEMICOLON@27..28 ";"
                    WHITESPACE@28..29 " "
                    R_BRACE@29..30 "}"
            "##]],
        );
    }

    #[test]
    fn decl_recovery_unexpected_token() {
        check(
            "@@@ circuit f() : Field { }",
            expect![[r#"
                SOURCE_FILE@0..27
                  ERROR@0..3
                    ERROR@0..1 "@"
                    ERROR@1..2 "@"
                    ERROR@2..3 "@"
                  CIRCUIT_DEF@3..27
                    WHITESPACE@3..4 " "
                    CIRCUIT_KW@4..11 "circuit"
                    WHITESPACE@11..12 " "
                    IDENT@12..13 "f"
                    L_PAREN@13..14 "("
                    R_PAREN@14..15 ")"
                    WHITESPACE@15..16 " "
                    COLON@16..17 ":"
                    FIELD_TYPE@17..23
                      WHITESPACE@17..18 " "
                      FIELD_KW@18..23 "Field"
                    BLOCK@23..27
                      WHITESPACE@23..24 " "
                      L_BRACE@24..25 "{"
                      WHITESPACE@25..26 " "
                      R_BRACE@26..27 "}"
                errors:
                  unexpected token at top level
            "#]],
        );
    }

    #[test]
    fn decl_export_circuit_decl() {
        check(
            "export circuit foo(x: Field) : Field;",
            expect![[r#"
                SOURCE_FILE@0..37
                  CIRCUIT_DECL@0..37
                    EXPORT_KW@0..6 "export"
                    WHITESPACE@6..7 " "
                    CIRCUIT_KW@7..14 "circuit"
                    WHITESPACE@14..15 " "
                    IDENT@15..18 "foo"
                    L_PAREN@18..19 "("
                    PARAM@19..27
                      IDENT_PAT@19..20
                        IDENT@19..20 "x"
                      COLON@20..21 ":"
                      FIELD_TYPE@21..27
                        WHITESPACE@21..22 " "
                        FIELD_KW@22..27 "Field"
                    R_PAREN@27..28 ")"
                    WHITESPACE@28..29 " "
                    COLON@29..30 ":"
                    FIELD_TYPE@30..36
                      WHITESPACE@30..31 " "
                      FIELD_KW@31..36 "Field"
                    SEMICOLON@36..37 ";"
            "#]],
        );
    }

    #[test]
    fn decl_export_circuit_without_return_type() {
        // Composable-contracts forward-looking syntax: an exported
        // circuit definition with no `: type` clause and a nested
        // `contract` in statement position. Mirrors
        // tests/corpus/composable/cases/contract-in-circuit/main.compact.
        check(
            "export circuit alejandro() { contract AB { circuit up(x: Field): []; } }",
            expect![[r#"
                SOURCE_FILE@0..72
                  CIRCUIT_DEF@0..72
                    EXPORT_KW@0..6 "export"
                    WHITESPACE@6..7 " "
                    CIRCUIT_KW@7..14 "circuit"
                    WHITESPACE@14..15 " "
                    IDENT@15..24 "alejandro"
                    L_PAREN@24..25 "("
                    R_PAREN@25..26 ")"
                    BLOCK@26..72
                      WHITESPACE@26..27 " "
                      L_BRACE@27..28 "{"
                      CONTRACT_DECL@28..70
                        WHITESPACE@28..29 " "
                        CONTRACT_KW@29..37 "contract"
                        WHITESPACE@37..38 " "
                        IDENT@38..40 "AB"
                        WHITESPACE@40..41 " "
                        L_BRACE@41..42 "{"
                        CONTRACT_CIRCUIT@42..68
                          WHITESPACE@42..43 " "
                          CIRCUIT_KW@43..50 "circuit"
                          WHITESPACE@50..51 " "
                          IDENT@51..53 "up"
                          L_PAREN@53..54 "("
                          STRUCT_FIELD@54..62
                            IDENT@54..55 "x"
                            COLON@55..56 ":"
                            FIELD_TYPE@56..62
                              WHITESPACE@56..57 " "
                              FIELD_KW@57..62 "Field"
                          R_PAREN@62..63 ")"
                          COLON@63..64 ":"
                          TUPLE_TYPE@64..67
                            WHITESPACE@64..65 " "
                            L_BRACKET@65..66 "["
                            R_BRACKET@66..67 "]"
                          SEMICOLON@67..68 ";"
                        WHITESPACE@68..69 " "
                        R_BRACE@69..70 "}"
                      WHITESPACE@70..71 " "
                      R_BRACE@71..72 "}"
            "#]],
        );
    }

    #[test]
    fn decl_contract_in_constructor() {
        // Composable-contracts forward-looking syntax: a constructor
        // body contains a nested `contract` declaration. Mirrors
        // tests/corpus/composable/cases/contract-in-constructor/main.compact.
        check(
            "constructor() { contract AB { circuit up(x: Field): []; } }",
            expect![[r#"
                SOURCE_FILE@0..59
                  CONSTRUCTOR_DEF@0..59
                    CONSTRUCTOR_KW@0..11 "constructor"
                    L_PAREN@11..12 "("
                    R_PAREN@12..13 ")"
                    BLOCK@13..59
                      WHITESPACE@13..14 " "
                      L_BRACE@14..15 "{"
                      CONTRACT_DECL@15..57
                        WHITESPACE@15..16 " "
                        CONTRACT_KW@16..24 "contract"
                        WHITESPACE@24..25 " "
                        IDENT@25..27 "AB"
                        WHITESPACE@27..28 " "
                        L_BRACE@28..29 "{"
                        CONTRACT_CIRCUIT@29..55
                          WHITESPACE@29..30 " "
                          CIRCUIT_KW@30..37 "circuit"
                          WHITESPACE@37..38 " "
                          IDENT@38..40 "up"
                          L_PAREN@40..41 "("
                          STRUCT_FIELD@41..49
                            IDENT@41..42 "x"
                            COLON@42..43 ":"
                            FIELD_TYPE@43..49
                              WHITESPACE@43..44 " "
                              FIELD_KW@44..49 "Field"
                          R_PAREN@49..50 ")"
                          COLON@50..51 ":"
                          TUPLE_TYPE@51..54
                            WHITESPACE@51..52 " "
                            L_BRACKET@52..53 "["
                            R_BRACKET@53..54 "]"
                          SEMICOLON@54..55 ";"
                        WHITESPACE@55..56 " "
                        R_BRACE@56..57 "}"
                      WHITESPACE@57..58 " "
                      R_BRACE@58..59 "}"
            "#]],
        );
    }

    #[test]
    fn decl_contract_with_export_ledger() {
        // Composable-contracts forward-looking syntax: a contract body
        // may contain `export ledger`. Mirrors
        // tests/corpus/composable/cases/export-in-definition/main.compact.
        check(
            "contract A { export ledger a: A; }",
            expect![[r#"
                SOURCE_FILE@0..34
                  CONTRACT_DECL@0..34
                    CONTRACT_KW@0..8 "contract"
                    WHITESPACE@8..9 " "
                    IDENT@9..10 "A"
                    WHITESPACE@10..11 " "
                    L_BRACE@11..12 "{"
                    LEDGER_DECL@12..32
                      WHITESPACE@12..13 " "
                      EXPORT_KW@13..19 "export"
                      WHITESPACE@19..20 " "
                      LEDGER_KW@20..26 "ledger"
                      WHITESPACE@26..27 " "
                      IDENT@27..28 "a"
                      COLON@28..29 ":"
                      TYPE_REF@29..31
                        WHITESPACE@29..30 " "
                        IDENT@30..31 "A"
                      SEMICOLON@31..32 ";"
                    WHITESPACE@32..33 " "
                    R_BRACE@33..34 "}"
            "#]],
        );
    }

    #[test]
    fn multiple_decls() {
        check(
            "pragma compact 0.15.0;\ncircuit f() : Field { }",
            expect![[r#"
                SOURCE_FILE@0..46
                  PRAGMA@0..22
                    PRAGMA_KW@0..6 "pragma"
                    WHITESPACE@6..7 " "
                    IDENT@7..14 "compact"
                    VERSION_EXPR@14..21
                      WHITESPACE@14..15 " "
                      VERSION_LIT@15..21 "0.15.0"
                    SEMICOLON@21..22 ";"
                  CIRCUIT_DEF@22..46
                    WHITESPACE@22..23 "\n"
                    CIRCUIT_KW@23..30 "circuit"
                    WHITESPACE@30..31 " "
                    IDENT@31..32 "f"
                    L_PAREN@32..33 "("
                    R_PAREN@33..34 ")"
                    WHITESPACE@34..35 " "
                    COLON@35..36 ":"
                    FIELD_TYPE@36..42
                      WHITESPACE@36..37 " "
                      FIELD_KW@37..42 "Field"
                    BLOCK@42..46
                      WHITESPACE@42..43 " "
                      L_BRACE@43..44 "{"
                      WHITESPACE@44..45 " "
                      R_BRACE@45..46 "}"
            "#]],
        );
    }
}
