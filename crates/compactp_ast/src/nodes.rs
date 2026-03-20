//! All AST node types except expressions (which live in [`crate::expr`]).
//!
//! Each struct is a zero-cost newtype over [`SyntaxNode`] with typed accessor
//! methods that navigate the CST children. The [`AstNode`] trait implementation
//! on each type enables casting from untyped `SyntaxNode` values.

use compactp_syntax::{SyntaxKind, SyntaxNode, SyntaxToken};

use crate::expr::Expr;
use crate::{AstNode, support};

// ---------------------------------------------------------------------------
// Macro to reduce boilerplate for simple AST node definitions
// ---------------------------------------------------------------------------

/// Define a simple AST node newtype wrapping `SyntaxNode`.
macro_rules! ast_node {
    (
        $(#[$meta:meta])*
        $name:ident => $kind:ident
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub struct $name(SyntaxNode);

        impl AstNode for $name {
            fn can_cast(kind: SyntaxKind) -> bool {
                kind == SyntaxKind::$kind
            }

            fn cast(node: SyntaxNode) -> Option<Self> {
                if Self::can_cast(node.kind()) {
                    Some(Self(node))
                } else {
                    None
                }
            }

            fn syntax(&self) -> &SyntaxNode {
                &self.0
            }
        }
    };
}

// ===========================================================================
// Top-level declarations
// ===========================================================================

ast_node! {
    /// The root node of a Compact source file.
    SourceFile => SOURCE_FILE
}

impl SourceFile {
    /// Iterate over all top-level `Pragma` declarations.
    pub fn pragmas(&self) -> impl Iterator<Item = Pragma> {
        support::children_nodes(&self.0)
    }

    /// Iterate over all top-level `Include` declarations.
    pub fn includes(&self) -> impl Iterator<Item = Include> {
        support::children_nodes(&self.0)
    }

    /// Iterate over all top-level `Import` declarations.
    pub fn imports(&self) -> impl Iterator<Item = Import> {
        support::children_nodes(&self.0)
    }

    /// Iterate over all top-level `CircuitDef` declarations.
    pub fn circuit_defs(&self) -> impl Iterator<Item = CircuitDef> {
        support::children_nodes(&self.0)
    }

    /// Iterate over all top-level `StructDef` declarations.
    pub fn struct_defs(&self) -> impl Iterator<Item = StructDef> {
        support::children_nodes(&self.0)
    }

    /// Iterate over all top-level `EnumDef` declarations.
    pub fn enum_defs(&self) -> impl Iterator<Item = EnumDef> {
        support::children_nodes(&self.0)
    }
}

ast_node! {
    /// `pragma id version-expr ;`
    Pragma => PRAGMA
}

impl Pragma {
    /// The pragma name identifier (e.g. `compact`).
    pub fn name(&self) -> Option<SyntaxToken> {
        support::child_token(&self.0, SyntaxKind::IDENT)
    }
}

ast_node! {
    /// `include "path" ;`
    Include => INCLUDE
}

impl Include {
    /// The string literal path.
    pub fn path(&self) -> Option<SyntaxToken> {
        support::child_token(&self.0, SyntaxKind::STRING_LIT)
    }
}

ast_node! {
    /// `import name gargs? prefix? ;`
    Import => IMPORT
}

impl Import {
    /// The imported module name (identifier form).
    pub fn name(&self) -> Option<SyntaxToken> {
        support::child_token(&self.0, SyntaxKind::IDENT)
    }

    /// The imported module path (string literal form).
    pub fn path(&self) -> Option<SyntaxToken> {
        support::child_token(&self.0, SyntaxKind::STRING_LIT)
    }

    /// Optional generic argument list.
    pub fn generic_args(&self) -> Option<GenericArgList> {
        support::child_node(&self.0)
    }

    /// Optional prefix declaration.
    pub fn prefix(&self) -> Option<PrefixDecl> {
        support::child_node(&self.0)
    }
}

ast_node! {
    /// An import specifier within a braced import list.
    ImportSpecifier => IMPORT_SPECIFIER
}

impl ImportSpecifier {
    /// The imported name.
    pub fn name(&self) -> Option<SyntaxToken> {
        support::child_token(&self.0, SyntaxKind::IDENT)
    }
}

ast_node! {
    /// `export { id, ... } ;?`
    ExportList => EXPORT_LIST
}

impl ExportList {
    /// All exported identifiers (the IDENT tokens within the braces).
    pub fn names(&self) -> impl Iterator<Item = SyntaxToken> + '_ {
        self.0
            .children_with_tokens()
            .filter_map(rowan::NodeOrToken::into_token)
            .filter(|t| t.kind() == SyntaxKind::IDENT)
    }
}

