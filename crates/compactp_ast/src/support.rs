//! Helper functions for typed AST node accessors.
//!
//! These functions traverse a `SyntaxNode`'s children to find tokens or child
//! nodes of specific kinds. They form the building blocks for all typed
//! accessor methods on AST wrapper types.

use compactp_syntax::{SyntaxKind, SyntaxNode, SyntaxToken};
use rowan::NodeOrToken;

use crate::AstNode;

/// Find the first direct child token with the given `SyntaxKind`.
pub fn child_token(parent: &SyntaxNode, kind: SyntaxKind) -> Option<SyntaxToken> {
    parent
        .children_with_tokens()
        .filter_map(NodeOrToken::into_token)
        .find(|t| t.kind() == kind)
}

/// Find the first direct child node that can be cast to the target AST type `N`.
pub fn child_node<N: AstNode>(parent: &SyntaxNode) -> Option<N> {
    parent.children().find_map(N::cast)
}

/// Iterate over all direct child nodes that can be cast to the target AST type `N`.
pub fn children_nodes<N: AstNode>(parent: &SyntaxNode) -> impl Iterator<Item = N> {
    parent.children().filter_map(N::cast)
}
