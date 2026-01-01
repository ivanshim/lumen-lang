#!/usr/bin/env python3
"""
Batch refactor all language modules to use lexeme-based token matching.
Converts from Token::Feature/Ident/Number/String to tok.lexeme string matching.
"""

import re
import os
from pathlib import Path

# Language-specific multi-char lexemes
LANG_LEXEMES = {
    "mini_rust": ["==", "!=", "<=", ">=", "&&", "||", ":=",
                   "let", "if", "else", "while", "break", "continue", "print", "true", "false"],
    "mini_c": ["==", "!=", "<=", ">=",
                "and", "or", "not", "if", "else", "while", "break", "continue", "printf", "true", "false"],
    "mini_php": ["==", "!=", "<=", ">=", "===", "!==",
                  "and", "or", "not", "if", "else", "while", "break", "continue", "echo", "true", "false"],
    "mini_sh": ["==", "!=", "<=", ">=",
                 "and", "or", "not", "if", "else", "while", "break", "continue", "echo", "true", "false"],
    "mini_apple_basic": ["==", "!=", "<=", ">=",
                          "AND", "OR", "NOT", "IF", "ELSE", "WHILE", "BREAK", "CONTINUE", "PRINT", "TRUE", "FALSE"],
    "mini_apple_pascal": ["==", "!=", "<=", ">=", ":=",
                           "and", "or", "not", "if", "else", "while", "break", "continue", "writeln", "true", "false"],
}

def refactor_token_matches(content):
    """Convert Token::Feature/Ident/Number matches to lexeme-based matching."""

    # Pattern 1: matches!(parser.peek(), Token::Feature(CONST))
    # -> parser.peek().lexeme == "const_value"
    content = re.sub(
        r'matches!\(parser\.peek\(\),\s*Token::Feature\(([A-Z_]+)\)\)',
        lambda m: f'parser.peek().lexeme == {m.group(1)}',
        content
    )

    # Pattern 2: matches!(parser.peek(), Token::Feature(k) if *k == CONST)
    # -> parser.peek().lexeme == CONST
    content = re.sub(
        r'matches!\(parser\.peek\(\),\s*Token::Feature\(k\)\s+if\s+\*k\s+==\s+([A-Z_]+)\)',
        lambda m: f'parser.peek().lexeme == {m.group(1)}',
        content
    )

    # Pattern 3: matches!(parser.peek(), Token::Ident(_))
    # -> Check if lexeme is identifier (starts with letter/underscore)
    content = re.sub(
        r'matches!\(parser\.peek\(\),\s*Token::Ident\(_\)\)',
        'parser.peek().lexeme.chars().next().map_or(false, |c| c.is_alphabetic() || c == \'_\')',
        content
    )

    # Pattern 4: matches!(parser.peek(), Token::Ident(s) if s == "word")
    # -> parser.peek().lexeme == "word"
    content = re.sub(
        r'matches!\(parser\.peek\(\),\s*Token::Ident\([^)]+\)\s+if\s+\w+\s+==\s+"([^"]+)"\)',
        lambda m: f'parser.peek().lexeme == "{m.group(1)}"',
        content
    )

    # Pattern 5: matches!(parser.peek(), Token::Number(_))
    # -> Check if lexeme starts with digit
    content = re.sub(
        r'matches!\(parser\.peek\(\),\s*Token::Number\(_\)\)',
        'parser.peek().lexeme.chars().next().map_or(false, |c| c.is_ascii_digit())',
        content
    )

    # Pattern 6: parser.advance() match arms
    # Token::Feature(CONST) => ... -> if lex == CONST ...
    content = re.sub(
        r'match\s+parser\.advance\(\)\s*\{([^}]+)\}',
        refactor_match_advance,
        content,
        flags=re.DOTALL
    )

    # Pattern 7: parser.advance() simple matching
    # Token::Ident(s) => s -> parser.advance().lexeme
    # Token::Number(s) => s -> parser.advance().lexeme
    content = re.sub(
        r'match\s+parser\.advance\(\)\s*\{\s*Token::(Ident|Number)\(s\)\s*=>\s*s,',
        'let lexeme = parser.advance().lexeme;',
        content
    )

    # Pattern 8: Inline Token::Feature checks
    # match parser.advance() { Token::Feature(k) if k == CONST => {} ... }
    # -> if parser.advance().lexeme != CONST { ... }
    content = re.sub(
        r'match\s+parser\.advance\(\)\s*\{\s*Token::Feature\(k\)\s+if\s+k\s+==\s+([A-Z_]+)\s*=>\s*\{\}',
        lambda m: f'if parser.advance().lexeme != {m.group(1)} {{',
        content
    )

    # Pattern 9: peek_n matching
    content = re.sub(
        r'parser\.peek_n\((\d+)\)\.map_or\(false,\s*\|t\|\s*matches!\(t,\s*Token::Feature\(([A-Z_]+)\)\)\)',
        lambda m: f'parser.peek_n({m.group(1)}).map_or(false, |t| t.lexeme == {m.group(2)})',
        content
    )

    return content

