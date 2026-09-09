// Shaping: block boundaries. An indented language gets Open and Close
// tokens from its indentation; the other styles carry delimiters in the
// text. Inside brackets a line end is only space.
//
// The brackets are weighed against one another here as well, where the
// language has words for a bracket amiss: this is the one pass that
// sees every bracket in the order it was written, and which bracket was
// opened, and on which line, is the whole of what such a complaint has
// to say. A language that gives no such words is not weighed at all,
// and a bracket amiss shows up later as whatever the reading makes of
// it.

use crate::lang::{Blocks, Lang};
use crate::lex::{Shape, Token};

/// A bracket opened and not yet answered: what was written, what would
/// answer it, and the line it stands on.
struct Waiting {
    open: String,
    close: String,
    row: usize,
}

/// Every pair of brackets a program writes as signs, blocks among them:
/// a block left open is as much a program that cannot be read as a
/// group left open is. Blocks a language marks with words never reach
/// here, a word being no sign.
fn bracket_pairs(lang: &Lang) -> Vec<(String, String)> {
    let mut pairs: Vec<(String, String)> = [&lang.grouping, &lang.calling, &lang.array_brackets, &lang.map_brackets, &lang.index_brackets]
        .into_iter()
        .flatten()
        .map(|b| (b.open.clone(), b.close.clone()))
        .collect();
    for (open, close) in lang.block_opens.iter().zip(&lang.block_closes) {
        pairs.push((open.clone(), close.clone()));
    }
    pairs
}

impl Lang {
    /// The words for a bracket opened and never answered, naming the
    /// line it was opened on where the reading stopped on a later one.
    fn unclosed_said(&self, open: &str, opened_on: usize, stopped_on: usize) -> String {
        let (before, after) = self.unclosed_words.clone().expect("words for a bracket left open");
        let mut said = format!("{before}{open}{after}");
        if let (true, Some(word)) = (opened_on != stopped_on, &self.unclosed_line) {
            said.push_str(&format!(" {word} {opened_on}"));
        }
        said
    }
}

/// The lines a text carries before the one the program was written on,
/// which the mark opening code puts there, are none of the program's:
/// a line the language names is counted from the program's own first.
pub fn layout(tokens: Vec<Token>, lang: &Lang, before: usize) -> Result<Vec<Token>, (String, usize)> {
    let pairs: Vec<&crate::lang::Brackets> = [&lang.grouping, &lang.calling, &lang.array_brackets].into_iter().flatten().collect();
    let weighed = bracket_pairs(lang);
    let watching = lang.unclosed_words.is_some() && lang.unmatched_words.is_some();
    let mut waiting: Vec<Waiting> = Vec::new();
    // Which line the reading has reached: a line end carries the row of
    // the line it closes, so whatever follows one stands on the next.
    let mut reached = 1usize;
    let indented = lang.blocks == Blocks::Indented;
    let mut out = Vec::with_capacity(tokens.len());
    let mut nesting = 0usize;
    let mut levels = vec![0usize];
    let boundary = |shape: Shape, row: usize| Token { shape, lexeme: String::new(), width: 0, row, column: 0 };
    for tok in tokens {
        reached = match tok.shape {
            Shape::LineEnd => tok.row + 1,
            Shape::Finish => reached,
            _ => reached.max(tok.row),
        };
        match tok.shape {
            Shape::Lead if indented && nesting == 0 => {
                if tok.width % lang.indent_width != 0 {
                    return Err((format!("Invalid indentation at line {}", tok.row), tok.row));
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
                    return Err((format!("Indentation mismatch at line {}", tok.row), tok.row));
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
                if watching {
                    if let Some((open, close)) = weighed.iter().find(|(open, _)| *open == tok.lexeme) {
                        waiting.push(Waiting { open: open.clone(), close: close.clone(), row: tok.row });
                    } else if weighed.iter().any(|(_, close)| *close == tok.lexeme) {
                        match waiting.pop() {
                            // A bracket answering one that was never
                            // opened stands alone and is said so.
                            None => {
                                let (head, tail) = lang.unmatched_words.clone().expect("words for a bracket answering none");
                                return Err((format!("{head}{}{tail}", tok.lexeme), tok.row));
                            }
                            // One answering a bracket of another kind
                            // names the bracket it found instead.
                            Some(held) if held.close != tok.lexeme => {
                                let mut said = lang.unclosed_said(&held.open, held.row.saturating_sub(before), tok.row.saturating_sub(before));
                                if let Some((head, tail)) = &lang.mismatch_words {
                                    said.push_str(&format!(" {head}{}{tail}", tok.lexeme));
                                }
                                return Err((said, tok.row));
                            }
                            Some(_) => {}
                        }
                    }
                }
                out.push(tok);
            }
            Shape::Finish => {
                // The innermost bracket still waiting is the one told
                // of: it is the one the reading was inside when the
                // text ran out.
                if let (true, Some(held)) = (watching, waiting.last()) {
                    return Err((lang.unclosed_said(&held.open, held.row.saturating_sub(before), reached.saturating_sub(before)), reached));
                }
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
