//! Typed AST wrappers for the Compact language.
//!
//! This crate provides zero-cost typed access to the lossless CST produced by
//! `compactp_parser`. Each AST type is a newtype over [`SyntaxNode`] that
//! exposes typed accessor methods for navigating the tree. No allocation or
//! cloning is required -- AST nodes are simply views into the existing CST.
//!
//! The crate depends only on `compactp_syntax` (not `compactp_parser`), so the
//! AST layer works over rowan [`SyntaxNode`] values regardless of how they were
//! produced.
//!
//! # Usage
//!
//! ```rust,ignore
//! use compactp_syntax::SyntaxNode;
//! use compactp_ast::{AstNode, nodes::*};
//!
//! let result = compactp_parser::parse(source);
//! let root = SyntaxNode::new_root(result.green);
//! let file = SourceFile::cast(root).expect("root should be SOURCE_FILE");
//! for circuit in file.circuit_defs() {
//!     println!("{:?}", circuit.name());
//! }
//! ```

pub mod expr;
pub mod nodes;
pub mod support;

// Re-export key types from compactp_syntax for convenience.
pub use compactp_syntax::{SyntaxKind, SyntaxNode, SyntaxToken};

/// Trait implemented by all typed AST node wrappers.
///
/// Provides the ability to check, cast, and unwrap [`SyntaxNode`] values into
/// their strongly-typed AST representations. All implementations are zero-cost:
/// casting is a simple kind check followed by a newtype wrap.
pub trait AstNode: Sized {
    /// Returns `true` if a [`SyntaxNode`] with the given kind can be cast to
    /// this AST type.
    fn can_cast(kind: SyntaxKind) -> bool;

    /// Attempt to cast a [`SyntaxNode`] into this AST type.
    ///
    /// Returns `Some(Self)` if the node's kind matches, `None` otherwise.
    fn cast(node: SyntaxNode) -> Option<Self>;

    /// Return a reference to the underlying [`SyntaxNode`].
    fn syntax(&self) -> &SyntaxNode;
}

// Re-export the most commonly used types at the crate root.
pub use expr::Expr;
pub use nodes::*;

#[cfg(test)]
mod tests {
    use compactp_syntax::SyntaxNode;

    use crate::AstNode;
    use crate::expr::*;
    use crate::nodes::*;

    /// Helper: parse source and return the root SyntaxNode.
    fn parse(source: &str) -> SyntaxNode {
        let result = compactp_parser::parse(source);
        SyntaxNode::new_root(result.green)
    }

    /// Find the first child of the root SourceFile that casts to the given type.
    fn root_child<N: AstNode>(file: &SourceFile) -> N {
        file.syntax()
            .children()
            .find_map(N::cast)
            .expect("expected child node")
    }

    // -----------------------------------------------------------------------
    // SourceFile
    // -----------------------------------------------------------------------

    #[test]
    fn source_file_cast() {
        let root = parse("");
        let file = SourceFile::cast(root).expect("root should be SOURCE_FILE");
        assert_eq!(
            file.syntax().kind(),
            compactp_syntax::SyntaxKind::SOURCE_FILE
        );
    }

    #[test]
    fn source_file_multiple_items() {
        let source = "ledger x: Field;\nledger y: Field;";
        let root = parse(source);
        let sf = SourceFile::cast(root).expect("root should be SourceFile");
        let items: Vec<_> = sf.syntax().children().collect();
        assert_eq!(items.len(), 2);
    }

    // -----------------------------------------------------------------------
    // Pragma
    // -----------------------------------------------------------------------

    #[test]
    fn pragma_accessors() {
        let root = parse("pragma compact 0.15.0;");
        let file = SourceFile::cast(root).unwrap();
        let pragma = file.pragmas().next().expect("should have Pragma");
        assert_eq!(pragma.name().unwrap().text(), "compact");
    }

    // -----------------------------------------------------------------------
    // Include
    // -----------------------------------------------------------------------

