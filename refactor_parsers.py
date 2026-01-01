#!/usr/bin/env python3
"""
Batch refactor all parser files to use lexeme-based token matching.
Handles complex patterns that sed cannot handle reliably.
"""

import re
from pathlib import Path

def refactor_content(content):
    """Apply all refactoring transformations to file content."""
    original = content

    # 1. matches!(parser.peek(), Token::Feature(CONST)) -> parser.peek().lexeme == CONST
    content = re.sub(
        r'matches!\(parser\.peek\(\),\s*Token::Feature\(([A-Z_]+)\)\)',
        r'parser.peek().lexeme == \1',
        content
    )

    # 2. matches!(parser.peek(), Token::Feature(k) if *k == CONST) -> parser.peek().lexeme == CONST
    content = re.sub(
        r'matches!\(parser\.peek\(\),\s*Token::Feature\(k\)\s+if\s+\*k\s+==\s+([A-Z_]+)\)',
        r'parser.peek().lexeme == \1',
        content
    )

    # 3. matches!(parser.peek(), Token::Ident(_)) -> identifier check
    content = re.sub(
        r'matches!\(parser\.peek\(\),\s*Token::Ident\(_\)\)',
        r'parser.peek().lexeme.chars().next().map_or(false, |c| c.is_alphabetic() || c == \'_\')',
        content
    )

    # 4. matches!(parser.peek(), Token::Ident(s) if s == "word") -> parser.peek().lexeme == "word"
    content = re.sub(
        r'matches!\(\s*parser\.peek\(\),\s*Token::Ident\([^)]+\)\s+if\s+\w+\s+==\s+"([^"]+)"\s*\)',
        r'parser.peek().lexeme == "\1"',
        content
    )

    # 5. matches!(parser.peek(), Token::Number(_)) -> digit check
    content = re.sub(
        r'matches!\(parser\.peek\(\),\s*Token::Number\(_\)\)',
        r'parser.peek().lexeme.chars().next().map_or(false, |c| c.is_ascii_digit())',
        content
    )

    # 6. matches!(parser.peek(), Token::Feature(TRUE) | Token::Feature(FALSE)) -> boolean check
    content = re.sub(
        r'matches!\(parser\.peek\(\),\s*Token::Feature\(TRUE\)\s*\|\s*Token::Feature\(FALSE\)\)',
        r'(parser.peek().lexeme == "true" || parser.peek().lexeme == "false")',
        content
    )
    content = re.sub(
        r'matches!\(parser\.peek\(\),\s*Token::Feature\(TRUE\)\s*\|\s*Token::Feature\(FALSE\)\)',
        r'(parser.peek().lexeme == "TRUE" || parser.peek().lexeme == "FALSE")',
        content
    )

    # 7. match parser.advance() { Token::Ident(s) => s, ... } -> let name = parser.advance().lexeme;
    content = re.sub(
        r'match\s+parser\.advance\(\)\s*\{\s*Token::Ident\(s\)\s*=>\s*s,\s*_\s*=>\s*unreachable!\(\),?\s*\}',
        r'parser.advance().lexeme',
        content
    )
    content = re.sub(
        r'match\s+parser\.advance\(\)\s*\{\s*Token::Number\(s\)\s*=>\s*s,\s*_\s*=>\s*unreachable!\(\),?\s*\}',
        r'parser.advance().lexeme',
        content
    )

    # 8. match parser.advance() { Token::Feature(TRUE) => ... } -> if lexeme == "true"
    content = re.sub(
        r'match\s+parser\.advance\(\)\s*\{\s*Token::Feature\(TRUE\)\s*=>\s*Ok\(Box::new\(BoolLiteral\s*\{\s*value:\s*true\s*\}\)\),\s*Token::Feature\(FALSE\)\s*=>\s*Ok\(Box::new\(BoolLiteral\s*\{\s*value:\s*false\s*\}\)\),\s*_\s*=>\s*unreachable!\(\),?\s*\}',
        r'{ let value = parser.advance().lexeme == "true"; Ok(Box::new(BoolLiteral { value })) }',
        content
    )

    # 9. match parser.advance() { Token::Feature(k) if k == X => {} ... } -> if parser.advance().lexeme != X { ... }
    content = re.sub(
        r'match\s+parser\.advance\(\)\s*\{\s*Token::Feature\(k\)\s+if\s+k\s+==\s+([A-Z_]+)\s*=>\s*\{\},\s*_\s*=>\s*return\s+Err\(([^)]+)\),?\s*\}',
        r'if parser.advance().lexeme != \1 { return Err(\2); }',
        content
    )

    # 10. parser.peek_n(1) with Token::Feature
    content = re.sub(
        r'parser\.peek_n\(([0-9]+)\)\.map_or\(false,\s*\|t\|\s*matches!\(t,\s*Token::Feature\(([A-Z_]+)\)\)\)',
        r'parser.peek_n(\1).map_or(false, |t| t.lexeme == \2)',
        content
    )

    # 11. (Token::Ident(_), Some(Token::Feature(EQUALS))) in tuple patterns
    content = re.sub(
        r'\(Token::Ident\(_\),\s*Some\(Token::Feature\(([A-Z_]+)\)\)\)',
        r'(tok, Some(next)) if tok.lexeme.chars().next().map_or(false, |c| c.is_alphabetic() || c == \'_\') && next.lexeme == \1',
        content
    )

    # 12. Remove Token enum import if no longer used
    if 'Token::' not in content and 'use crate::kernel::lexer::Token;' in content:
        content = re.sub(r'use crate::kernel::lexer::Token;\n', '', content)

    # 13. Remove token registration calls
    content = re.sub(r'\s*reg\.tokens\.add_keyword\([^)]+\);\n', '', content)
    content = re.sub(r'\s*reg\.tokens\.add_single_char\([^)]+\);\n', '', content)
    content = re.sub(r'\s*reg\.tokens\.add_two_char\([^)]+\);\n', '', content)

    # 14. Add comment for register functions that had token registration
    if 'pub fn register(reg: &mut Registry)' in content and original != content:
        if '// No token registration needed' not in content:
            content = re.sub(
                r'(pub fn register\(reg: &mut Registry\)\s*\{\s*)\n',
                r'\1\n    // No token registration needed - kernel handles all segmentation\n',
                content,
                count=1
            )

    # 15. Clean up empty registration sections
    content = re.sub(r'// Register tokens\s*\n\s*\n', '', content)
    content = re.sub(r'// Token definitions\s*\n\s*\n\s*pub const', r'pub const', content)

    return content

def process_file(filepath):
    """Process a single Rust file."""
    try:
        content = filepath.read_text()
        refactored = refactor_content(content)

        if refactored != content:
            filepath.write_text(refactored)
            print(f"✓ {filepath}")
            return True
        return False
    except Exception as e:
        print(f"✗ {filepath}: {e}")
        return False

def main():
    base = Path('/home/user/lumen-lang')
    changed_count = 0
    total_count = 0

    # Process all language modules except lumen (already done)
    for lang in ['mini_rust', 'mini_c', 'mini_php', 'mini_sh', 'mini_apple_basic', 'mini_apple_pascal']:
        lang_path = base / f"src_{lang}"
        if not lang_path.exists():
            continue

        print(f"\n{'='*60}")
        print(f"Processing {lang}...")
        print(f"{'='*60}")

        for rs_file in lang_path.rglob('*.rs'):
            # Skip examples
            if 'examples' in str(rs_file):
                continue

            total_count += 1
            if process_file(rs_file):
                changed_count += 1

    print(f"\n{'='*60}")
    print(f"Complete! Changed {changed_count}/{total_count} files")
    print(f"{'='*60}")

if __name__ == '__main__':
    main()