ast_node! {
    /// `export? module name gparams? { decls... }`
    ModuleDef => MODULE_DEF
}

impl ModuleDef {
    /// The `export` keyword if present.
    pub fn export_kw(&self) -> Option<SyntaxToken> {
        support::child_token(&self.0, SyntaxKind::EXPORT_KW)
    }

    /// The module name.
    pub fn name(&self) -> Option<SyntaxToken> {
        support::child_token(&self.0, SyntaxKind::IDENT)
    }

    /// Optional generic parameter list.
    pub fn generic_params(&self) -> Option<GenericParamList> {
        support::child_node(&self.0)
    }

    /// Whether this module is exported.
    pub fn is_exported(&self) -> bool {
        self.export_kw().is_some()
    }
}

ast_node! {
    /// `export? sealed? ledger name : type ;`
    LedgerDecl => LEDGER_DECL
}

impl LedgerDecl {
    /// The `export` keyword if present.
    pub fn export_kw(&self) -> Option<SyntaxToken> {
        support::child_token(&self.0, SyntaxKind::EXPORT_KW)
    }

    /// The `sealed` keyword if present.
    pub fn sealed_kw(&self) -> Option<SyntaxToken> {
        support::child_token(&self.0, SyntaxKind::SEALED_KW)
    }

    /// The ledger name.
    pub fn name(&self) -> Option<SyntaxToken> {
        support::child_token(&self.0, SyntaxKind::IDENT)
    }

    /// The type annotation for this ledger.
    pub fn ty(&self) -> Option<Type> {
        support::child_node(&self.0)
    }

    /// Whether this ledger is exported.
    pub fn is_exported(&self) -> bool {
        self.export_kw().is_some()
    }

    /// Whether this ledger is sealed.
    pub fn is_sealed(&self) -> bool {
        self.sealed_kw().is_some()
    }
}

ast_node! {
    /// `constructor ( params ) block`
    ConstructorDef => CONSTRUCTOR_DEF
}

impl ConstructorDef {
    /// The parameter nodes (PARAM children).
    pub fn params(&self) -> impl Iterator<Item = Param> {
        support::children_nodes(&self.0)
    }

    /// The body block.
    pub fn body(&self) -> Option<Block> {
        support::child_node(&self.0)
    }
}

ast_node! {
    /// `export? pure? circuit name gparams? ( params ) : type block`
    CircuitDef => CIRCUIT_DEF
}

impl CircuitDef {
    /// The `export` keyword if present.
    pub fn export_kw(&self) -> Option<SyntaxToken> {
        support::child_token(&self.0, SyntaxKind::EXPORT_KW)
    }

    /// The `pure` keyword if present.
    pub fn pure_kw(&self) -> Option<SyntaxToken> {
        support::child_token(&self.0, SyntaxKind::PURE_KW)
    }

    /// The circuit name.
    pub fn name(&self) -> Option<SyntaxToken> {
        support::child_token(&self.0, SyntaxKind::IDENT)
    }

    /// Optional generic parameter list.
    pub fn generic_params(&self) -> Option<GenericParamList> {
        support::child_node(&self.0)
    }

    /// The parameter nodes.
    pub fn params(&self) -> impl Iterator<Item = Param> {
        support::children_nodes(&self.0)
    }

    /// The return type. This is the first type child node found in the CST.
    /// Note: parameter types are nested inside PARAM nodes, so they won't be
    /// returned here.
    pub fn return_type(&self) -> Option<Type> {
        support::child_node(&self.0)
    }

    /// The body block.
    pub fn body(&self) -> Option<Block> {
        support::child_node(&self.0)
    }

    /// Whether this circuit is exported.
    pub fn is_exported(&self) -> bool {
        self.export_kw().is_some()
    }

    /// Whether this circuit is pure.
    pub fn is_pure(&self) -> bool {
        self.pure_kw().is_some()
    }
}