    #[test]
    fn include_accessors() {
        let root = parse(r#"include "std/lib.compact";"#);
        let file = SourceFile::cast(root).unwrap();
        let inc = file.includes().next().expect("should have Include");
        assert_eq!(inc.path().unwrap().text(), "\"std/lib.compact\"");
    }

    // -----------------------------------------------------------------------
    // Import
    // -----------------------------------------------------------------------

    #[test]
    fn import_ident() {
        let root = parse("import Foo;");
        let file = SourceFile::cast(root).unwrap();
        let imp = file.imports().next().expect("should have Import");
        assert_eq!(imp.name().unwrap().text(), "Foo");
        assert!(imp.path().is_none());
    }

    #[test]
    fn import_string_path() {
        let root = parse(r#"import "path/to/module";"#);
        let file = SourceFile::cast(root).unwrap();
        let imp = file.imports().next().expect("should have Import");
        assert!(imp.path().is_some());
    }

    #[test]
    fn import_with_generics() {
        let root = parse("import Foo<10, Field>;");
        let file = SourceFile::cast(root).unwrap();
        let imp = file.imports().next().unwrap();
        assert!(imp.generic_args().is_some());
    }

    #[test]
    fn import_with_prefix() {
        let root = parse("import Foo prefix bar;");
        let file = SourceFile::cast(root).unwrap();
        let imp = file.imports().next().unwrap();
        let prefix = imp.prefix().expect("should have prefix");
        assert_eq!(prefix.name().unwrap().text(), "bar");
    }

    // -----------------------------------------------------------------------
    // ExportList
    // -----------------------------------------------------------------------

    #[test]
    fn export_list_names() {
        let root = parse("export { foo, bar };");
        let file = SourceFile::cast(root).unwrap();
        let el: ExportList = root_child(&file);
        let names: Vec<_> = el.names().map(|t| t.text().to_string()).collect();
        assert_eq!(names, vec!["foo", "bar"]);
    }

    // -----------------------------------------------------------------------
    // ModuleDef
    // -----------------------------------------------------------------------

    #[test]
    fn module_def_accessors() {
        let root = parse("export module Foo<T> { }");
        let file = SourceFile::cast(root).unwrap();
        let m: ModuleDef = root_child(&file);
        assert!(m.is_exported());
        assert_eq!(m.name().unwrap().text(), "Foo");
        assert!(m.generic_params().is_some());
    }

    // -----------------------------------------------------------------------
    // LedgerDecl
    // -----------------------------------------------------------------------

    #[test]
    fn ledger_decl_accessors() {
        let root = parse("export sealed ledger myLedger : Field;");
        let file = SourceFile::cast(root).unwrap();
        let l: LedgerDecl = root_child(&file);
        assert!(l.is_exported());
        assert!(l.is_sealed());
        assert_eq!(l.name().unwrap().text(), "myLedger");
        assert!(l.ty().is_some());
    }

    #[test]
    fn ledger_not_sealed() {
        let root = parse("ledger myLedger : Field;");
        let file = SourceFile::cast(root).unwrap();
        let l: LedgerDecl = root_child(&file);
        assert!(!l.is_exported());
        assert!(!l.is_sealed());
    }

    // -----------------------------------------------------------------------
    // ConstructorDef
    // -----------------------------------------------------------------------

    #[test]
    fn constructor_def_accessors() {
        let root = parse("constructor(x: Field) { }");
        let file = SourceFile::cast(root).unwrap();
        let c: ConstructorDef = root_child(&file);
        assert_eq!(c.params().count(), 1);
        assert!(c.body().is_some());
    }

    // -----------------------------------------------------------------------
    // CircuitDef
    // -----------------------------------------------------------------------

    #[test]
    fn circuit_def_accessors() {
        let root = parse("export pure circuit foo(x: Field) : Field { return x; }");
        let file = SourceFile::cast(root).unwrap();
        let circuit = file.circuit_defs().next().expect("should have CircuitDef");
        assert!(circuit.is_exported());
        assert!(circuit.is_pure());
        assert_eq!(circuit.name().unwrap().text(), "foo");
        assert_eq!(circuit.params().count(), 1);
        assert!(circuit.return_type().is_some());
        assert!(circuit.body().is_some());
    }

    #[test]
    fn circuit_def_not_exported_not_pure() {
        let root = parse("circuit foo() : Field { }");
        let file = SourceFile::cast(root).unwrap();
        let circuit = file.circuit_defs().next().unwrap();
        assert!(!circuit.is_exported());
        assert!(!circuit.is_pure());
    }

    #[test]
    fn circuit_def_with_generics() {
        let root = parse("circuit foo<T>(x: T) : T { return x; }");
        let file = SourceFile::cast(root).unwrap();
        let circuit = file.circuit_defs().next().unwrap();
        let gp = circuit.generic_params().expect("should have generics");
        assert_eq!(gp.params().count(), 1);
        let first_param = gp.params().next().unwrap();
        assert_eq!(first_param.name().unwrap().text(), "T");
        assert!(!first_param.is_numeric());
    }

    #[test]
    fn numeric_generic_param() {
        let root = parse("struct Foo<#N> { x: Uint<N>; }");
        let file = SourceFile::cast(root).unwrap();
        let s: StructDef = root_child(&file);
        let gp = s.generic_params().unwrap();
        let param = gp.params().next().unwrap();
        assert!(param.is_numeric());
        assert_eq!(param.name().unwrap().text(), "N");
    }

    // -----------------------------------------------------------------------
    // CircuitDecl
    // -----------------------------------------------------------------------

    #[test]
    fn circuit_decl_accessors() {
        let root = parse("export circuit foo(x: Field) : Field;");
        let file = SourceFile::cast(root).unwrap();
        let decl: CircuitDecl = root_child(&file);
        assert!(decl.is_exported());
        assert_eq!(decl.name().unwrap().text(), "foo");
        assert_eq!(decl.params().count(), 1);
        assert!(decl.return_type().is_some());
    }

    // -----------------------------------------------------------------------
    // WitnessDecl
    // -----------------------------------------------------------------------

    #[test]
    fn witness_decl_accessors() {
        let root = parse("export witness myW(x: Field) : Boolean;");
        let file = SourceFile::cast(root).unwrap();
        let w: WitnessDecl = root_child(&file);
        assert!(w.is_exported());
        assert_eq!(w.name().unwrap().text(), "myW");
        assert!(w.return_type().is_some());
    }

    // -----------------------------------------------------------------------
    // ContractDecl
    // -----------------------------------------------------------------------

    #[test]
    fn contract_decl_accessors() {
        let root = parse("export contract MyC { circuit f() : Field; }");
        let file = SourceFile::cast(root).unwrap();
        let c: ContractDecl = root_child(&file);
        assert!(c.is_exported());
        assert_eq!(c.name().unwrap().text(), "MyC");
        let circuits: Vec<_> = c.circuits().collect();
        assert_eq!(circuits.len(), 1);
        assert_eq!(circuits[0].name().unwrap().text(), "f");
    }

    #[test]
    fn contract_pure_circuit() {
        let root = parse("contract MyC { pure circuit f() : Field; }");
        let file = SourceFile::cast(root).unwrap();
        let c: ContractDecl = root_child(&file);
        let cc = c.circuits().next().unwrap();
        assert!(cc.is_pure());
    }

    // -----------------------------------------------------------------------
    // StructDef
    // -----------------------------------------------------------------------

    #[test]
    fn struct_def_accessors() {
        let root = parse("export struct Point { x: Field; y: Boolean; }");
        let file = SourceFile::cast(root).unwrap();
        let s = file.struct_defs().next().expect("should have StructDef");
        assert!(s.is_exported());
        assert_eq!(s.name().unwrap().text(), "Point");
        let fields: Vec<_> = s.fields().collect();
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].name().unwrap().text(), "x");
        assert_eq!(fields[1].name().unwrap().text(), "y");
    }

