use compactp_syntax::SyntaxKind;

/// Lex source code into a sequence of (SyntaxKind, &str) pairs.
///
/// Every byte of the input is represented in the output — nothing is discarded.
/// This is required for lossless CST construction.
pub fn lex(source: &str) -> Vec<(SyntaxKind, &str)> {
    let mut tokens = Vec::new();
    let mut pos = 0;
    let bytes = source.as_bytes();

    while pos < bytes.len() {
        let start = pos;
        let b = bytes[pos];

        match b {
            // Whitespace
            b' ' | b'\t' | b'\r' | b'\n' => {
                pos += 1;
                while pos < bytes.len() && matches!(bytes[pos], b' ' | b'\t' | b'\r' | b'\n') {
                    pos += 1;
                }
                tokens.push((SyntaxKind::WHITESPACE, &source[start..pos]));
            }

            // Line comment or block comment or slash
            b'/' => {
                pos += 1;
                if pos < bytes.len() && bytes[pos] == b'/' {
                    // Line comment: consume until newline (not including it)
                    pos += 1;
                    while pos < bytes.len() && bytes[pos] != b'\n' {
                        pos += 1;
                    }
                    tokens.push((SyntaxKind::LINE_COMMENT, &source[start..pos]));
                } else if pos < bytes.len() && bytes[pos] == b'*' {
                    // Block comment: consume until */
                    pos += 1;
                    let mut terminated = false;
                    let mut nested = false;
                    while pos < bytes.len() {
                        if bytes[pos] == b'*' && pos + 1 < bytes.len() && bytes[pos + 1] == b'/' {
                            pos += 2;
                            terminated = true;
                            break;
                        }
                        if bytes[pos] == b'/' && pos + 1 < bytes.len() && bytes[pos + 1] == b'*' {
                            nested = true;
                        }
                        pos += 1;
                    }
                    if !terminated || nested {
                        tokens.push((SyntaxKind::ERROR, &source[start..pos]));
                    } else {
                        tokens.push((SyntaxKind::BLOCK_COMMENT, &source[start..pos]));
                    }
                } else {
                    tokens.push((SyntaxKind::SLASH, &source[start..pos]));
                }
            }

            // String literal (double-quoted)
            b'"' => {
                pos += 1;
                let mut terminated = false;
                while pos < bytes.len() {
                    match bytes[pos] {
                        b'"' => {
                            pos += 1;
                            terminated = true;
                            break;
                        }
                        b'\\' => {
                            pos += 1; // skip the backslash
                            if pos < bytes.len() {
                                pos += 1; // skip the escaped char
                            }
                        }
                        _ => pos += 1,
                    }
                }
                if terminated {
                    tokens.push((SyntaxKind::STRING_LIT, &source[start..pos]));
                } else {
                    tokens.push((SyntaxKind::ERROR, &source[start..pos]));
                }
            }

            // String literal (single-quoted)
            b'\'' => {
                pos += 1;
                let mut terminated = false;
                while pos < bytes.len() {
                    match bytes[pos] {
                        b'\'' => {
                            pos += 1;
                            terminated = true;
                            break;
                        }
                        b'\\' => {
                            pos += 1;
                            if pos < bytes.len() {
                                pos += 1;
                            }
                        }
                        _ => pos += 1,
                    }
                }
                if terminated {
                    tokens.push((SyntaxKind::STRING_LIT, &source[start..pos]));
                } else {
                    tokens.push((SyntaxKind::ERROR, &source[start..pos]));
                }
            }

            // Numeric literals: 0-prefixed or decimal
            b'0' => {
                pos += 1;
                if pos < bytes.len() {
                    match bytes[pos] {
                        b'x' | b'X' => {
                            pos += 1;
                            let digit_start = pos;
                            while pos < bytes.len() && is_hex_digit(bytes[pos]) {
                                pos += 1;
                            }
                            if pos == digit_start {
                                tokens.push((SyntaxKind::ERROR, &source[start..pos]));
                            } else {
                                tokens.push((SyntaxKind::HEX_LIT, &source[start..pos]));
                            }
                        }
                        b'o' | b'O' => {
                            pos += 1;
                            let digit_start = pos;
                            while pos < bytes.len() && matches!(bytes[pos], b'0'..=b'7') {
                                pos += 1;
                            }
                            if pos == digit_start {
                                tokens.push((SyntaxKind::ERROR, &source[start..pos]));
                            } else {
                                tokens.push((SyntaxKind::OCT_LIT, &source[start..pos]));
                            }
                        }
                        b'b' | b'B' => {
                            pos += 1;
                            let digit_start = pos;
                            while pos < bytes.len() && matches!(bytes[pos], b'0' | b'1') {
                                pos += 1;
                            }
                            if pos == digit_start {
                                tokens.push((SyntaxKind::ERROR, &source[start..pos]));
                            } else {
                                tokens.push((SyntaxKind::BIN_LIT, &source[start..pos]));
                            }
                        }
                        b'.' => {
                            // Could be version literal (0.N.N) or just 0 followed by dot
                            pos = lex_version_or_int(source, bytes, start, pos, &mut tokens);
                        }
                        _ => {
                            tokens.push((SyntaxKind::INT_LIT, &source[start..pos]));
                        }
                    }
                } else {
                    tokens.push((SyntaxKind::INT_LIT, &source[start..pos]));
                }
            }

            b'1'..=b'9' => {
                pos += 1;
                while pos < bytes.len() && bytes[pos].is_ascii_digit() {
                    pos += 1;
                }
                // Check for version literal: N.N or N.N.N
                if pos < bytes.len() && bytes[pos] == b'.' {
                    pos = lex_version_or_int(source, bytes, start, pos, &mut tokens);
                } else {
                    tokens.push((SyntaxKind::INT_LIT, &source[start..pos]));
                }
            }

            // Identifiers and keywords
            b if is_ident_start(b) => {
                pos += 1;
                while pos < bytes.len() && is_ident_continue(bytes[pos]) {
                    pos += 1;
                }
                let text = &source[start..pos];
                let kind = keyword_or_ident(text);
                tokens.push((kind, text));
            }

            // Multi-char operators and punctuation
            b'=' => {
                pos += 1;
                if pos < bytes.len() && bytes[pos] == b'=' {
                    pos += 1;
                    tokens.push((SyntaxKind::EQ_EQ, &source[start..pos]));
                } else if pos < bytes.len() && bytes[pos] == b'>' {
                    pos += 1;
                    tokens.push((SyntaxKind::FAT_ARROW, &source[start..pos]));
                } else {
                    tokens.push((SyntaxKind::EQ, &source[start..pos]));
                }
            }

            b'!' => {
                pos += 1;
                if pos < bytes.len() && bytes[pos] == b'=' {
                    pos += 1;
                    tokens.push((SyntaxKind::BANG_EQ, &source[start..pos]));
                } else {
                    tokens.push((SyntaxKind::BANG, &source[start..pos]));
                }
            }

            b'<' => {
                pos += 1;
                if pos < bytes.len() && bytes[pos] == b'=' {
                    pos += 1;
                    tokens.push((SyntaxKind::LT_EQ, &source[start..pos]));
                } else {
                    tokens.push((SyntaxKind::LT, &source[start..pos]));
                }
            }

            b'>' => {
                pos += 1;
                if pos < bytes.len() && bytes[pos] == b'=' {
                    pos += 1;
                    tokens.push((SyntaxKind::GT_EQ, &source[start..pos]));
                } else {
                    tokens.push((SyntaxKind::GT, &source[start..pos]));
                }
            }

            b'&' => {
                pos += 1;
                if pos < bytes.len() && bytes[pos] == b'&' {
                    pos += 1;
                    tokens.push((SyntaxKind::AMP_AMP, &source[start..pos]));
                } else {
                    // Single & is an error per upstream lexer
                    tokens.push((SyntaxKind::ERROR, &source[start..pos]));
                }
            }

            b'|' => {
                pos += 1;
                if pos < bytes.len() && bytes[pos] == b'|' {
                    pos += 1;
                    tokens.push((SyntaxKind::PIPE_PIPE, &source[start..pos]));
                } else {
                    // Single | is an error per upstream lexer
                    tokens.push((SyntaxKind::ERROR, &source[start..pos]));
                }
            }

            b'+' => {
                pos += 1;
                if pos < bytes.len() && bytes[pos] == b'=' {
                    pos += 1;
                    tokens.push((SyntaxKind::PLUS_EQ, &source[start..pos]));
                } else {
                    tokens.push((SyntaxKind::PLUS, &source[start..pos]));
                }
            }

            b'-' => {
                pos += 1;
                if pos < bytes.len() && bytes[pos] == b'=' {
                    pos += 1;
                    tokens.push((SyntaxKind::MINUS_EQ, &source[start..pos]));
                } else {
                    tokens.push((SyntaxKind::MINUS, &source[start..pos]));
                }
            }

            b'*' => {
                pos += 1;
                tokens.push((SyntaxKind::STAR, &source[start..pos]));
            }

            b'.' => {
                pos += 1;
                if pos < bytes.len() && bytes[pos] == b'.' {
                    pos += 1;
                    if pos < bytes.len() && bytes[pos] == b'.' {
                        pos += 1;
                        tokens.push((SyntaxKind::DOT_DOT_DOT, &source[start..pos]));
                    } else {
                        tokens.push((SyntaxKind::DOT_DOT, &source[start..pos]));
                    }
                } else {
                    tokens.push((SyntaxKind::DOT, &source[start..pos]));
                }
            }

            b'?' => {
                pos += 1;
                tokens.push((SyntaxKind::QUESTION, &source[start..pos]));
            }

            // Single-char delimiters
            b'(' => {
                pos += 1;
                tokens.push((SyntaxKind::L_PAREN, &source[start..pos]));
            }
            b')' => {
                pos += 1;
                tokens.push((SyntaxKind::R_PAREN, &source[start..pos]));
            }
            b'{' => {
                pos += 1;
                tokens.push((SyntaxKind::L_BRACE, &source[start..pos]));
            }
            b'}' => {
                pos += 1;
                tokens.push((SyntaxKind::R_BRACE, &source[start..pos]));
            }
            b'[' => {
                pos += 1;
                tokens.push((SyntaxKind::L_BRACKET, &source[start..pos]));
            }
            b']' => {
                pos += 1;
                tokens.push((SyntaxKind::R_BRACKET, &source[start..pos]));
            }
            b',' => {
                pos += 1;
                tokens.push((SyntaxKind::COMMA, &source[start..pos]));
            }
            b':' => {
                pos += 1;
                tokens.push((SyntaxKind::COLON, &source[start..pos]));
            }
            b';' => {
                pos += 1;
                tokens.push((SyntaxKind::SEMICOLON, &source[start..pos]));
            }
            b'#' => {
                pos += 1;
                tokens.push((SyntaxKind::HASH, &source[start..pos]));
            }

            // Unknown character — advance by full UTF-8 codepoint width.
            // The outer `while pos < source.len()` loop guarantees at least
            // one char is available here; fall back to one-byte advance if the
            // slice is somehow empty (defensive, keeps the lexer progressing).
            _ => {
                let step = source[pos..].chars().next().map_or(1, char::len_utf8);
                pos += step;
                tokens.push((SyntaxKind::ERROR, &source[start..pos]));
            }
        }
    }

    tokens
}