ast_node! {
    /// `export? circuit name gparams? ( args ) : type ;`
    CircuitDecl => CIRCUIT_DECL
}

impl CircuitDecl {
    /// The `export` keyword if present.
    pub fn export_kw(&self) -> Option<SyntaxToken> {
        support::child_token(&self.0, SyntaxKind::EXPORT_KW)
    }

    /// The circuit name.
    pub fn name(&self) -> Option<SyntaxToken> {
        support::child_token(&self.0, SyntaxKind::IDENT)
    }

    /// Optional generic parameter list.
    pub fn generic_params(&self) -> Option<GenericParamList> {
        support::child_node(&self.0)
    }

    /// The parameter nodes.
    pub fn params(&self) -> impl Iterator<Item = Param> {
        support::children_nodes(&self.0)
    }

    /// The return type.
    pub fn return_type(&self) -> Option<Type> {
        support::child_node(&self.0)
    }

    /// Whether this circuit declaration is exported.
    pub fn is_exported(&self) -> bool {
        self.export_kw().is_some()
    }
}

ast_node! {
    /// `export? witness name gparams? ( args ) : type ;`
    WitnessDecl => WITNESS_DECL
}

impl WitnessDecl {
    /// The `export` keyword if present.
    pub fn export_kw(&self) -> Option<SyntaxToken> {
        support::child_token(&self.0, SyntaxKind::EXPORT_KW)
    }

    /// The witness name.
    pub fn name(&self) -> Option<SyntaxToken> {
        support::child_token(&self.0, SyntaxKind::IDENT)
    }

    /// Optional generic parameter list.
    pub fn generic_params(&self) -> Option<GenericParamList> {
        support::child_node(&self.0)
    }

    /// The return type.
    pub fn return_type(&self) -> Option<Type> {
        support::child_node(&self.0)
    }

    /// Whether this witness is exported.
    pub fn is_exported(&self) -> bool {
        self.export_kw().is_some()
    }
}

ast_node! {
    /// `export? contract name { circuit-decls... } ;?`
    ContractDecl => CONTRACT_DECL
}

impl ContractDecl {
    /// The `export` keyword if present.
    pub fn export_kw(&self) -> Option<SyntaxToken> {
        support::child_token(&self.0, SyntaxKind::EXPORT_KW)
    }

    /// The contract name.
    pub fn name(&self) -> Option<SyntaxToken> {
        support::child_token(&self.0, SyntaxKind::IDENT)
    }

    /// The circuit declarations inside the contract.
    pub fn circuits(&self) -> impl Iterator<Item = ContractCircuit> {
        support::children_nodes(&self.0)
    }

    /// Whether this contract is exported.
    pub fn is_exported(&self) -> bool {
        self.export_kw().is_some()
    }
}

ast_node! {
    /// `pure? circuit name ( args ) : type ;` — circuit declaration inside a contract.
    ContractCircuit => CONTRACT_CIRCUIT
}

impl ContractCircuit {
    /// The `pure` keyword if present.
    pub fn pure_kw(&self) -> Option<SyntaxToken> {
        support::child_token(&self.0, SyntaxKind::PURE_KW)
    }

    /// The circuit name.
    pub fn name(&self) -> Option<SyntaxToken> {
        support::child_token(&self.0, SyntaxKind::IDENT)
    }

    /// The return type.
    pub fn return_type(&self) -> Option<Type> {
        support::child_node(&self.0)
    }

    /// Whether this contract circuit is pure.
    pub fn is_pure(&self) -> bool {
        self.pure_kw().is_some()
    }
}

ast_node! {
    /// `export? struct name gparams? { fields... } ;?`
    StructDef => STRUCT_DEF
}

impl StructDef {
    /// The `export` keyword if present.
    pub fn export_kw(&self) -> Option<SyntaxToken> {
        support::child_token(&self.0, SyntaxKind::EXPORT_KW)
    }

    /// The struct name.
    pub fn name(&self) -> Option<SyntaxToken> {
        support::child_token(&self.0, SyntaxKind::IDENT)
    }

    /// Optional generic parameter list.
    pub fn generic_params(&self) -> Option<GenericParamList> {
        support::child_node(&self.0)
    }

    /// The struct fields.
    pub fn fields(&self) -> impl Iterator<Item = StructField> {
        support::children_nodes(&self.0)
    }