    #[test]
    fn struct_field_type() {
        let root = parse("struct Foo { x: Field; }");
        let file = SourceFile::cast(root).unwrap();
        let s: StructDef = root_child(&file);
        let field = s.fields().next().unwrap();
        let ty = field.ty().expect("field should have type");
        assert!(matches!(ty, Type::Field(_)));
    }

    // -----------------------------------------------------------------------
    // EnumDef
    // -----------------------------------------------------------------------

    #[test]
    fn enum_def_accessors() {
        let root = parse("export enum Color { Red, Green, Blue }");
        let file = SourceFile::cast(root).unwrap();
        let e = file.enum_defs().next().expect("should have EnumDef");
        assert!(e.is_exported());
        assert_eq!(e.name().unwrap().text(), "Color");
        let variants: Vec<_> = e
            .variants()
            .map(|v| v.name().unwrap().text().to_string())
            .collect();
        assert_eq!(variants, vec!["Red", "Green", "Blue"]);
    }

    // -----------------------------------------------------------------------
    // Type sum type
    // -----------------------------------------------------------------------

    #[test]
    fn type_boolean() {
        let root = parse("circuit f() : Boolean { }");
        let file = SourceFile::cast(root).unwrap();
        let circuit = file.circuit_defs().next().unwrap();
        assert!(matches!(circuit.return_type().unwrap(), Type::Boolean(_)));
    }

