/// All syntax kinds for the Compact language.
///
/// This enum covers both tokens (leaf nodes) and nodes (interior nodes) in the CST.
/// It is `#[repr(u16)]` for rowan compatibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[allow(non_camel_case_types)]
#[repr(u16)]
pub enum SyntaxKind {
    // === Trivia ===
    WHITESPACE = 0,
    LINE_COMMENT,
    BLOCK_COMMENT,

    // === Literals ===
    INT_LIT,
    HEX_LIT,
    OCT_LIT,
    BIN_LIT,
    STRING_LIT,
    VERSION_LIT,

    // === Boolean keyword literals ===
    TRUE_KW,
    FALSE_KW,

    // === Keywords ===
    PRAGMA_KW,
    INCLUDE_KW,
    IMPORT_KW,
    FROM_KW,
    PREFIX_KW,
    EXPORT_KW,
    MODULE_KW,
    LEDGER_KW,
    CONSTRUCTOR_KW,
    CIRCUIT_KW,
    WITNESS_KW,
    CONTRACT_KW,
    STRUCT_KW,
    ENUM_KW,
    TYPE_KW,
    CONST_KW,
    RETURN_KW,
    IF_KW,
    ELSE_KW,
    FOR_KW,
    OF_KW,
    ASSERT_KW,
    AS_KW,
    PURE_KW,
    SEALED_KW,
    NEW_KW,
    MAP_KW,
    FOLD_KW,
    DEFAULT_KW,
    DISCLOSE_KW,
    PAD_KW,
    SLICE_KW,

    // === Builtin type keywords ===
    BOOLEAN_KW,
    FIELD_KW,
    UINT_KW,
    BYTES_KW,
    OPAQUE_KW,
    VECTOR_KW,

    // === Operators ===
    EQ,
    PLUS_EQ,
    MINUS_EQ,
    EQ_EQ,
    BANG_EQ,
    LT,
    LT_EQ,
    GT,
    GT_EQ,
    AMP_AMP,
    PIPE_PIPE,
    PLUS,
    MINUS,
    STAR,
    SLASH,
    BANG,
    QUESTION,
    FAT_ARROW,
    DOT,
    DOT_DOT,
    DOT_DOT_DOT,

    // === Delimiters ===
    L_PAREN,
    R_PAREN,
    L_BRACE,
    R_BRACE,
    L_BRACKET,
    R_BRACKET,
    COMMA,
    SEMICOLON,
    COLON,
    HASH,

    // === Special tokens ===
    IDENT,
    ERROR,
    EOF,

    // === Node kinds: Top-level ===
    SOURCE_FILE,
    PRAGMA,
    INCLUDE,
    IMPORT,
    IMPORT_SPECIFIER,
    IMPORT_SPECIFIER_LIST,
    EXPORT_LIST,
    MODULE_DEF,
    LEDGER_DECL,
    CONSTRUCTOR_DEF,
    CIRCUIT_DEF,
    CIRCUIT_DECL,
    WITNESS_DECL,
    CONTRACT_DECL,
    CONTRACT_CIRCUIT,
    STRUCT_DEF,
    STRUCT_FIELD,
    ENUM_DEF,
    ENUM_VARIANT,
    TYPE_DECL,

    // === Node kinds: Types ===
    TYPE_REF,
    BOOLEAN_TYPE,
    FIELD_TYPE,
    UINT_TYPE,
    BYTES_TYPE,
    OPAQUE_TYPE,
    VECTOR_TYPE,
    TUPLE_TYPE,
    RECORD_TYPE,
    GENERIC_ARG_LIST,
    GENERIC_ARG,
    GENERIC_PARAM_LIST,
    GENERIC_PARAM,
    TYPE_SIZE,

    // === Node kinds: Patterns ===
    IDENT_PAT,
    TUPLE_PAT,
    TUPLE_PAT_ELT,
    STRUCT_PAT,
    STRUCT_PAT_FIELD,
    TYPED_PAT,