    /// Whether this struct is exported.
    pub fn is_exported(&self) -> bool {
        self.export_kw().is_some()
    }
}

ast_node! {
    /// A field within a struct definition: `name : type`.
    StructField => STRUCT_FIELD
}

impl StructField {
    /// The field name.
    pub fn name(&self) -> Option<SyntaxToken> {
        support::child_token(&self.0, SyntaxKind::IDENT)
    }

    /// The field type.
    pub fn ty(&self) -> Option<Type> {
        support::child_node(&self.0)
    }
}

ast_node! {
    /// `export? enum name { variants... } ;?`
    EnumDef => ENUM_DEF
}

impl EnumDef {
    /// The `export` keyword if present.
    pub fn export_kw(&self) -> Option<SyntaxToken> {
        support::child_token(&self.0, SyntaxKind::EXPORT_KW)
    }

    /// The enum name.
    pub fn name(&self) -> Option<SyntaxToken> {
        support::child_token(&self.0, SyntaxKind::IDENT)
    }

    /// The enum variants.
    pub fn variants(&self) -> impl Iterator<Item = EnumVariant> {
        support::children_nodes(&self.0)
    }

    /// Whether this enum is exported.
    pub fn is_exported(&self) -> bool {
        self.export_kw().is_some()
    }
}

ast_node! {
    /// A single variant within an enum definition.
    EnumVariant => ENUM_VARIANT
}

impl EnumVariant {
    /// The variant name.
    pub fn name(&self) -> Option<SyntaxToken> {
        support::child_token(&self.0, SyntaxKind::IDENT)
    }
}

ast_node! {
    /// `prefix id` declaration within an import.
    PrefixDecl => PREFIX_DECL
}

impl PrefixDecl {
    /// The prefix identifier.
    pub fn name(&self) -> Option<SyntaxToken> {
        support::child_token(&self.0, SyntaxKind::IDENT)
    }
}

// ===========================================================================
// Type nodes
// ===========================================================================

/// Sum type for all type AST nodes.
///
/// Grammar positions that accept a type use this enum for exhaustive matching.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    /// A named type reference, possibly with generic arguments: `MyType<Field, 10>`.
    Ref(TypeRef),
    /// `Boolean`
    Boolean(BooleanType),
    /// `Field`
    Field(FieldType),
    /// `Uint<size>` or `Uint<lo..hi>`
    Uint(UintType),
    /// `Bytes<size>`
    Bytes(BytesType),
    /// `Opaque<"tag">`
    Opaque(OpaqueType),
    /// `Vector<size, type>`
    Vector(VectorType),
    /// `[type, ..., type]`
    Tuple(TupleType),
}

impl AstNode for Type {
    fn can_cast(kind: SyntaxKind) -> bool {
        matches!(
            kind,
            SyntaxKind::TYPE_REF
                | SyntaxKind::BOOLEAN_TYPE
                | SyntaxKind::FIELD_TYPE
                | SyntaxKind::UINT_TYPE
                | SyntaxKind::BYTES_TYPE
                | SyntaxKind::OPAQUE_TYPE
                | SyntaxKind::VECTOR_TYPE
                | SyntaxKind::TUPLE_TYPE
        )
    }

    fn cast(node: SyntaxNode) -> Option<Self> {
        match node.kind() {
            SyntaxKind::TYPE_REF => Some(Self::Ref(TypeRef(node))),
            SyntaxKind::BOOLEAN_TYPE => Some(Self::Boolean(BooleanType(node))),
            SyntaxKind::FIELD_TYPE => Some(Self::Field(FieldType(node))),
            SyntaxKind::UINT_TYPE => Some(Self::Uint(UintType(node))),
            SyntaxKind::BYTES_TYPE => Some(Self::Bytes(BytesType(node))),
            SyntaxKind::OPAQUE_TYPE => Some(Self::Opaque(OpaqueType(node))),
            SyntaxKind::VECTOR_TYPE => Some(Self::Vector(VectorType(node))),
            SyntaxKind::TUPLE_TYPE => Some(Self::Tuple(TupleType(node))),
            _ => None,
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        match self {
            Self::Ref(n) => &n.0,
            Self::Boolean(n) => &n.0,
            Self::Field(n) => &n.0,
            Self::Uint(n) => &n.0,
            Self::Bytes(n) => &n.0,
            Self::Opaque(n) => &n.0,
            Self::Vector(n) => &n.0,
            Self::Tuple(n) => &n.0,
        }
    }
}

ast_node! {
    /// A named type reference: `id` or `id<args>`.
    TypeRef => TYPE_REF
}

impl TypeRef {
    /// The type name.
    pub fn name(&self) -> Option<SyntaxToken> {
        support::child_token(&self.0, SyntaxKind::IDENT)
    }