    #[test]
    fn type_field() {
        let root = parse("circuit f() : Field { }");
        let file = SourceFile::cast(root).unwrap();
        let circuit = file.circuit_defs().next().unwrap();
        assert!(matches!(circuit.return_type().unwrap(), Type::Field(_)));
    }

    #[test]
    fn type_uint() {
        let root = parse("circuit f() : Uint<8> { }");
        let file = SourceFile::cast(root).unwrap();
        let circuit = file.circuit_defs().next().unwrap();
        match circuit.return_type().unwrap() {
            Type::Uint(u) => assert_eq!(u.sizes().count(), 1),
            other => panic!("expected Uint, got {other:?}"),
        }
    }

    #[test]
    fn type_uint_range() {
        let root = parse("circuit f() : Uint<1..8> { }");
        let file = SourceFile::cast(root).unwrap();
        let circuit = file.circuit_defs().next().unwrap();
        match circuit.return_type().unwrap() {
            Type::Uint(u) => assert_eq!(u.sizes().count(), 2),
            other => panic!("expected Uint, got {other:?}"),
        }
    }

    #[test]
    fn type_bytes() {
        let root = parse("circuit f() : Bytes<32> { }");
        let file = SourceFile::cast(root).unwrap();
        let circuit = file.circuit_defs().next().unwrap();
        match circuit.return_type().unwrap() {
            Type::Bytes(b) => assert!(b.size().is_some()),
            other => panic!("expected Bytes, got {other:?}"),
        }
    }

