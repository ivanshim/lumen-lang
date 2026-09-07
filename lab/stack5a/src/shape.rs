// Shaping: block boundaries. An indented language gets Open and Close
// tokens from its indentation; the other styles carry delimiters in the
// text. Inside brackets a line end is only space.

use crate::language::{Def, Style};
use crate::scan::{Kind, Tok};

pub fn shape(toks: Vec<Tok>, def: &Def) -> Result<Vec<Tok>, String> {
    let pairs: Vec<&crate::language::Pair> = [&def.group, &def.call, &def.array].into_iter().flatten().collect();
    let indented = def.style == Style::Indented;
    let mut out = Vec::with_capacity(toks.len());
    let mut nesting = 0usize;
    let mut levels = vec![0usize];
    let boundary = |kind: Kind, line: usize| Tok { kind, text: String::new(), width: 0, line, col: 0 };
    for tok in toks {
        match tok.kind {
            Kind::Indent if indented && nesting == 0 => {
                if tok.width % def.indent != 0 {
                    return Err(format!("Invalid indentation at line {}", tok.line));
                }
                let level = tok.width / def.indent;
                let current = *levels.last().expect("a level");
                if level > current {
                    levels.push(level);
                    out.push(boundary(Kind::Open, tok.line));
                    continue;
                }
                while *levels.last().expect("a level") > level {
                    levels.pop();
                    out.push(boundary(Kind::Close, tok.line));
                }
                if *levels.last().expect("a level") != level {
                    return Err(format!("Indentation mismatch at line {}", tok.line));
                }
            }
            Kind::Indent => {}
            Kind::Eol if nesting > 0 => {}
            Kind::Symbol => {
                if pairs.iter().any(|p| p.open == tok.text) {
                    nesting += 1;
                } else if pairs.iter().any(|p| p.close == tok.text) {
                    nesting = nesting.saturating_sub(1);
                }
                out.push(tok);
            }
            Kind::End => {
                while levels.len() > 1 {
                    levels.pop();
                    out.push(boundary(Kind::Close, tok.line));
                }
                out.push(tok);
            }
            _ => out.push(tok),
        }
    }
    Ok(out)
}