    /// Optional generic argument list.
    pub fn generic_args(&self) -> Option<GenericArgList> {
        support::child_node(&self.0)
    }
}

ast_node! {
    /// `Boolean`
    BooleanType => BOOLEAN_TYPE
}

ast_node! {
    /// `Field`
    FieldType => FIELD_TYPE
}

ast_node! {
    /// `Uint<size>` or `Uint<lo..hi>`
    UintType => UINT_TYPE
}

impl UintType {
    /// The type size children (1 for `Uint<N>`, 2 for `Uint<lo..hi>`).
    pub fn sizes(&self) -> impl Iterator<Item = TypeSize> {
        support::children_nodes(&self.0)
    }
}

ast_node! {
    /// `Bytes<size>`
    BytesType => BYTES_TYPE
}

impl BytesType {
    /// The size parameter.
    pub fn size(&self) -> Option<TypeSize> {
        support::child_node(&self.0)
    }
}

ast_node! {
    /// `Opaque<"tag">`
    OpaqueType => OPAQUE_TYPE
}

impl OpaqueType {
    /// The string literal tag.
    pub fn tag(&self) -> Option<SyntaxToken> {
        support::child_token(&self.0, SyntaxKind::STRING_LIT)
    }
}

ast_node! {
    /// `Vector<size, type>`
    VectorType => VECTOR_TYPE
}

impl VectorType {
    /// The size parameter.
    pub fn size(&self) -> Option<TypeSize> {
        support::child_node(&self.0)
    }

    /// The element type.
    pub fn element_type(&self) -> Option<Type> {
        support::child_node(&self.0)
    }
}

ast_node! {
    /// `[type, ..., type]`
    TupleType => TUPLE_TYPE
}

impl TupleType {
    /// The element types.
    pub fn element_types(&self) -> impl Iterator<Item = Type> {
        support::children_nodes(&self.0)
    }
}

ast_node! {
    /// A generic argument list: `<arg, ..., arg>`.
    GenericArgList => GENERIC_ARG_LIST
}

impl GenericArgList {
    /// The generic arguments.
    pub fn args(&self) -> impl Iterator<Item = GenericArg> {
        support::children_nodes(&self.0)
    }
}

ast_node! {
    /// A single generic argument (either a type or a numeric literal).
    GenericArg => GENERIC_ARG
}

ast_node! {
    /// A generic parameter list: `<param, ..., param>`.
    GenericParamList => GENERIC_PARAM_LIST
}

impl GenericParamList {
    /// The generic parameters.
    pub fn params(&self) -> impl Iterator<Item = GenericParam> {
        support::children_nodes(&self.0)
    }
}

ast_node! {
    /// A single generic parameter: `T` or `#N`.
    GenericParam => GENERIC_PARAM
}

impl GenericParam {
    /// The parameter name.
    pub fn name(&self) -> Option<SyntaxToken> {
        support::child_token(&self.0, SyntaxKind::IDENT)
    }

    /// Whether this is a numeric type parameter (prefixed with `#`).
    pub fn is_numeric(&self) -> bool {
        support::child_token(&self.0, SyntaxKind::HASH).is_some()
    }
}

ast_node! {
    /// A type size: a numeric literal or identifier used in parameterized types.
    TypeSize => TYPE_SIZE
}

ast_node! {
    /// A typed parameter: `pattern : type`.
    Param => PARAM
}

impl Param {
    /// The pattern for this parameter.
    pub fn pattern(&self) -> Option<Pat> {
        support::child_node(&self.0)
    }