    #[test]
    fn type_opaque() {
        let root = parse(r#"circuit f() : Opaque<"foo"> { }"#);
        let file = SourceFile::cast(root).unwrap();
        let circuit = file.circuit_defs().next().unwrap();
        match circuit.return_type().unwrap() {
            Type::Opaque(o) => assert_eq!(o.tag().unwrap().text(), "\"foo\""),
            other => panic!("expected Opaque, got {other:?}"),
        }
    }

    #[test]
    fn type_vector() {
        let root = parse("circuit f() : Vector<10, Field> { }");
        let file = SourceFile::cast(root).unwrap();
        let circuit = file.circuit_defs().next().unwrap();
        match circuit.return_type().unwrap() {
            Type::Vector(v) => {
                assert!(v.size().is_some());
                assert!(v.element_type().is_some());
            }
            other => panic!("expected Vector, got {other:?}"),
        }
    }

    #[test]
    fn type_tuple() {
        let root = parse("circuit f() : [Field, Boolean] { }");
        let file = SourceFile::cast(root).unwrap();
        let circuit = file.circuit_defs().next().unwrap();
        match circuit.return_type().unwrap() {
            Type::Tuple(t) => assert_eq!(t.element_types().count(), 2),
            other => panic!("expected Tuple, got {other:?}"),
        }
    }

    #[test]
    fn type_ref_simple() {
        let root = parse("circuit f() : MyType { }");
        let file = SourceFile::cast(root).unwrap();
        let circuit = file.circuit_defs().next().unwrap();
        match circuit.return_type().unwrap() {
            Type::Ref(r) => {
                assert_eq!(r.name().unwrap().text(), "MyType");
                assert!(r.generic_args().is_none());
            }
            other => panic!("expected TypeRef, got {other:?}"),
        }
    }

    #[test]
    fn type_ref_with_generics() {
        let root = parse("circuit f() : MyType<Field, 10> { }");
        let file = SourceFile::cast(root).unwrap();
        let circuit = file.circuit_defs().next().unwrap();
        match circuit.return_type().unwrap() {
            Type::Ref(r) => {
                assert_eq!(r.name().unwrap().text(), "MyType");
                let ga = r.generic_args().unwrap();
                assert_eq!(ga.args().count(), 2);
            }
            other => panic!("expected TypeRef, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // Statement types
    // -----------------------------------------------------------------------

    #[test]
    fn block_stmts() {
        let root = parse("circuit f() : Field { return 1; return 2; }");
        let file = SourceFile::cast(root).unwrap();
        let circuit = file.circuit_defs().next().unwrap();
        let block = circuit.body().unwrap();
        let stmts: Vec<_> = block.stmts().collect();
        assert_eq!(stmts.len(), 2);
        assert!(matches!(stmts[0], Stmt::Return(_)));
        assert!(matches!(stmts[1], Stmt::Return(_)));
    }

    #[test]
    fn stmt_return_bare() {
        let root = parse("circuit f() : Field { return; }");
        let file = SourceFile::cast(root).unwrap();
        let circuit = file.circuit_defs().next().unwrap();
        let block = circuit.body().unwrap();
        match block.stmts().next().unwrap() {
            Stmt::Return(r) => assert!(r.value().is_none()),
            other => panic!("expected Return, got {other:?}"),
        }
    }

    #[test]
    fn stmt_if_else() {
        let root = parse("circuit f() : Field { if (x) { return 1; } else { return 2; } }");
        let file = SourceFile::cast(root).unwrap();
        let circuit = file.circuit_defs().next().unwrap();
        let block = circuit.body().unwrap();
        match block.stmts().next().unwrap() {
            Stmt::If(i) => {
                assert!(i.then_branch().is_some());
                assert!(i.else_kw().is_some());
            }
            other => panic!("expected If, got {other:?}"),
        }
    }

    #[test]
    fn stmt_for() {
        let root = parse("circuit f() : Field { for (const i of 0..10) { x; } }");
        let file = SourceFile::cast(root).unwrap();
        let circuit = file.circuit_defs().next().unwrap();
        let block = circuit.body().unwrap();
        match block.stmts().next().unwrap() {
            Stmt::For(f) => {
                assert_eq!(f.var_name().unwrap().text(), "i");
                assert!(f.body().is_some());
            }
            other => panic!("expected For, got {other:?}"),
        }
    }

    #[test]
    fn stmt_assert_call_form() {
        let root = parse(r#"circuit f() : Field { assert(x, "fail"); }"#);
        let file = SourceFile::cast(root).unwrap();
        let circuit = file.circuit_defs().next().unwrap();
        let block = circuit.body().unwrap();
        // Assert is now parsed as an expression (CALL_EXPR) inside EXPR_STMT
        match block.stmts().next().unwrap() {
            Stmt::Expr(e) => {
                assert_eq!(e.syntax().kind(), compactp_syntax::SyntaxKind::EXPR_STMT);
            }
            other => panic!("expected Expr, got {other:?}"),
        }
    }

    #[test]
    fn stmt_const_typed() {
        let root = parse("circuit f() : Field { const x : Field = 1; }");
        let file = SourceFile::cast(root).unwrap();
        let circuit = file.circuit_defs().next().unwrap();
        let block = circuit.body().unwrap();
        match block.stmts().next().unwrap() {
            Stmt::Const(c) => {
                match c.pattern().expect("should have pattern") {
                    Pat::Ident(i) => assert_eq!(i.name().unwrap().text(), "x"),
                    other => panic!("expected IdentPat, got {other:?}"),
                }
                assert!(c.ty().is_some());
            }
            other => panic!("expected Const, got {other:?}"),
        }
    }

    #[test]
    fn stmt_assign() {
        let root = parse("circuit f() : Field { x = 1; }");
        let file = SourceFile::cast(root).unwrap();
        let circuit = file.circuit_defs().next().unwrap();
        let block = circuit.body().unwrap();
        match block.stmts().next().unwrap() {
            Stmt::Assign(a) => assert_eq!(a.op().unwrap().text(), "="),
            other => panic!("expected Assign, got {other:?}"),
        }
    }

    #[test]
    fn stmt_plus_assign() {
        let root = parse("circuit f() : Field { x += 1; }");
        let file = SourceFile::cast(root).unwrap();
        let circuit = file.circuit_defs().next().unwrap();
        let block = circuit.body().unwrap();
        match block.stmts().next().unwrap() {
            Stmt::Assign(a) => assert_eq!(a.op().unwrap().text(), "+="),
            other => panic!("expected Assign, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // Pattern types
    // -----------------------------------------------------------------------

    #[test]
    fn pat_ident() {
        let root = parse("circuit f() : Field { const x = 1; }");
        let file = SourceFile::cast(root).unwrap();
        let circuit = file.circuit_defs().next().unwrap();
        let block = circuit.body().unwrap();
        match block.stmts().next().unwrap() {
            Stmt::Const(c) => match c.pattern().unwrap() {
                Pat::Ident(i) => assert_eq!(i.name().unwrap().text(), "x"),
                other => panic!("expected IdentPat, got {other:?}"),
            },
            other => panic!("expected Const, got {other:?}"),
        }
    }

    #[test]
    fn pat_tuple() {
        let root = parse("circuit f() : Field { const [a, b] = x; }");
        let file = SourceFile::cast(root).unwrap();
        let circuit = file.circuit_defs().next().unwrap();
        let block = circuit.body().unwrap();
        match block.stmts().next().unwrap() {
            Stmt::Const(c) => match c.pattern().unwrap() {
                Pat::Tuple(t) => assert_eq!(t.elements().count(), 2),
                other => panic!("expected TuplePat, got {other:?}"),
            },
            other => panic!("expected Const, got {other:?}"),
        }
    }

    #[test]
    fn pat_struct() {
        let root = parse("circuit f() : Field { const {a, b: c} = x; }");
        let file = SourceFile::cast(root).unwrap();
        let circuit = file.circuit_defs().next().unwrap();
        let block = circuit.body().unwrap();
        match block.stmts().next().unwrap() {
            Stmt::Const(c) => match c.pattern().unwrap() {
                Pat::Struct(s) => {
                    let fields: Vec<_> = s.fields().collect();
                    assert_eq!(fields.len(), 2);
                    assert_eq!(fields[0].name().unwrap().text(), "a");
                    assert_eq!(fields[1].name().unwrap().text(), "b");
                    match fields[1].pattern().unwrap() {
                        Pat::Ident(i) => assert_eq!(i.name().unwrap().text(), "c"),
                        other => panic!("expected IdentPat, got {other:?}"),
                    }
                }
                other => panic!("expected StructPat, got {other:?}"),
            },
            other => panic!("expected Const, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // Expression types
    // -----------------------------------------------------------------------

    #[test]
    fn expr_binary() {
        let root = parse("circuit f() : Field { return a + b; }");
        let file = SourceFile::cast(root).unwrap();
        let circuit = file.circuit_defs().next().unwrap();
        let block = circuit.body().unwrap();
        match block.stmts().next().unwrap() {
            Stmt::Return(r) => {
                let binary = r
                    .syntax()
                    .descendants()
                    .find_map(BinaryExpr::cast)
                    .expect("should find BinaryExpr");
                assert_eq!(binary.op().unwrap().text(), "+");
            }
            other => panic!("expected Return, got {other:?}"),
        }
    }

    #[test]
    fn expr_unary() {
        let root = parse("circuit f() : Field { return !x; }");
        let file = SourceFile::cast(root).unwrap();
        let circuit = file.circuit_defs().next().unwrap();
        let block = circuit.body().unwrap();
        match block.stmts().next().unwrap() {
            Stmt::Return(r) => {
                let unary = r
                    .syntax()
                    .descendants()
                    .find_map(UnaryExpr::cast)
                    .expect("should find UnaryExpr");
                assert_eq!(unary.op().unwrap().text(), "!");
            }
            other => panic!("expected Return, got {other:?}"),
        }
    }

    #[test]
    fn expr_ternary() {
        let root = parse("circuit f() : Field { return a ? b : c; }");
        let file = SourceFile::cast(root).unwrap();
        let circuit = file.circuit_defs().next().unwrap();
        let block = circuit.body().unwrap();
        match block.stmts().next().unwrap() {
            Stmt::Return(r) => {
                let ternary = r
                    .syntax()
                    .descendants()
                    .find_map(TernaryExpr::cast)
                    .expect("should find TernaryExpr");
                assert!(ternary.question().is_some());
            }
            other => panic!("expected Return, got {other:?}"),
        }
    }

    #[test]
    fn expr_call() {
        let root = parse("circuit f() : Field { return g(x); }");
        let file = SourceFile::cast(root).unwrap();
        let circuit = file.circuit_defs().next().unwrap();
        let block = circuit.body().unwrap();
        match block.stmts().next().unwrap() {
            Stmt::Return(r) => {
                let call = r
                    .syntax()
                    .descendants()
                    .find_map(CallExpr::cast)
                    .expect("should find CallExpr");
                assert_eq!(call.name().unwrap().text(), "g");
            }
            other => panic!("expected Return, got {other:?}"),
        }
    }

    #[test]
    fn expr_member() {
        let root = parse("circuit f() : Field { return x.y; }");
        let file = SourceFile::cast(root).unwrap();
        let circuit = file.circuit_defs().next().unwrap();
        let block = circuit.body().unwrap();
        match block.stmts().next().unwrap() {
            Stmt::Return(r) => {
                let member = r
                    .syntax()
                    .descendants()
                    .find_map(MemberExpr::cast)
                    .expect("should find MemberExpr");
                assert_eq!(member.field().unwrap().text(), "y");
            }
            other => panic!("expected Return, got {other:?}"),
        }
    }

    #[test]
    fn expr_index() {
        let root = parse("circuit f() : Field { return x[0]; }");
        let file = SourceFile::cast(root).unwrap();
        let circuit = file.circuit_defs().next().unwrap();
        let block = circuit.body().unwrap();
        match block.stmts().next().unwrap() {
            Stmt::Return(r) => {
                assert!(
                    r.syntax().descendants().find_map(IndexExpr::cast).is_some(),
                    "should find IndexExpr"
                );
            }
            other => panic!("expected Return, got {other:?}"),
        }
    }

    #[test]
    fn expr_array() {
        let root = parse("circuit f() : Field { return [1, 2, 3]; }");
        let file = SourceFile::cast(root).unwrap();
        let circuit = file.circuit_defs().next().unwrap();
        let block = circuit.body().unwrap();
        match block.stmts().next().unwrap() {
            Stmt::Return(r) => {
                assert!(
                    r.syntax().descendants().find_map(ArrayExpr::cast).is_some(),
                    "should find ArrayExpr"
                );
            }
            other => panic!("expected Return, got {other:?}"),
        }
    }

    #[test]
    fn expr_struct_literal() {
        let root = parse("circuit f() : Field { return Foo { x: 1 }; }");
        let file = SourceFile::cast(root).unwrap();
        let circuit = file.circuit_defs().next().unwrap();
        let block = circuit.body().unwrap();
        match block.stmts().next().unwrap() {
            Stmt::Return(r) => {
                let se = r
                    .syntax()
                    .descendants()
                    .find_map(StructExpr::cast)
                    .expect("should find StructExpr");
                assert_eq!(se.name().unwrap().text(), "Foo");
                assert_eq!(se.field_inits().count(), 1);
            }
            other => panic!("expected Return, got {other:?}"),
        }
    }

    #[test]
    fn expr_default() {
        let root = parse("circuit f() : Field { return default<Field>; }");
        let file = SourceFile::cast(root).unwrap();
        let circuit = file.circuit_defs().next().unwrap();
        let block = circuit.body().unwrap();
        match block.stmts().next().unwrap() {
            Stmt::Return(r) => {
                let de = r
                    .syntax()
                    .descendants()
                    .find_map(DefaultExpr::cast)
                    .expect("should find DefaultExpr");
                assert!(de.ty().is_some());
            }
            other => panic!("expected Return, got {other:?}"),
        }
    }

    #[test]
    fn expr_cast() {
        let root = parse("circuit f() : Field { return x as Field; }");
        let file = SourceFile::cast(root).unwrap();
        let circuit = file.circuit_defs().next().unwrap();
        let block = circuit.body().unwrap();
        match block.stmts().next().unwrap() {
            Stmt::Return(r) => {
                let ce = r
                    .syntax()
                    .descendants()
                    .find_map(CastExpr::cast)
                    .expect("should find CastExpr");
                assert!(ce.ty().is_some());
            }
            other => panic!("expected Return, got {other:?}"),
        }
    }

    #[test]
    fn expr_paren() {
        let root = parse("circuit f() : Field { return (x); }");
        let file = SourceFile::cast(root).unwrap();
        let circuit = file.circuit_defs().next().unwrap();
        let block = circuit.body().unwrap();
        match block.stmts().next().unwrap() {
            Stmt::Return(r) => {
                assert!(
                    r.syntax().descendants().find_map(ParenExpr::cast).is_some(),
                    "should find ParenExpr"
                );
            }
            other => panic!("expected Return, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // Param accessors
    // -----------------------------------------------------------------------

    #[test]
    fn param_accessors() {
        let root = parse("circuit f(x: Field) : Field { }");
        let file = SourceFile::cast(root).unwrap();
        let circuit = file.circuit_defs().next().unwrap();
        let param = circuit.params().next().expect("should have param");
        match param.pattern().expect("param should have pattern") {
            Pat::Ident(i) => assert_eq!(i.name().unwrap().text(), "x"),
            other => panic!("expected IdentPat, got {other:?}"),
        }
        assert!(matches!(
            param.ty().expect("param should have type"),
            Type::Field(_)
        ));
    }

    // -----------------------------------------------------------------------
    // AstNode trait: can_cast and failed cast
    // -----------------------------------------------------------------------

    #[test]
    fn cast_wrong_kind_returns_none() {
        let root = parse("pragma compact 0.15.0;");
        // Trying to cast the root (SOURCE_FILE) as a CircuitDef should fail
        assert!(CircuitDef::cast(root).is_none());
    }

    #[test]
    fn can_cast_checks() {
        assert!(CircuitDef::can_cast(
            compactp_syntax::SyntaxKind::CIRCUIT_DEF
        ));
        assert!(!CircuitDef::can_cast(
            compactp_syntax::SyntaxKind::STRUCT_DEF
        ));
        assert!(Expr::can_cast(compactp_syntax::SyntaxKind::BINARY_EXPR));
        assert!(!Expr::can_cast(compactp_syntax::SyntaxKind::BLOCK));
    }
}
