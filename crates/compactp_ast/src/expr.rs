//! Expression AST types and the [`Expr`] sum-type enum.
//!
//! All expression node types are zero-cost newtypes over [`SyntaxNode`] with
//! typed accessor methods. The [`Expr`] enum provides exhaustive matching over
//! all expression variants.

use compactp_syntax::{SyntaxKind, SyntaxNode, SyntaxToken};

use crate::nodes::{Block, GenericArgList, ParamList, Type};
use crate::{AstNode, support};

// ---------------------------------------------------------------------------
// Macro to reduce boilerplate
// ---------------------------------------------------------------------------

macro_rules! ast_node {
    (
        $(#[$meta:meta])*
        $name:ident => $kind:ident
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub struct $name(pub(crate) SyntaxNode);

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
// Expr sum-type enum
// ===========================================================================

/// Sum type for all expression AST nodes.
///
/// Grammar positions that accept an expression use this enum for exhaustive
/// matching. Each variant wraps a specific expression node type.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Expr {
    /// A literal value (number, string, boolean)
    Literal(LiteralExpr),
    /// A name reference (identifier)
    Name(NameExpr),
    /// `cond ? then : else`
    Ternary(TernaryExpr),
    /// `lhs op rhs`
    Binary(BinaryExpr),
    /// `!expr`
    Unary(UnaryExpr),
    /// `expr as type`
    Cast(CastExpr),
    /// `name(args)` or `expr.method(args)`
    Call(CallExpr),
    /// `expr.field`
    Member(MemberExpr),
    /// `expr[index]`
    Index(IndexExpr),
    /// `[elem, ...]`
    Array(ArrayExpr),
    /// `Bytes[elem, ...]`
    Bytes(BytesExpr),
    /// `...expr`
    Spread(SpreadExpr),
    /// `Name { fields... }`
    Struct(StructExpr),
    /// `default<type>`
    Default(DefaultExpr),
    /// `map(fn, exprs...)`
    Map(MapExpr),
    /// `fold(fn, init, exprs...)`
    Fold(FoldExpr),
    /// `disclose(expr)`
    Disclose(DiscloseExpr),
    /// `pad(n, str)`
    Pad(PadExpr),
    /// `slice<n>(expr, expr)`
    Slice(SliceExpr),
    /// `(params) : type? => body`
    Lambda(LambdaExpr),
    /// `(expr)`
    Paren(ParenExpr),
}

impl AstNode for Expr {
    fn can_cast(kind: SyntaxKind) -> bool {
        matches!(
            kind,
            SyntaxKind::LITERAL_EXPR
                | SyntaxKind::NAME_EXPR
                | SyntaxKind::TERNARY_EXPR
                | SyntaxKind::BINARY_EXPR
                | SyntaxKind::UNARY_EXPR
                | SyntaxKind::CAST_EXPR
                | SyntaxKind::CALL_EXPR
                | SyntaxKind::MEMBER_EXPR
                | SyntaxKind::INDEX_EXPR
                | SyntaxKind::ARRAY_EXPR
                | SyntaxKind::BYTES_EXPR
                | SyntaxKind::SPREAD_EXPR
                | SyntaxKind::STRUCT_EXPR
                | SyntaxKind::DEFAULT_EXPR
                | SyntaxKind::MAP_EXPR
                | SyntaxKind::FOLD_EXPR
                | SyntaxKind::DISCLOSE_EXPR
                | SyntaxKind::PAD_EXPR
                | SyntaxKind::SLICE_EXPR
                | SyntaxKind::LAMBDA_EXPR
                | SyntaxKind::PAREN_EXPR
        )
    }

    fn cast(node: SyntaxNode) -> Option<Self> {
        match node.kind() {
            SyntaxKind::LITERAL_EXPR => Some(Self::Literal(LiteralExpr(node))),
            SyntaxKind::NAME_EXPR => Some(Self::Name(NameExpr(node))),
            SyntaxKind::TERNARY_EXPR => Some(Self::Ternary(TernaryExpr(node))),
            SyntaxKind::BINARY_EXPR => Some(Self::Binary(BinaryExpr(node))),
            SyntaxKind::UNARY_EXPR => Some(Self::Unary(UnaryExpr(node))),
            SyntaxKind::CAST_EXPR => Some(Self::Cast(CastExpr(node))),
            SyntaxKind::CALL_EXPR => Some(Self::Call(CallExpr(node))),
            SyntaxKind::MEMBER_EXPR => Some(Self::Member(MemberExpr(node))),
            SyntaxKind::INDEX_EXPR => Some(Self::Index(IndexExpr(node))),
            SyntaxKind::ARRAY_EXPR => Some(Self::Array(ArrayExpr(node))),
            SyntaxKind::BYTES_EXPR => Some(Self::Bytes(BytesExpr(node))),
            SyntaxKind::SPREAD_EXPR => Some(Self::Spread(SpreadExpr(node))),
            SyntaxKind::STRUCT_EXPR => Some(Self::Struct(StructExpr(node))),
            SyntaxKind::DEFAULT_EXPR => Some(Self::Default(DefaultExpr(node))),
            SyntaxKind::MAP_EXPR => Some(Self::Map(MapExpr(node))),
            SyntaxKind::FOLD_EXPR => Some(Self::Fold(FoldExpr(node))),
            SyntaxKind::DISCLOSE_EXPR => Some(Self::Disclose(DiscloseExpr(node))),
            SyntaxKind::PAD_EXPR => Some(Self::Pad(PadExpr(node))),
            SyntaxKind::SLICE_EXPR => Some(Self::Slice(SliceExpr(node))),
            SyntaxKind::LAMBDA_EXPR => Some(Self::Lambda(LambdaExpr(node))),
            SyntaxKind::PAREN_EXPR => Some(Self::Paren(ParenExpr(node))),
            _ => None,
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        match self {
            Self::Literal(n) => &n.0,
            Self::Name(n) => &n.0,
            Self::Ternary(n) => &n.0,
            Self::Binary(n) => &n.0,
            Self::Unary(n) => &n.0,
            Self::Cast(n) => &n.0,
            Self::Call(n) => &n.0,
            Self::Member(n) => &n.0,
            Self::Index(n) => &n.0,
            Self::Array(n) => &n.0,
            Self::Bytes(n) => &n.0,
            Self::Spread(n) => &n.0,
            Self::Struct(n) => &n.0,
            Self::Default(n) => &n.0,
            Self::Map(n) => &n.0,
            Self::Fold(n) => &n.0,
            Self::Disclose(n) => &n.0,
            Self::Pad(n) => &n.0,
            Self::Slice(n) => &n.0,
            Self::Lambda(n) => &n.0,
            Self::Paren(n) => &n.0,
        }
    }
}

// ===========================================================================
// Expression node types
// ===========================================================================

ast_node! {
    /// A literal value: number, string, or boolean.
    LiteralExpr => LITERAL_EXPR
}

ast_node! {
    /// A name reference (identifier expression).
    NameExpr => NAME_EXPR
}

impl NameExpr {
    /// The identifier token.
    pub fn ident(&self) -> Option<SyntaxToken> {
        support::child_token(&self.0, SyntaxKind::IDENT)
    }
}

ast_node! {
    /// Ternary conditional: `cond ? then : else`.
    TernaryExpr => TERNARY_EXPR
}

impl TernaryExpr {
    /// The `?` token.
    pub fn question(&self) -> Option<SyntaxToken> {
        support::child_token(&self.0, SyntaxKind::QUESTION)
    }
}

ast_node! {
    /// Binary expression: `lhs op rhs`.
    BinaryExpr => BINARY_EXPR
}

impl BinaryExpr {
    /// The operator token.
    pub fn op(&self) -> Option<SyntaxToken> {
        self.0
            .children_with_tokens()
            .filter_map(rowan::NodeOrToken::into_token)
            .find(|t| {
                matches!(
                    t.kind(),
                    SyntaxKind::PLUS
                        | SyntaxKind::MINUS
                        | SyntaxKind::STAR
                        | SyntaxKind::SLASH
                        | SyntaxKind::EQ_EQ
                        | SyntaxKind::BANG_EQ
                        | SyntaxKind::LT
                        | SyntaxKind::LT_EQ
                        | SyntaxKind::GT
                        | SyntaxKind::GT_EQ
                        | SyntaxKind::AMP_AMP
                        | SyntaxKind::PIPE_PIPE
                )
            })
    }
}

ast_node! {
    /// Unary prefix expression: `!expr`.
    UnaryExpr => UNARY_EXPR
}

impl UnaryExpr {
    /// The operator token (`!`).
    pub fn op(&self) -> Option<SyntaxToken> {
        support::child_token(&self.0, SyntaxKind::BANG)
    }
}

ast_node! {
    /// Cast expression: `expr as type`.
    CastExpr => CAST_EXPR
}

impl CastExpr {
    /// The target type.
    pub fn ty(&self) -> Option<Type> {
        support::child_node(&self.0)
    }
}

ast_node! {
    /// Function call expression: `name(args)` or `expr.method(args)`.
    CallExpr => CALL_EXPR
}

impl CallExpr {
    /// The function name (for direct calls like `foo(args)`).
    /// For method calls (`expr.method(args)`), the IDENT is the method name.
    pub fn name(&self) -> Option<SyntaxToken> {
        support::child_token(&self.0, SyntaxKind::IDENT)
    }

    /// Optional generic argument list.
    pub fn generic_args(&self) -> Option<GenericArgList> {
        support::child_node(&self.0)
    }
}

ast_node! {
    /// Member access expression: `expr.field`.
    MemberExpr => MEMBER_EXPR
}

impl MemberExpr {
    /// The field name.
    pub fn field(&self) -> Option<SyntaxToken> {
        support::child_token(&self.0, SyntaxKind::IDENT)
    }
}

ast_node! {
    /// Index expression: `expr[index]`.
    IndexExpr => INDEX_EXPR
}

ast_node! {
    /// Array literal: `[elem, ...]`.
    ArrayExpr => ARRAY_EXPR
}

ast_node! {
    /// Bytes literal: `Bytes[elem, ...]`.
    BytesExpr => BYTES_EXPR
}

ast_node! {
    /// Spread expression: `...expr`.
    SpreadExpr => SPREAD_EXPR
}

ast_node! {
    /// Struct literal: `Name { fields... }`.
    StructExpr => STRUCT_EXPR
}

impl StructExpr {
    /// The struct name.
    pub fn name(&self) -> Option<SyntaxToken> {
        support::child_token(&self.0, SyntaxKind::IDENT)
    }

    /// The field initializers.
    pub fn field_inits(&self) -> impl Iterator<Item = StructFieldInit> {
        support::children_nodes(&self.0)
    }

    /// The struct update (spread) expression, if any.
    pub fn update(&self) -> Option<StructUpdate> {
        support::child_node(&self.0)
    }
}

ast_node! {
    /// A struct field initializer: `name: expr`.
    StructFieldInit => STRUCT_FIELD_INIT
}

impl StructFieldInit {
    /// The field name.
    pub fn name(&self) -> Option<SyntaxToken> {
        support::child_token(&self.0, SyntaxKind::IDENT)
    }
}

ast_node! {
    /// A struct update (spread) expression: `...expr`.
    StructUpdate => STRUCT_UPDATE
}

ast_node! {
    /// Default value expression: `default<type>`.
    DefaultExpr => DEFAULT_EXPR
}

impl DefaultExpr {
    /// The type argument.
    pub fn ty(&self) -> Option<Type> {
        support::child_node(&self.0)
    }
}

ast_node! {
    /// Map expression: `map(fn, exprs...)`.
    MapExpr => MAP_EXPR
}

ast_node! {
    /// Fold expression: `fold(fn, init, exprs...)`.
    FoldExpr => FOLD_EXPR
}

ast_node! {
    /// Disclose expression: `disclose(expr)`.
    DiscloseExpr => DISCLOSE_EXPR
}

ast_node! {
    /// Pad expression: `pad(n, str)`.
    PadExpr => PAD_EXPR
}

ast_node! {
    /// Slice expression: `slice<n>(expr, expr)`.
    SliceExpr => SLICE_EXPR
}

impl SliceExpr {
    /// Optional generic argument list.
    pub fn generic_args(&self) -> Option<GenericArgList> {
        support::child_node(&self.0)
    }
}

ast_node! {
    /// Lambda expression: `(params) : type? => body`.
    LambdaExpr => LAMBDA_EXPR
}

impl LambdaExpr {
    /// The parameter list.
    pub fn param_list(&self) -> Option<ParamList> {
        support::child_node(&self.0)
    }

    /// The optional return type annotation.
    pub fn return_type(&self) -> Option<Type> {
        support::child_node(&self.0)
    }

    /// The lambda body block, if the body is a block.
    pub fn body_block(&self) -> Option<Block> {
        support::child_node(&self.0)
    }
}

ast_node! {
    /// Parenthesized expression: `(expr)`.
    ParenExpr => PAREN_EXPR
}
