pub mod expr;
pub mod nodes;
mod support;

use compactp_syntax::{SyntaxKind, SyntaxNode};

/// Trait for typed AST nodes that wrap untyped CST nodes.
pub trait AstNode: Sized {
    fn can_cast(kind: SyntaxKind) -> bool;
    fn cast(node: SyntaxNode) -> Option<Self>;
    fn syntax(&self) -> &SyntaxNode;
}

// Re-export the most commonly used types
pub use expr::Expr;
pub use nodes::*;

#[cfg(test)]
mod tests {
    use super::*;
    use compactp_syntax::SyntaxNode as SN;

    fn parse_first<T: AstNode>(source: &str) -> Option<T> {
        let result = compactp_parser::parse(source);
        let root = SN::new_root(result.green);
        root.children().find_map(T::cast)
    }

    #[test]
    fn circuit_def_accessors() {
        let circuit: CircuitDef =
            parse_first("export pure circuit foo(x: Field): Field { return x; }")
                .expect("should have CircuitDef");
        assert!(circuit.is_exported());
        assert!(circuit.is_pure());
        assert_eq!(circuit.name().unwrap().text(), "foo");
        assert!(circuit.body().is_some());
    }

    #[test]
    fn ledger_decl_accessors() {
        let ledger: LedgerDecl =
            parse_first("export ledger value: Field;").expect("should have LedgerDecl");
        assert!(ledger.export_kw().is_some());
        assert_eq!(ledger.name().unwrap().text(), "value");
    }

    #[test]
    fn struct_def_accessors() {
        let s: StructDef = parse_first("export struct Point { x: Field; y: Field; }")
            .expect("should have StructDef");
        assert!(s.export_kw().is_some());
        assert_eq!(s.name().unwrap().text(), "Point");
    }

    #[test]
    fn pragma_accessors() {
        let pragma: Pragma =
            parse_first("pragma language_version >= 0.22.0;").expect("should have Pragma");
        assert_eq!(pragma.name().unwrap().text(), "language_version");
    }

    #[test]
    fn source_file_items() {
        let source = "ledger x: Field;\nledger y: Field;";
        let result = compactp_parser::parse(source);
        let root = SN::new_root(result.green);
        let sf = SourceFile::cast(root).expect("root should be SourceFile");
        let items: Vec<_> = sf.syntax().children().collect();
        assert_eq!(items.len(), 2);
    }
}