    /// The type annotation for this parameter.
    pub fn ty(&self) -> Option<Type> {
        support::child_node(&self.0)
    }
}

ast_node! {
    /// A parameter list: `( param, ... )`, used in lambda expressions.
    ParamList => PARAM_LIST
}

impl ParamList {
    /// The parameters in this list.
    pub fn params(&self) -> impl Iterator<Item = Param> {
        support::children_nodes(&self.0)
    }
}

// ===========================================================================
// Statement nodes
// ===========================================================================

/// Sum type for all statement AST nodes.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Stmt {
    /// `{ stmts... }`
    Block(Block),
    /// `lhs op= rhs ;`
    Assign(AssignStmt),
    /// `const pattern : type? = expr ;`
    Const(ConstStmt),
    /// Multi-const statement.
    MultiConst(MultiConstStmt),
    /// An expression statement.
    Expr(ExprStmt),
    /// `return expr? ;`
    Return(ReturnStmt),
    /// `if (cond) then else?`
    If(IfStmt),
    /// `for (const id of range) body`
    For(ForStmt),
    /// `assert ...`
    Assert(AssertStmt),
}

impl AstNode for Stmt {
    fn can_cast(kind: SyntaxKind) -> bool {
        matches!(
            kind,
            SyntaxKind::BLOCK
                | SyntaxKind::ASSIGN_STMT
                | SyntaxKind::CONST_STMT
                | SyntaxKind::MULTI_CONST_STMT
                | SyntaxKind::EXPR_STMT
                | SyntaxKind::RETURN_STMT
                | SyntaxKind::IF_STMT
                | SyntaxKind::FOR_STMT
                | SyntaxKind::ASSERT_STMT
        )
    }

    fn cast(node: SyntaxNode) -> Option<Self> {
        match node.kind() {
            SyntaxKind::BLOCK => Some(Self::Block(Block(node))),
            SyntaxKind::ASSIGN_STMT => Some(Self::Assign(AssignStmt(node))),
            SyntaxKind::CONST_STMT => Some(Self::Const(ConstStmt(node))),
            SyntaxKind::MULTI_CONST_STMT => Some(Self::MultiConst(MultiConstStmt(node))),
            SyntaxKind::EXPR_STMT => Some(Self::Expr(ExprStmt(node))),
            SyntaxKind::RETURN_STMT => Some(Self::Return(ReturnStmt(node))),
            SyntaxKind::IF_STMT => Some(Self::If(IfStmt(node))),
            SyntaxKind::FOR_STMT => Some(Self::For(ForStmt(node))),
            SyntaxKind::ASSERT_STMT => Some(Self::Assert(AssertStmt(node))),
            _ => None,
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        match self {
            Self::Block(n) => &n.0,
            Self::Assign(n) => &n.0,
            Self::Const(n) => &n.0,
            Self::MultiConst(n) => &n.0,
            Self::Expr(n) => &n.0,
            Self::Return(n) => &n.0,
            Self::If(n) => &n.0,
            Self::For(n) => &n.0,
            Self::Assert(n) => &n.0,
        }
    }
}

ast_node! {
    /// `{ stmts... }`
    Block => BLOCK
}

impl Block {
    /// Iterate over statements in this block.
    pub fn stmts(&self) -> impl Iterator<Item = Stmt> {
        support::children_nodes(&self.0)
    }
}

ast_node! {
    /// `lhs op= rhs ;`
    AssignStmt => ASSIGN_STMT
}

impl AssignStmt {
    /// The operator token (`=`, `+=`, or `-=`).
    pub fn op(&self) -> Option<SyntaxToken> {
        self.0
            .children_with_tokens()
            .filter_map(rowan::NodeOrToken::into_token)
            .find(|t| {
                matches!(
                    t.kind(),
                    SyntaxKind::EQ | SyntaxKind::PLUS_EQ | SyntaxKind::MINUS_EQ
                )
            })
    }
}

ast_node! {
    /// `const pattern : type? = expr ;`
    ConstStmt => CONST_STMT
}

impl ConstStmt {
    /// The pattern being bound.
    pub fn pattern(&self) -> Option<Pat> {
        support::child_node(&self.0)
    }

    /// The optional type annotation.
    pub fn ty(&self) -> Option<Type> {
        support::child_node(&self.0)
    }

