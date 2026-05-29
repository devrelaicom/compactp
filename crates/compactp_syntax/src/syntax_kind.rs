/// All syntax kinds for the Compact language.
///
/// This enum covers both tokens (leaf nodes) and nodes (interior nodes) in the CST.
/// It is `#[repr(u16)]` for rowan compatibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[allow(non_camel_case_types)]
#[repr(u16)]
pub enum SyntaxKind {
    // === Trivia ===
    /// Whitespace token (spaces, tabs, newlines).
    WHITESPACE = 0,
    /// `// ...` single-line comment token.
    LINE_COMMENT,
    /// `/* ... */` block comment token.
    BLOCK_COMMENT,

    // === Literals ===
    /// Decimal integer literal token (e.g. `42`).
    INT_LIT,
    /// Hexadecimal integer literal token (e.g. `0xff`).
    HEX_LIT,
    /// Octal integer literal token (e.g. `0o755`).
    OCT_LIT,
    /// Binary integer literal token (e.g. `0b1010`).
    BIN_LIT,
    /// String literal token (e.g. `"hello"`).
    STRING_LIT,
    /// Version literal token used in `pragma language_version` (e.g. `0.16`).
    VERSION_LIT,

    // === Boolean keyword literals ===
    /// `true` keyword token (boolean literal).
    TRUE_KW,
    /// `false` keyword token (boolean literal).
    FALSE_KW,

    // === Keywords ===
    /// `pragma` keyword token.
    PRAGMA_KW,
    /// `include` keyword token.
    INCLUDE_KW,
    /// `import` keyword token.
    IMPORT_KW,
    /// `from` keyword token (used in import statements).
    FROM_KW,
    /// `prefix` keyword token.
    PREFIX_KW,
    /// `export` keyword token.
    EXPORT_KW,
    /// `module` keyword token.
    MODULE_KW,
    /// `ledger` keyword token.
    LEDGER_KW,
    /// `constructor` keyword token.
    CONSTRUCTOR_KW,
    /// `circuit` keyword token.
    CIRCUIT_KW,
    /// `witness` keyword token.
    WITNESS_KW,
    /// `contract` keyword token.
    CONTRACT_KW,
    /// `struct` keyword token.
    STRUCT_KW,
    /// `enum` keyword token.
    ENUM_KW,
    /// `type` keyword token (type alias declaration).
    TYPE_KW,
    /// `const` keyword token.
    CONST_KW,
    /// `return` keyword token.
    RETURN_KW,
    /// `if` keyword token.
    IF_KW,
    /// `else` keyword token.
    ELSE_KW,
    /// `for` keyword token.
    FOR_KW,
    /// `of` keyword token (used in `for ... of` loops).
    OF_KW,
    /// `assert` keyword token.
    ASSERT_KW,
    /// `as` keyword token (cast expressions).
    AS_KW,
    /// `pure` keyword token (function/circuit modifier).
    PURE_KW,
    /// `sealed` keyword token (ledger modifier).
    SEALED_KW,
    /// `new` keyword token.
    NEW_KW,
    /// `map` keyword token (map combinator).
    MAP_KW,
    /// `fold` keyword token (fold combinator).
    FOLD_KW,
    /// `default` keyword token (default-value expression).
    DEFAULT_KW,
    /// `disclose` keyword token.
    DISCLOSE_KW,
    /// `pad` keyword token (byte-padding builtin).
    PAD_KW,
    /// `slice` keyword token (slice builtin).
    SLICE_KW,

    // === Builtin type keywords ===
    /// `Boolean` builtin type keyword token.
    BOOLEAN_KW,
    /// `Field` builtin type keyword token.
    FIELD_KW,
    /// `Uint` builtin type keyword token.
    UINT_KW,
    /// `Bytes` builtin type keyword token.
    BYTES_KW,
    /// `Opaque` builtin type keyword token.
    OPAQUE_KW,
    /// `Vector` builtin type keyword token.
    VECTOR_KW,
    /// `Unsigned` builtin type keyword token (paired with `Integer`).
    UNSIGNED_KW,
    /// `Integer` builtin type keyword token.
    INTEGER_KW,