def refactor_match_advance(match_block):
    """Refactor match parser.advance() blocks."""
    content = match_block.group(0)
    # This is complex - for now, just do simple replacements
    # Token::Feature(k) if k == X => ... becomes if lex == X
    content = re.sub(
        r'Token::Feature\(k\)\s+if\s+k\s+==\s+([A-Z_]+)',
        r'_ if parser.peek().lexeme == \1',
        content
    )
    return content

def remove_token_registration(content):
    """Remove old token registration calls."""
    # Remove reg.tokens.add_keyword
    content = re.sub(r'\s*reg\.tokens\.add_keyword\([^)]+\);?\n', '', content)
    # Remove reg.tokens.add_single_char
    content = re.sub(r'\s*reg\.tokens\.add_single_char\([^)]+\);?\n', '', content)
    # Remove reg.tokens.add_two_char
    content = re.sub(r'\s*reg\.tokens\.add_two_char\([^)]+\);?\n', '', content)

    return content

def update_dispatcher(lang_path, lang_name):
    """Update language dispatcher to register multi-char lexemes."""
    dispatcher_file = lang_path / f"src_{lang_name}.rs"
    if not dispatcher_file.exists():
        return

    content = dispatcher_file.read_text()

    # Find register_all function and add multi-char lexemes registration
    lexemes = LANG_LEXEMES.get(lang_name, [])
    lexeme_list = ',\n        '.join([f'"{lex}"' for lex in lexemes])

    registration = f'''    // Register multi-character lexemes for maximal-munch segmentation
    // The kernel lexer will use these for pure lossless ASCII segmentation
    registry.tokens.set_multichar_lexemes(vec![
        {lexeme_list},
    ]);

'''

    # Insert after "pub fn register_all" line
    content = re.sub(
        r'(pub fn register_all\([^)]+\) \{\s*\n)',
        r'\1' + registration,
        content
    )

    dispatcher_file.write_text(content)
    print(f"Updated dispatcher: {dispatcher_file}")

def process_file(filepath):
    """Process a single Rust file."""
    if 'examples' in str(filepath):
        return  # Skip example files

    content = filepath.read_text()
    original = content

    # Apply refactoring
    content = refactor_token_matches(content)
    content = remove_token_registration(content)

    # Add comment about no token registration needed
    if 'pub fn register(reg: &mut Registry)' in content and 'reg.tokens.' in original:
        content = re.sub(
            r'(pub fn register\(reg: &mut Registry\) \{\s*\n)',
            r'\1    // No token registration needed - kernel handles all segmentation\n',
            content
        )

    if content != original:
        filepath.write_text(content)
        print(f"Updated: {filepath}")

def main():
    base = Path('/home/user/lumen-lang')

    # Update all language dispatchers
    for lang_name in LANG_LEXEMES.keys():
        lang_path = base / f"src_{lang_name}"
        if lang_path.exists():
            print(f"\nProcessing {lang_name}...")
            update_dispatcher(lang_path, lang_name)

            # Process all .rs files in this language
            for rs_file in lang_path.rglob('*.rs'):
                process_file(rs_file)

if __name__ == '__main__':
    main()
