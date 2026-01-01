#!/bin/bash
# Batch refactor all language modules to use lexeme-based token matching

set -e

echo "Starting batch refactoring of all language modules..."

# Function to refactor a single file
refactor_file() {
    local file="$1"

    # Skip if file doesn't exist or is in examples
    if [[ ! -f "$file" ]] || [[ "$file" == *"/examples/"* ]]; then
        return
    fi

    echo "Processing: $file"

    # Create backup
    cp "$file" "$file.bak"

    # Pattern 1: matches!(parser.peek(), Token::Feature(CONST)) -> parser.peek().lexeme == CONST
    sed -i 's/matches!(parser\.peek(), Token::Feature(\([A-Z_]*\)))/parser.peek().lexeme == \1/g' "$file"

    # Pattern 2: matches!(parser.peek(), Token::Feature(k) if \*k == CONST) -> parser.peek().lexeme == CONST
    sed -i 's/matches!(parser\.peek(), Token::Feature(k) if \*k == \([A-Z_]*\))/parser.peek().lexeme == \1/g' "$file"

    # Pattern 3: matches!(parser.peek(), Token::Ident(_)) -> identifier check
    sed -i 's/matches!(parser\.peek(), Token::Ident(_))/parser.peek().lexeme.chars().next().map_or(false, |c| c.is_alphabetic() || c == '\''_'\'')/g' "$file"

    # Pattern 4: matches!(parser.peek(), Token::Ident(s) if s == "word") -> parser.peek().lexeme == "word"
    sed -i 's/matches!(parser\.peek(), Token::Ident([^)]*) if [^ ]* == "\([^"]*\)")/parser.peek().lexeme == "\1"/g' "$file"

    # Pattern 5: matches!(parser.peek(), Token::Number(_)) -> digit check
    sed -i 's/matches!(parser\.peek(), Token::Number(_))/parser.peek().lexeme.chars().next().map_or(false, |c| c.is_ascii_digit())/g' "$file"

    # Pattern 6: Token::Ident(s) => s in match -> lexeme
    sed -i 's/Token::Ident(s) => s/lexeme = parser.advance().lexeme/g' "$file"
    sed -i 's/Token::Number(s) => s/lexeme = parser.advance().lexeme/g' "$file"

    # Pattern 7: match parser.advance() { Token::Feature(k) if k == CONST => {} -> if parser.advance().lexeme != CONST {
    sed -i 's/match parser\.advance() { Token::Feature(k) if k == \([A-Z_]*\) => {}/if parser.advance().lexeme != \1 {/g' "$file"

    # Pattern 8: parser.peek_n matching
    sed -i 's/parser\.peek_n(\([0-9]*\))\.map_or(false, |t| matches!(t, Token::Feature(\([A-Z_]*\))))/parser.peek_n(\1).map_or(false, |t| t.lexeme == \2)/g' "$file"

    # Pattern 9: Remove token registration calls
    sed -i '/reg\.tokens\.add_keyword/d' "$file"
    sed -i '/reg\.tokens\.add_single_char/d' "$file"
    sed -i '/reg\.tokens\.add_two_char/d' "$file"

    # Pattern 10: Add comment in empty register functions
    if grep -q "pub fn register(reg: &mut Registry) {$" "$file" 2>/dev/null; then
        sed -i '/pub fn register(reg: &mut Registry) {$/a\    // No token registration needed - kernel handles all segmentation' "$file"
    fi

    # Remove use statements for Token if no longer needed (simple heuristic)
    if ! grep -q "Token::" "$file" 2>/dev/null; then
        sed -i '/use crate::kernel::lexer::Token;/d' "$file"
    fi
}

# Export function for use with find -exec
export -f refactor_file

# Refactor all language module files (excluding lumen which is already done)
for lang in mini_rust mini_c mini_php mini_sh mini_apple_basic mini_apple_pascal; do
    echo "===================="
    echo "Refactoring $lang..."
    echo "===================="

    find "/home/user/lumen-lang/src_$lang" -name "*.rs" -type f -exec bash -c 'refactor_file "$0"' {} \;
done

echo "Batch refactoring complete!"
echo "Backup files created with .bak extension"