    // === Operators ===
    /// `=` assignment operator token.
    EQ,
    /// `+=` compound-assignment operator token.
    PLUS_EQ,
    /// `-=` compound-assignment operator token.
    MINUS_EQ,
    /// `==` equality operator token.
    EQ_EQ,
    /// `!=` inequality operator token.
    BANG_EQ,
    /// `<` less-than operator token.
    LT,
    /// `<=` less-than-or-equal operator token.
    LT_EQ,
    /// `>` greater-than operator token.
    GT,
    /// `>=` greater-than-or-equal operator token.
    GT_EQ,
    /// `&&` logical-and operator token.
    AMP_AMP,
    /// `||` logical-or operator token.
    PIPE_PIPE,
    /// `+` plus operator token (addition/unary plus).
    PLUS,
    /// `-` minus operator token (subtraction/unary negation).
    MINUS,
    /// `*` star operator token (multiplication).
    STAR,
    /// `/` slash operator token (division).
    SLASH,
    /// `!` bang operator token (logical not).
    BANG,
    /// `?` question-mark operator token (ternary).
    QUESTION,
    /// `=>` fat-arrow token (lambda body).
    FAT_ARROW,
    /// `.` dot token (member access).
    DOT,
    /// `..` dot-dot token (range).
    DOT_DOT,
    /// `...` dot-dot-dot token (spread).
    DOT_DOT_DOT,

    // === Delimiters ===
    /// `(` left parenthesis delimiter.
    L_PAREN,
    /// `)` right parenthesis delimiter.
    R_PAREN,
    /// `{` left brace delimiter.
    L_BRACE,
    /// `}` right brace delimiter.
    R_BRACE,
    /// `[` left bracket delimiter.
    L_BRACKET,
    /// `]` right bracket delimiter.
    R_BRACKET,
    /// `,` comma delimiter.
    COMMA,
    /// `;` semicolon delimiter.
    SEMICOLON,
    /// `:` colon delimiter (type annotations, struct fields).
    COLON,
    /// `#` hash delimiter (attribute markers, version pragmas).
    HASH,

    // === Special tokens ===
    /// Identifier token (user-defined names).
    IDENT,
    /// Lexer/parser error token used to preserve malformed input.
    ERROR,
    /// End-of-file marker token.
    EOF,

    // === Node kinds: Top-level ===
    /// Root node containing all top-level items of a single source file.
    SOURCE_FILE,
    /// `pragma` directive node (e.g. `pragma language_version 0.16;`).
    PRAGMA,
    /// `include` directive node.
    INCLUDE,
    /// `import` declaration node.
    IMPORT,
    /// Single import specifier (`name` or `name as alias`) within an import list.
    IMPORT_SPECIFIER,
    /// Brace-delimited list of import specifiers.
    IMPORT_SPECIFIER_LIST,
    /// Brace-delimited list of names following the `export` keyword.
    EXPORT_LIST,
    /// `module` definition node.
    MODULE_DEF,
    /// `ledger` declaration node.
    LEDGER_DECL,
    /// `constructor` definition node.
    CONSTRUCTOR_DEF,
    /// `circuit` definition node (full definition with body).
    CIRCUIT_DEF,
    /// `circuit` declaration node (signature only, no body).
    CIRCUIT_DECL,
    /// `witness` declaration node.
    WITNESS_DECL,
    /// `contract` declaration node (imported contract reference).
    CONTRACT_DECL,
    /// Single circuit entry inside a `contract` declaration.
    CONTRACT_CIRCUIT,
    /// `struct` definition node.
    STRUCT_DEF,
    /// Single field declaration inside a struct.
    STRUCT_FIELD,
    /// `enum` definition node.
    ENUM_DEF,
    /// Single variant declaration inside an enum.
    ENUM_VARIANT,
    /// `type` alias declaration node.
    TYPE_DECL,

