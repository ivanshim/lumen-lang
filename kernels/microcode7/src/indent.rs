// Block structure: indentation becomes open and close tokens; line ends
// inside brackets are dropped.

use crate::scan::{Shape, Token};
use crate::table::{Blocks, Table};

pub fn indent(tokens: Vec<Token>, table: &Table) -> Result<Vec<Token>, String> {
    let by_indent = table.blocks == Blocks::Indented;
    let unit = table.count("block.indent_size").unwrap_or(4);
    let opens: Vec<&str> = ["syntax.group.open", "syntax.call.open", "syntax.array.open"].iter().filter_map(|k| table.single(k)).collect();
    let closes: Vec<&str> = ["syntax.group.close", "syntax.call.close", "syntax.array.close"].iter().filter_map(|k| table.single(k)).collect();
    let mut out = Vec::with_capacity(tokens.len());
    let mut inside = 0usize;
    let mut stack = vec![0usize];
    let mark = |k: Shape, line: u32| Token { shape: k, lexeme: String::new(), span: 0, row: line };
    for t in tokens {
        match t.shape {
            Shape::Lead if by_indent && inside == 0 => {
                if t.span % unit != 0 {
                    return Err(format!("Invalid indentation at line {}", t.row));
                }
                let level = t.span / unit;
                let top = *stack.last().unwrap();
                if level > top {
                    stack.push(level);
                    out.push(mark(Shape::Open, t.row));
                } else {
                    while *stack.last().unwrap() > level {
                        stack.pop();
                        out.push(mark(Shape::Close, t.row));
                    }
                    if *stack.last().unwrap() != level {
                        return Err(format!("Indentation mismatch at line {}", t.row));
                    }
                }
            }
            Shape::Lead => {}
            // A line end is space within brackets always, and
            // throughout a language whose statements end only where the
            // terminator stands: an expression may run on to the next
            // line there, as it may within brackets.
            Shape::LineEnd if inside > 0 || table.flag("ext.stmt.terminator.only") => {}
            Shape::Sign => {
                if opens.contains(&t.lexeme.as_str()) {
                    inside += 1;
                } else if closes.contains(&t.lexeme.as_str()) {
                    inside = inside.saturating_sub(1);
                }
                out.push(t);
            }
            Shape::Finish => {
                while stack.len() > 1 {
                    stack.pop();
                    out.push(mark(Shape::Close, t.row));
                }
                out.push(t);
            }
            _ => out.push(t),
        }
    }
    Ok(out)
}
