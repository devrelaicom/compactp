#!/usr/bin/env bash
set -euo pipefail

# compactp Interactive Demo
# Builds the project and walks through every CLI command.

BOLD='\033[1m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
RED='\033[0;31m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

section() {
    echo ""
    echo -e "${BOLD}${CYAN}═══════════════════════════════════════════════${NC}"
    echo -e "${BOLD}${CYAN}  $1${NC}"
    echo -e "${BOLD}${CYAN}═══════════════════════════════════════════════${NC}"
    echo ""
}

pause() {
    echo -e "${YELLOW}Press Enter to continue...${NC}"
    read -r </dev/tty || true
}

section "Building compactp"
cargo build --release 2>&1
COMPACTP=./target/release/compactp
echo -e "${GREEN}Build complete!${NC}"
echo ""
$COMPACTP --version
pause

section "1. Lex Command — Tokenize a file"
echo -e "Input: tests/fixtures/demo/valid.compact"
echo ""
$COMPACTP lex tests/fixtures/demo/valid.compact
pause

section "2. Parse Command — Check for errors (valid file)"
echo -e "Input: tests/fixtures/demo/valid.compact"
echo ""
$COMPACTP parse tests/fixtures/demo/valid.compact
echo -e "\n${GREEN}Exit code: $?${NC}"
pause

section "3. Parse Command — Check for errors (invalid file)"
echo -e "Input: tests/fixtures/demo/invalid.compact"
echo ""
$COMPACTP parse tests/fixtures/demo/invalid.compact 2>&1 || true
pause

section "4. CST Command — Dump concrete syntax tree"
echo -e "Input: tests/fixtures/demo/valid.compact"
echo ""
$COMPACTP cst tests/fixtures/demo/valid.compact
pause

section "5. Stats Command — File statistics"
echo -e "Input: tests/fixtures/demo/valid.compact"
echo ""
$COMPACTP stats tests/fixtures/demo/valid.compact
pause

section "6. JSON Output — Machine-readable format"
echo -e "Input: tests/fixtures/demo/valid.compact"
echo ""
$COMPACTP parse --format json tests/fixtures/demo/valid.compact
pause

section "7. Stdin Input — Pipe source code"
echo -e "Input: echo 'ledger x: Field;' | compactp parse"
echo ""
echo 'ledger x: Field;' | $COMPACTP parse --stdin-filename inline.compact
echo -e "\n${GREEN}Exit code: 0${NC}"
pause

section "8. Stats on entire corpus"
echo -e "Running stats on tests/corpus/ directory..."
echo ""
$COMPACTP stats tests/corpus/ 2>/dev/null | tail -20 || true
echo "..."
pause

section "Demo Complete!"
echo -e "${GREEN}compactp is a production-grade parser for the Compact language (Midnight Network).${NC}"
echo ""
echo "Key features:"
echo "  - Lossless concrete syntax tree (every byte preserved)"
echo "  - Typed AST wrappers for safe access"
echo "  - Error recovery (continues parsing after errors)"
echo "  - Human and JSON output formats"
echo "  - File, directory, and stdin input"
echo ""
echo -e "Run ${BOLD}compactp --help${NC} for all options."