    // === Node kinds: Types ===
    /// Named type reference (user-defined or imported type).
    TYPE_REF,
    /// `Boolean` builtin type node.
    BOOLEAN_TYPE,
    /// `Field` builtin type node.
    FIELD_TYPE,
    /// `Uint<...>` builtin type node.
    UINT_TYPE,
    /// `Unsigned Integer<...>` builtin type node.
    UNSIGNED_INTEGER_TYPE,
    /// `Bytes<...>` builtin type node.
    BYTES_TYPE,
    /// `Opaque<...>` builtin type node.
    OPAQUE_TYPE,
    /// `Vector<...>` builtin type node.
    VECTOR_TYPE,
    /// Tuple type node (parenthesized comma-separated types).
    TUPLE_TYPE,
    /// Record/struct-literal type node.
    RECORD_TYPE,
    /// Angle-bracket-delimited list of generic arguments at a type use site.
    GENERIC_ARG_LIST,
    /// Single generic argument (type or const) within a [`GENERIC_ARG_LIST`](Self::GENERIC_ARG_LIST).
    GENERIC_ARG,
    /// Angle-bracket-delimited list of generic parameters at a declaration site.
    GENERIC_PARAM_LIST,
    /// Single generic parameter within a [`GENERIC_PARAM_LIST`](Self::GENERIC_PARAM_LIST).
    GENERIC_PARAM,
    /// Size/length expression embedded in a sized builtin type (e.g. `Bytes<32>`).
    TYPE_SIZE,

    // === Node kinds: Patterns ===
    /// Identifier pattern (binds a single name).
    IDENT_PAT,
    /// Tuple destructuring pattern.
    TUPLE_PAT,
    /// Single element inside a tuple destructuring pattern.
    TUPLE_PAT_ELT,
    /// Struct destructuring pattern.
    STRUCT_PAT,
    /// Single field binding inside a struct destructuring pattern.
    STRUCT_PAT_FIELD,
    /// Pattern with an explicit type annotation (`pat: Type`).
    TYPED_PAT,

    // === Node kinds: Statements ===
    /// Brace-delimited statement block.
    BLOCK,
    /// Assignment statement (`lhs = rhs;`, `lhs += rhs;`, etc.).
    ASSIGN_STMT,
    /// Assignment used as an expression (`lhs = rhs`).
    ///
    /// Only valid inside a parenthesized expression context (e.g. the
    /// body of a lambda or as a parenthesized sub-expression). The
    /// statement form `lhs = rhs;` is emitted as [`ASSIGN_STMT`](Self::ASSIGN_STMT).
    ASSIGN_EXPR,
    /// Compound-assignment used as an expression (`lhs += rhs`, `lhs -= rhs`, ...).
    ///
    /// Only valid inside a parenthesized expression context. The
    /// statement form is emitted as [`ASSIGN_STMT`](Self::ASSIGN_STMT).
    COMPOUND_ASSIGN_EXPR,
    /// Expression used as a statement (terminated by `;`).
    EXPR_STMT,
    /// `return` statement.
    RETURN_STMT,
    /// `if` / `else` statement.
    IF_STMT,
    /// `for` statement (counted or `for ... of` form).
    FOR_STMT,
    /// `assert` statement.
    ASSERT_STMT,
    /// `const` binding statement with a single name.
    CONST_STMT,
    /// `const` binding statement that destructures into multiple names.
    MULTI_CONST_STMT,

