// Layout: where blocks begin and end.
//
// An indented language gets Open and Close tokens from the changes in
// indentation; the other styles carry their delimiters in the text.
// Inside brackets a line end is just space, so calls, groups and array
// literals may run over several lines.

use crate::definition::{Language, Layout};
use crate::lexer::{Tk, Token};

pub fn shape(tokens: Vec<Token>, lang: &Language) -> Result<Vec<Token>, String> {
    let brackets: Vec<&crate::definition::Brackets> = [&lang.group, &lang.call, &lang.array].into_iter().flatten().collect();
    let opens = |t: &str| brackets.iter().any(|b| b.open == t);
    let closes = |t: &str| brackets.iter().any(|b| b.close == t);
    let indented = lang.layout == Layout::Indented;
    let mut out = Vec::with_capacity(tokens.len());
    let mut depth = 0usize;
    let mut levels = vec![0usize];
    let mark = |kind: Tk, line: usize| Token { kind, text: String::new(), width: 0, line, col: 0 };

    for tok in tokens {
        match tok.kind {
            Tk::Indent => {
                if !indented || depth > 0 {
                    continue;
                }
                if tok.width % lang.indent != 0 {
                    return Err(format!("Invalid indentation at line {}", tok.line));
                }
                let level = tok.width / lang.indent;
                let current = *levels.last().unwrap();
                if level > current {
                    levels.push(level);
                    out.push(mark(Tk::Open, tok.line));
                } else {
                    while *levels.last().unwrap() > level {
                        levels.pop();
                        out.push(mark(Tk::Close, tok.line));
                    }
                    if *levels.last().unwrap() != level {
                        return Err(format!("Indentation mismatch at line {}", tok.line));
                    }
                }
            }
            Tk::Eol => {
                if depth == 0 {
                    out.push(tok);
                }
            }
            Tk::Symbol => {
                if opens(&tok.text) {
                    depth += 1;
                } else if closes(&tok.text) {
                    depth = depth.saturating_sub(1);
                }
                out.push(tok);
            }
            Tk::End => {
                while levels.len() > 1 {
                    levels.pop();
                    out.push(mark(Tk::Close, tok.line));
                }
                out.push(tok);
            }
            _ => out.push(tok),
        }
    }
    Ok(out)
}