/// Try to lex a version literal (N.N or N.N.N) starting from a position where we've
/// already consumed digits and are sitting at a dot. If this doesn't look like a version
/// literal, fall back to INT_LIT and reset pos.
fn lex_version_or_int<'a>(
    source: &'a str,
    bytes: &[u8],
    start: usize,
    dot_pos: usize,
    tokens: &mut Vec<(SyntaxKind, &'a str)>,
) -> usize {
    let mut pos = dot_pos + 1; // skip the first dot

    // Need at least one digit after the first dot
    if pos >= bytes.len() || !bytes[pos].is_ascii_digit() {
        // Not a version literal, emit as INT_LIT (without the dot)
        tokens.push((SyntaxKind::INT_LIT, &source[start..dot_pos]));
        return dot_pos; // return pos at the dot so it gets lexed as DOT
    }

    // Consume second number
    while pos < bytes.len() && bytes[pos].is_ascii_digit() {
        pos += 1;
    }

    // Check for third component (N.N.N)
    if pos < bytes.len() && bytes[pos] == b'.' {
        let second_dot = pos;
        pos += 1;
        if pos < bytes.len() && bytes[pos].is_ascii_digit() {
            // Third component
            while pos < bytes.len() && bytes[pos].is_ascii_digit() {
                pos += 1;
            }
            tokens.push((SyntaxKind::VERSION_LIT, &source[start..pos]));
            return pos;
        }
        // Only two components: N.N (still a valid version per upstream)
        tokens.push((SyntaxKind::VERSION_LIT, &source[start..second_dot]));
        return second_dot;
    }

    // Two components: N.N
    tokens.push((SyntaxKind::VERSION_LIT, &source[start..pos]));
    pos
}