    /// The initializer expression.
    pub fn value(&self) -> Option<Expr> {
        support::child_node(&self.0)
    }
}

ast_node! {
    /// Multi-const statement (reserved for future use).
    MultiConstStmt => MULTI_CONST_STMT
}

ast_node! {
    /// An expression used as a statement.
    ExprStmt => EXPR_STMT
}

ast_node! {
    /// `return expr? ;`
    ReturnStmt => RETURN_STMT
}

impl ReturnStmt {
    /// The returned expression, if any.
    pub fn value(&self) -> Option<Expr> {
        support::child_node(&self.0)
    }
}

ast_node! {
    /// `if (cond) then-stmt else-stmt?`
    IfStmt => IF_STMT
}

impl IfStmt {
    /// The then branch (first Block or Stmt child).
    pub fn then_branch(&self) -> Option<Block> {
        support::child_node(&self.0)
    }

    /// The else keyword, if present.
    pub fn else_kw(&self) -> Option<SyntaxToken> {
        support::child_token(&self.0, SyntaxKind::ELSE_KW)
    }
}

ast_node! {
    /// `for (const id of range-or-expr) body`
    ForStmt => FOR_STMT
}

impl ForStmt {
    /// The loop variable name.
    pub fn var_name(&self) -> Option<SyntaxToken> {
        support::child_token(&self.0, SyntaxKind::IDENT)
    }

    /// The loop body.
    pub fn body(&self) -> Option<Block> {
        support::child_node(&self.0)
    }
}

ast_node! {
    /// `assert(cond, "msg") ;` or `assert expr "msg" ;`
    AssertStmt => ASSERT_STMT
}

impl AssertStmt {
    /// The message string literal.
    pub fn message(&self) -> Option<SyntaxToken> {
        support::child_token(&self.0, SyntaxKind::STRING_LIT)
    }
}

// ===========================================================================
// Pattern nodes
// ===========================================================================

/// Sum type for all pattern AST nodes.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Pat {
    /// A simple identifier pattern.
    Ident(IdentPat),
    /// A tuple destructuring pattern: `[a, b, c]`.
    Tuple(TuplePat),
    /// A struct destructuring pattern: `{a, b: c}`.
    Struct(StructPat),
}

impl AstNode for Pat {
    fn can_cast(kind: SyntaxKind) -> bool {
        matches!(
            kind,
            SyntaxKind::IDENT_PAT | SyntaxKind::TUPLE_PAT | SyntaxKind::STRUCT_PAT
        )
    }

    fn cast(node: SyntaxNode) -> Option<Self> {
        match node.kind() {
            SyntaxKind::IDENT_PAT => Some(Self::Ident(IdentPat(node))),
            SyntaxKind::TUPLE_PAT => Some(Self::Tuple(TuplePat(node))),
            SyntaxKind::STRUCT_PAT => Some(Self::Struct(StructPat(node))),
            _ => None,
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        match self {
            Self::Ident(n) => &n.0,
            Self::Tuple(n) => &n.0,
            Self::Struct(n) => &n.0,
        }
    }
}

ast_node! {
    /// A simple identifier pattern.
    IdentPat => IDENT_PAT
}

impl IdentPat {
    /// The identifier name.
    pub fn name(&self) -> Option<SyntaxToken> {
        support::child_token(&self.0, SyntaxKind::IDENT)
    }
}

ast_node! {
    /// A tuple destructuring pattern: `[pat, ...]`.
    TuplePat => TUPLE_PAT
}

impl TuplePat {
    /// The tuple pattern elements.
    pub fn elements(&self) -> impl Iterator<Item = TuplePatElt> {
        support::children_nodes(&self.0)
    }
}

ast_node! {
    /// A single element in a tuple pattern.
    TuplePatElt => TUPLE_PAT_ELT
}

impl TuplePatElt {
    /// The inner pattern.
    pub fn pattern(&self) -> Option<Pat> {
        support::child_node(&self.0)
    }
}

ast_node! {
    /// A struct destructuring pattern: `{field, ...}`.
    StructPat => STRUCT_PAT
}

impl StructPat {
    /// The struct pattern fields.
    pub fn fields(&self) -> impl Iterator<Item = StructPatField> {
        support::children_nodes(&self.0)
    }
}

ast_node! {
    /// A single field in a struct pattern: `name` or `name: pat`.
    StructPatField => STRUCT_PAT_FIELD
}

impl StructPatField {
    /// The field name.
    pub fn name(&self) -> Option<SyntaxToken> {
        support::child_token(&self.0, SyntaxKind::IDENT)
    }

    /// The inner pattern, if this field has a `: pattern` binding.
    pub fn pattern(&self) -> Option<Pat> {
        support::child_node(&self.0)
    }
}