    // === Node kinds: Expressions ===
    /// Literal expression (wraps a literal token such as [`INT_LIT`](Self::INT_LIT)).
    LITERAL_EXPR,
    /// Bare-name reference expression (resolves to a binding in scope).
    NAME_EXPR,
    /// Ternary `cond ? then : else` expression.
    TERNARY_EXPR,
    /// Binary infix-operator expression (arithmetic, comparison, logical).
    BINARY_EXPR,
    /// Unary prefix-operator expression (`!x`, `-x`).
    UNARY_EXPR,
    /// `expr as Type` cast expression.
    CAST_EXPR,
    /// Function/circuit call expression.
    CALL_EXPR,
    /// `expr.field` member-access expression.
    MEMBER_EXPR,
    /// `expr[index]` index expression.
    INDEX_EXPR,
    /// Parenthesized expression (`( expr )`).
    PAREN_EXPR,
    /// Comma-separated expression sequence (e.g. inside parentheses or arguments).
    EXPR_SEQ,
    /// Array/vector literal expression (`[ a, b, c ]`).
    ARRAY_EXPR,
    /// Bytes literal expression (`bytes[...]`).
    BYTES_EXPR,
    /// Spread expression (`...expr`) inside a collection or argument list.
    SPREAD_EXPR,
    /// Struct/record literal expression (`Foo { field: value, ... }`).
    STRUCT_EXPR,
    /// Single `field: value` entry inside a [`STRUCT_EXPR`](Self::STRUCT_EXPR).
    STRUCT_FIELD_INIT,
    /// Struct update expression (`{ ...base, field: value }`).
    STRUCT_UPDATE,
    /// `default<Type>` expression that produces the default value of a type.
    DEFAULT_EXPR,
    /// `map` combinator expression.
    MAP_EXPR,
    /// `fold` combinator expression.
    FOLD_EXPR,
    /// `disclose(expr)` expression that lifts private data into the public domain.
    DISCLOSE_EXPR,
    /// `pad(...)` builtin expression that pads bytes to a fixed length.
    PAD_EXPR,
    /// `slice(...)` builtin expression that slices a sized collection.
    SLICE_EXPR,
    /// Lambda/closure expression (`(params) => body`).
    LAMBDA_EXPR,
    /// Parameter list of a lambda, circuit, or witness declaration.
    PARAM_LIST,
    /// Single parameter within a [`PARAM_LIST`](Self::PARAM_LIST).
    PARAM,
    /// Range expression (`lo..hi`) used in `for` loops and slices.
    RANGE_EXPR,
    /// `prefix` declaration node attached to an import.
    PREFIX_DECL,
    /// Named argument (`name: value`) within a call expression.
    NAMED_ARG,

    // === Node kinds: Version expressions ===
    /// Top-level version-constraint expression in a `pragma language_version` directive.
    VERSION_EXPR,
    /// Logical-and version-constraint expression.
    VERSION_AND_EXPR,
    /// Logical-or version-constraint expression.
    VERSION_OR_EXPR,
    /// Unary version-constraint expression (negation or relational prefix).
    VERSION_UNARY_EXPR,
    /// Parenthesized version-constraint expression.
    VERSION_PAREN_EXPR,
}

impl SyntaxKind {
    /// Returns `true` for whitespace and comment tokens.
    ///
    /// The lexer and parser preserve trivia in the green tree (so source can be
    /// round-tripped losslessly), but most downstream consumers — formatters,
    /// linters, code navigators — want to ignore it. Use this to filter trivia
    /// out of a token stream or descendants iterator.
    ///
    /// # Examples
    ///
    /// ```
    /// use compactp_syntax::SyntaxKind;
    ///
    /// assert!(SyntaxKind::WHITESPACE.is_trivia());
    /// assert!(SyntaxKind::LINE_COMMENT.is_trivia());
    /// assert!(SyntaxKind::BLOCK_COMMENT.is_trivia());
    ///
    /// assert!(!SyntaxKind::IDENT.is_trivia());
    /// assert!(!SyntaxKind::CIRCUIT_KW.is_trivia());
    /// ```
    pub fn is_trivia(self) -> bool {
        matches!(
            self,
            Self::WHITESPACE | Self::LINE_COMMENT | Self::BLOCK_COMMENT
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
