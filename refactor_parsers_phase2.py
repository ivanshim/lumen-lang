#!/usr/bin/env python3
"""
Phase 2: Handle remaining complex Token enum patterns.
"""

import re
from pathlib import Path

def refactor_content_phase2(content):
    """Apply phase 2 refactoring transformations."""
    original = content

    # 1. match parser.advance() { Token::Number(s) => Ok(...) } -> let s = parser.advance().lexeme; Ok(...)
    content = re.sub(
        r'match\s+parser\.advance\(\)\s*\{\s*Token::Number\(s\)\s*=>\s*Ok\(([^}]+)\}\),\s*_\s*=>\s*unreachable!\(\),?\s*\}',
        r'let s = parser.advance().lexeme; Ok(\1})',
        content,
        flags=re.DOTALL
    )

    # 2. match parser.advance() { Token::Ident(s) => s, _ => unreachable!() } -> parser.advance().lexeme
    content = re.sub(
        r'match\s+parser\.advance\(\)\s*\{\s*Token::Ident\(s\)\s*=>\s*s,\s*_\s*=>\s*unreachable!\(\),?\s*\}',
        r'parser.advance().lexeme',
        content
    )

    # 3. match parser.advance() { Token::Feature(X) => {} _ => return Err(...) } -> if parser.advance().lexeme != X { return Err(...) }
    content = re.sub(
        r'match\s+parser\.advance\(\)\s*\{\s*Token::Feature\(([A-Z_:]+)\)\s*=>\s*\{\},?\s*_\s*=>\s*return\s+Err\(([^)]+)\),?\s*\}',
        r'if parser.advance().lexeme != \1 { return Err(\2); }',
        content
    )

    # 4. matches!(parser.peek(), Token::Feature(k) if *k == X || *k == Y) -> check lexeme OR
    content = re.sub(
        r'matches!\(parser\.peek\(\),\s*Token::Feature\(k\)\s+if\s+\*k\s+==\s+([A-Z_]+)\s+\|\|\s+\*k\s+==\s+([A-Z_]+)\)',
        r'(parser.peek().lexeme == \1 || parser.peek().lexeme == \2)',
        content
    )

    # 5. while !matches!(parser.peek(), Token::Feature(k) if *k == X || *k == Y) -> while lexeme != X && lexeme != Y
    content = re.sub(
        r'while\s+!matches!\(parser\.peek\(\),\s*Token::Feature\(k\)\s+if\s+\*k\s+==\s+([A-Z_]+)\s+\|\|\s+\*k\s+==\s+([A-Z_]+)\)',
        r'while parser.peek().lexeme != \1 && parser.peek().lexeme != \2',
        content
    )

    # 6. Token::Feature(X) in match arms -> use lexeme comparison instead
    content = re.sub(
        r'(\s+)Token::Feature\(([A-Z_:]+)\)\s*=>\s*\{\}',
        r'\1_ if parser.peek().lexeme == \2 => {}',
        content
    )

    # 7. tok: Token::Feature(EOF) -> tok: Token::new(EOF.to_string())
    content = re.sub(
        r'tok:\s*Token::Feature\(([A-Z_]+)\)',
        r'tok: Token::new(\1.to_string())',
        content
    )

    return content

def process_file(filepath):
    """Process a single Rust file."""
    try:
        content = filepath.read_text()
        refactored = refactor_content_phase2(content)

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

    # Process all language modules
    for lang in ['lumen', 'mini_rust', 'mini_c', 'mini_php', 'mini_sh', 'mini_apple_basic', 'mini_apple_pascal']:
        lang_path = base / f"src_{lang}"
        if not lang_path.exists():
            continue

        print(f"\nProcessing {lang}...")

        for rs_file in lang_path.rglob('*.rs'):
            if 'examples' in str(rs_file):
                continue

            total_count += 1
            if process_file(rs_file):
                changed_count += 1

    print(f"\nPhase 2 Complete! Changed {changed_count}/{total_count} files")

if __name__ == '__main__':
    main()