fn is_hex_digit(b: u8) -> bool {
    b.is_ascii_hexdigit()
}

fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_' || b == b'$'
}

fn is_ident_continue(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'$'
}

/// Classify an identifier text as a keyword or IDENT.
fn keyword_or_ident(text: &str) -> SyntaxKind {
    match text {
        "true" => SyntaxKind::TRUE_KW,
        "false" => SyntaxKind::FALSE_KW,
        "pragma" => SyntaxKind::PRAGMA_KW,
        "include" => SyntaxKind::INCLUDE_KW,
        "import" => SyntaxKind::IMPORT_KW,
        "from" => SyntaxKind::FROM_KW,
        "prefix" => SyntaxKind::PREFIX_KW,
        "export" => SyntaxKind::EXPORT_KW,
        "module" => SyntaxKind::MODULE_KW,
        "ledger" => SyntaxKind::LEDGER_KW,
        "constructor" => SyntaxKind::CONSTRUCTOR_KW,
        "circuit" => SyntaxKind::CIRCUIT_KW,
        "witness" => SyntaxKind::WITNESS_KW,
        "contract" => SyntaxKind::CONTRACT_KW,
        "struct" => SyntaxKind::STRUCT_KW,
        "enum" => SyntaxKind::ENUM_KW,
        "type" => SyntaxKind::TYPE_KW,
        "const" => SyntaxKind::CONST_KW,
        "return" => SyntaxKind::RETURN_KW,
        "if" => SyntaxKind::IF_KW,
        "else" => SyntaxKind::ELSE_KW,
        "for" => SyntaxKind::FOR_KW,
        "of" => SyntaxKind::OF_KW,
        "assert" => SyntaxKind::ASSERT_KW,
        "as" => SyntaxKind::AS_KW,
        "pure" => SyntaxKind::PURE_KW,
        "sealed" => SyntaxKind::SEALED_KW,
        "new" => SyntaxKind::NEW_KW,
        "map" => SyntaxKind::MAP_KW,
        "fold" => SyntaxKind::FOLD_KW,
        "default" => SyntaxKind::DEFAULT_KW,
        "disclose" => SyntaxKind::DISCLOSE_KW,
        "pad" => SyntaxKind::PAD_KW,
        "slice" => SyntaxKind::SLICE_KW,
        "Boolean" => SyntaxKind::BOOLEAN_KW,
        "Field" => SyntaxKind::FIELD_KW,
        "Uint" => SyntaxKind::UINT_KW,
        "Bytes" => SyntaxKind::BYTES_KW,
        "Opaque" => SyntaxKind::OPAQUE_KW,
        "Vector" => SyntaxKind::VECTOR_KW,
        "Unsigned" => SyntaxKind::UNSIGNED_KW,
        "Integer" => SyntaxKind::INTEGER_KW,
        _ => SyntaxKind::IDENT,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use expect_test::{Expect, expect};

    fn check(input: &str, expected: Expect) {
        let tokens: Vec<_> = lex(input)
            .iter()
            .map(|(kind, text)| format!("{kind:?} {text:?}"))
            .collect();
        expected.assert_eq(&tokens.join("\n"));
    }

    #[test]
    fn lex_whitespace() {
        check("  \n\t", expect![[r#"WHITESPACE "  \n\t""#]]);
    }

    #[test]
    fn lex_keywords() {
        check(
            "circuit pure export",
            expect![[r#"
                CIRCUIT_KW "circuit"
                WHITESPACE " "
                PURE_KW "pure"
                WHITESPACE " "
                EXPORT_KW "export""#]],
        );
    }

    #[test]
    fn lex_identifier_with_dollar() {
        check(
            "private$secret_key",
            expect![[r#"IDENT "private$secret_key""#]],
        );
    }

    #[test]
    fn lex_numeric_literals() {
        check(
            "42 0x1F 0o77 0b1010",
            expect![[r#"
                INT_LIT "42"
                WHITESPACE " "
                HEX_LIT "0x1F"
                WHITESPACE " "
                OCT_LIT "0o77"
                WHITESPACE " "
                BIN_LIT "0b1010""#]],
        );
    }

    #[test]
    fn lex_operators() {
        check(
            "== != <= >= && || += -= =>",
            expect![[r#"
                EQ_EQ "=="
                WHITESPACE " "
                BANG_EQ "!="
                WHITESPACE " "
                LT_EQ "<="
                WHITESPACE " "
                GT_EQ ">="
                WHITESPACE " "
                AMP_AMP "&&"
                WHITESPACE " "
                PIPE_PIPE "||"
                WHITESPACE " "
                PLUS_EQ "+="
                WHITESPACE " "
                MINUS_EQ "-="
                WHITESPACE " "
                FAT_ARROW "=>""#]],
        );
    }

    #[test]
    fn lex_dots() {
        check(
            ". .. ...",
            expect![[r#"
                DOT "."
                WHITESPACE " "
                DOT_DOT ".."
                WHITESPACE " "
                DOT_DOT_DOT "...""#]],
        );
    }

    #[test]
    fn lex_string() {
        check(
            r#""hello world""#,
            expect![[r#"STRING_LIT "\"hello world\"""#]],
        );
    }

    #[test]
    fn lex_single_quoted_string() {
        check("'hello'", expect![[r#"STRING_LIT "'hello'""#]]);
    }

    #[test]
    fn lex_line_comment() {
        check(
            "// a comment\ncode",
            expect![[r#"
                LINE_COMMENT "// a comment"
                WHITESPACE "\n"
                IDENT "code""#]],
        );
    }

    #[test]
    fn lex_block_comment() {
        check("/* block */", expect![[r#"BLOCK_COMMENT "/* block */""#]]);
    }

    #[test]
    fn lex_version_literal() {
        check("0.15.0", expect![[r#"VERSION_LIT "0.15.0""#]]);
    }

    #[test]
    fn lex_two_part_version() {
        check("0.15", expect![[r#"VERSION_LIT "0.15""#]]);
    }

    #[test]
    fn lex_boolean_keywords() {
        check(
            "true false",
            expect![[r#"
                TRUE_KW "true"
                WHITESPACE " "
                FALSE_KW "false""#]],
        );
    }

    #[test]
    fn lex_builtin_type_keywords() {
        check(
            "Boolean Field Uint Bytes Opaque Vector",
            expect![[r#"
                BOOLEAN_KW "Boolean"
                WHITESPACE " "
                FIELD_KW "Field"
                WHITESPACE " "
                UINT_KW "Uint"
                WHITESPACE " "
                BYTES_KW "Bytes"
                WHITESPACE " "
                OPAQUE_KW "Opaque"
                WHITESPACE " "
                VECTOR_KW "Vector""#]],
        );
    }

    #[test]
    fn lex_delimiters() {
        check(
            "(){}<>[],:;",
            expect![[r#"
                L_PAREN "("
                R_PAREN ")"
                L_BRACE "{"
                R_BRACE "}"
                LT "<"
                GT ">"
                L_BRACKET "["
                R_BRACKET "]"
                COMMA ","
                COLON ":"
                SEMICOLON ";""#]],
        );
    }

    #[test]
    fn lex_hash() {
        check("#", expect![[r##"HASH "#""##]]);
    }

    // === Edge case tests ===

    #[test]
    fn lex_empty_input() {
        check("", expect![[""]]);
    }

    #[test]
    fn lex_unterminated_string() {
        check(r#""hello"#, expect![[r#"ERROR "\"hello""#]]);
    }

    #[test]
    fn lex_unterminated_block_comment() {
        check("/* unterminated", expect![[r#"ERROR "/* unterminated""#]]);
    }

    #[test]
    fn lex_zero_literal() {
        check("0", expect![[r#"INT_LIT "0""#]]);
    }

    #[test]
    fn lex_keyword_prefix_as_ident() {
        // "forked" should be IDENT, not FOR_KW + IDENT
        check("forked", expect![[r#"IDENT "forked""#]]);
    }

    #[test]
    fn lex_ident_starting_with_underscore() {
        check("_private", expect![[r#"IDENT "_private""#]]);
    }

    #[test]
    fn lex_single_ampersand_is_error() {
        check("&", expect![[r#"ERROR "&""#]]);
    }

    #[test]
    fn lex_single_pipe_is_error() {
        check("|", expect![[r#"ERROR "|""#]]);
    }

    #[test]
    fn lex_slash_token() {
        check("/", expect![[r#"SLASH "/""#]]);
    }

    #[test]
    fn lex_string_with_escape() {
        check(
            r#""hello\nworld""#,
            expect![[r#"STRING_LIT "\"hello\\nworld\"""#]],
        );
    }

    #[test]
    fn lex_all_keywords() {
        check(
            "pragma include import from prefix export module ledger constructor circuit witness contract struct enum type const return if else for of assert as pure sealed new map fold default disclose pad slice",
            expect![[r#"
                PRAGMA_KW "pragma"
                WHITESPACE " "
                INCLUDE_KW "include"
                WHITESPACE " "
                IMPORT_KW "import"
                WHITESPACE " "
                FROM_KW "from"
                WHITESPACE " "
                PREFIX_KW "prefix"
                WHITESPACE " "
                EXPORT_KW "export"
                WHITESPACE " "
                MODULE_KW "module"
                WHITESPACE " "
                LEDGER_KW "ledger"
                WHITESPACE " "
                CONSTRUCTOR_KW "constructor"
                WHITESPACE " "
                CIRCUIT_KW "circuit"
                WHITESPACE " "
                WITNESS_KW "witness"
                WHITESPACE " "
                CONTRACT_KW "contract"
                WHITESPACE " "
                STRUCT_KW "struct"
                WHITESPACE " "
                ENUM_KW "enum"
                WHITESPACE " "
                TYPE_KW "type"
                WHITESPACE " "
                CONST_KW "const"
                WHITESPACE " "
                RETURN_KW "return"
                WHITESPACE " "
                IF_KW "if"
                WHITESPACE " "
                ELSE_KW "else"
                WHITESPACE " "
                FOR_KW "for"
                WHITESPACE " "
                OF_KW "of"
                WHITESPACE " "
                ASSERT_KW "assert"
                WHITESPACE " "
                AS_KW "as"
                WHITESPACE " "
                PURE_KW "pure"
                WHITESPACE " "
                SEALED_KW "sealed"
                WHITESPACE " "
                NEW_KW "new"
                WHITESPACE " "
                MAP_KW "map"
                WHITESPACE " "
                FOLD_KW "fold"
                WHITESPACE " "
                DEFAULT_KW "default"
                WHITESPACE " "
                DISCLOSE_KW "disclose"
                WHITESPACE " "
                PAD_KW "pad"
                WHITESPACE " "
                SLICE_KW "slice""#]],
        );
    }

    #[test]
    fn lex_question_mark() {
        check("?", expect![[r#"QUESTION "?""#]]);
    }

    #[test]
    fn lex_bang_alone() {
        check("!", expect![[r#"BANG "!""#]]);
    }

    #[test]
    fn lex_eq_alone() {
        check("=", expect![[r#"EQ "=""#]]);
    }

    #[test]
    fn lex_star() {
        check("*", expect![[r#"STAR "*""#]]);
    }

    #[test]
    fn lex_plus_minus() {
        check(
            "+ -",
            expect![[r#"
                PLUS "+"
                WHITESPACE " "
                MINUS "-""#]],
        );
    }
}