    // === Node kinds: Statements ===
    BLOCK,
    ASSIGN_STMT,
    EXPR_STMT,
    RETURN_STMT,
    IF_STMT,
    FOR_STMT,
    ASSERT_STMT,
    CONST_STMT,
    MULTI_CONST_STMT,

    // === Node kinds: Expressions ===
    LITERAL_EXPR,
    NAME_EXPR,
    TERNARY_EXPR,
    BINARY_EXPR,
    UNARY_EXPR,
    CAST_EXPR,
    CALL_EXPR,
    MEMBER_EXPR,
    INDEX_EXPR,
    PAREN_EXPR,
    EXPR_SEQ,
    ARRAY_EXPR,
    BYTES_EXPR,
    SPREAD_EXPR,
    STRUCT_EXPR,
    STRUCT_FIELD_INIT,
    STRUCT_UPDATE,
    DEFAULT_EXPR,
    MAP_EXPR,
    FOLD_EXPR,
    DISCLOSE_EXPR,
    PAD_EXPR,
    SLICE_EXPR,
    LAMBDA_EXPR,
    PARAM_LIST,
    PARAM,
    RANGE_EXPR,
    PREFIX_DECL,

    // === Node kinds: Version expressions ===
    VERSION_EXPR,
    VERSION_AND_EXPR,
    VERSION_OR_EXPR,
    VERSION_UNARY_EXPR,
    VERSION_PAREN_EXPR,
}

impl SyntaxKind {
    /// Returns true for whitespace and comment tokens.
    pub fn is_trivia(self) -> bool {
        matches!(
            self,
            Self::WHITESPACE | Self::LINE_COMMENT | Self::BLOCK_COMMENT
        )
    }

    /// Returns true for all keyword variants (including builtin type keywords and boolean literals).
    pub fn is_keyword(self) -> bool {
        matches!(
            self,
            Self::TRUE_KW
                | Self::FALSE_KW
                | Self::PRAGMA_KW
                | Self::INCLUDE_KW
                | Self::IMPORT_KW
                | Self::FROM_KW
                | Self::PREFIX_KW
                | Self::EXPORT_KW
                | Self::MODULE_KW
                | Self::LEDGER_KW
                | Self::CONSTRUCTOR_KW
                | Self::CIRCUIT_KW
                | Self::WITNESS_KW
                | Self::CONTRACT_KW
                | Self::STRUCT_KW
                | Self::ENUM_KW
                | Self::TYPE_KW
                | Self::CONST_KW
                | Self::RETURN_KW
                | Self::IF_KW
                | Self::ELSE_KW
                | Self::FOR_KW
                | Self::OF_KW
                | Self::ASSERT_KW
                | Self::AS_KW
                | Self::PURE_KW
                | Self::SEALED_KW
                | Self::NEW_KW
                | Self::MAP_KW
                | Self::FOLD_KW
                | Self::DEFAULT_KW
                | Self::DISCLOSE_KW
                | Self::PAD_KW
                | Self::SLICE_KW
                | Self::BOOLEAN_KW
                | Self::FIELD_KW
                | Self::UINT_KW
                | Self::BYTES_KW
                | Self::OPAQUE_KW
                | Self::VECTOR_KW
        )
    }
}

impl From<SyntaxKind> for u16 {
    fn from(kind: SyntaxKind) -> u16 {
        kind as u16
    }
}

impl From<u16> for SyntaxKind {
    fn from(raw: u16) -> SyntaxKind {
        // Safety: we validate the raw value is within range
        assert!(
            raw <= SyntaxKind::VERSION_PAREN_EXPR as u16,
            "invalid SyntaxKind raw value: {raw}, max is {}",
            SyntaxKind::VERSION_PAREN_EXPR as u16
        );
        // SAFETY: SyntaxKind is repr(u16) and we've validated the range
        unsafe { std::mem::transmute(raw) }
    }
}

impl From<SyntaxKind> for rowan::SyntaxKind {
    fn from(kind: SyntaxKind) -> rowan::SyntaxKind {
        rowan::SyntaxKind(kind.into())
    }
}
