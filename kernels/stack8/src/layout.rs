// Shaping: block boundaries. An indented language gets Open and Close
// tokens from its indentation; the other styles carry delimiters in the
// text. Inside brackets a line end is only space.

use crate::lang::{Lang, Blocks};
use crate::lex::{Shape, Token};

pub fn layout(tokens: Vec<Token>, lang: &Lang) -> Result<Vec<Token>, String> {
    let pairs: Vec<&crate::lang::Brackets> = [&lang.grouping, &lang.calling, &lang.array_brackets].into_iter().flatten().collect();
    let indented = lang.blocks == Blocks::Indented;
    let mut out = Vec::with_capacity(tokens.len());
    let mut nesting = 0usize;
    let mut levels = vec![0usize];
    let boundary = |shape: Shape, row: usize| Token { shape, lexeme: String::new(), width: 0, row, column: 0 };
    for tok in tokens {
        match tok.shape {
            Shape::Lead if indented && nesting == 0 => {
                if tok.width % lang.indent_width != 0 {
                    return Err(format!("Invalid indentation at line {}", tok.row));
                }
                let level = tok.width / lang.indent_width;
                let current = *levels.last().expect("a level");
                if level > current {
                    levels.push(level);
                    out.push(boundary(Shape::Open, tok.row));
                    continue;
                }
                while *levels.last().expect("a level") > level {
                    levels.pop();
                    out.push(boundary(Shape::Close, tok.row));
                }
                if *levels.last().expect("a level") != level {
                    return Err(format!("Indentation mismatch at line {}", tok.row));
                }
            }
            Shape::Lead => {}
            // A line end is space inside brackets always, and
            // everywhere in a language whose statements end only where
            // the terminator is written: there an expression may be
            // carried on to the next line, as it may inside brackets.
            Shape::LineEnd if nesting > 0 || lang.terminator_only => {}
            Shape::Sign => {
                if pairs.iter().any(|p| p.open == tok.lexeme) {
                    nesting += 1;
                } else if pairs.iter().any(|p| p.close == tok.lexeme) {
                    nesting = nesting.saturating_sub(1);
                }
                out.push(tok);
            }
            Shape::Finish => {
                while levels.len() > 1 {
                    levels.pop();
                    out.push(boundary(Shape::Close, tok.row));
                }
                out.push(tok);
            }
            _ => out.push(tok),
        }
    }
    Ok(out)
}
