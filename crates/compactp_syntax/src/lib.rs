mod syntax_kind;
pub use syntax_kind::SyntaxKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CompactLanguage {}

impl rowan::Language for CompactLanguage {
    type Kind = SyntaxKind;

    fn kind_from_raw(raw: rowan::SyntaxKind) -> SyntaxKind {
        SyntaxKind::from(raw.0)
    }

    fn kind_to_raw(kind: SyntaxKind) -> rowan::SyntaxKind {
        rowan::SyntaxKind(kind.into())
    }
}

pub type SyntaxNode = rowan::SyntaxNode<CompactLanguage>;
pub type SyntaxToken = rowan::SyntaxToken<CompactLanguage>;

#[cfg(test)]
mod tests {
    use super::*;
    use rowan::Language;

    #[test]
    fn syntax_kind_is_u16() {
        let kind = SyntaxKind::WHITESPACE;
        let _raw: u16 = kind.into();
    }

    #[test]
    fn language_impl_exists() {
        let kind = CompactLanguage::kind_from_raw(rowan::SyntaxKind(0));
        assert_eq!(kind, SyntaxKind::WHITESPACE);
    }

    #[test]
    fn syntax_kind_is_trivia() {
        assert!(SyntaxKind::WHITESPACE.is_trivia());
        assert!(SyntaxKind::LINE_COMMENT.is_trivia());
        assert!(SyntaxKind::BLOCK_COMMENT.is_trivia());
        assert!(!SyntaxKind::IDENT.is_trivia());
        assert!(!SyntaxKind::CIRCUIT_KW.is_trivia());
    }

    #[test]
    fn roundtrip_through_rowan() {
        let kind = SyntaxKind::CIRCUIT_DEF;
        let raw = CompactLanguage::kind_to_raw(kind);
        let back = CompactLanguage::kind_from_raw(raw);
        assert_eq!(kind, back);
    }

    #[test]
    fn all_variants_roundtrip() {
        // Verify every variant survives u16 roundtrip
        let last = SyntaxKind::VERSION_PAREN_EXPR as u16;
        for raw in 0..=last {
            let kind = SyntaxKind::from(raw);
            let back: u16 = kind.into();
            assert_eq!(raw, back);
        }
    }
}
