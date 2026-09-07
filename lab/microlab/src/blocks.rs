// Block structure: indentation becomes open and close tokens; line ends
// inside brackets are dropped.

use crate::lexer::{Tk, Tok};
use crate::spec::{Layout, Spec};

pub fn shape(toks: Vec<Tok>, spec: &Spec) -> Result<Vec<Tok>, String> {
    let by_indent = spec.layout == Layout::Indent;
    let unit = spec.number("block.indent_size").unwrap_or(4);
    let opens: Vec<&str> = ["syntax.group.open", "syntax.call.open", "syntax.array.open"].iter().filter_map(|k| spec.one(k)).collect();
    let closes: Vec<&str> = ["syntax.group.close", "syntax.call.close", "syntax.array.close"].iter().filter_map(|k| spec.one(k)).collect();
    let mut out = Vec::with_capacity(toks.len());
    let mut inside = 0usize;
    let mut stack = vec![0usize];
    let mark = |k: Tk, line: u32| Tok { kind: k, text: String::new(), width: 0, line };
    for t in toks {
        match t.kind {
            Tk::Indent if by_indent && inside == 0 => {
                if t.width % unit != 0 {
                    return Err(format!("Invalid indentation at line {}", t.line));
                }
                let level = t.width / unit;
                let top = *stack.last().unwrap();
                if level > top {
                    stack.push(level);
                    out.push(mark(Tk::BlockOpen, t.line));
                } else {
                    while *stack.last().unwrap() > level {
                        stack.pop();
                        out.push(mark(Tk::BlockClose, t.line));
                    }
                    if *stack.last().unwrap() != level {
                        return Err(format!("Indentation mismatch at line {}", t.line));
                    }
                }
            }
            Tk::Indent => {}
            Tk::Eol if inside > 0 => {}
            Tk::Sym => {
                if opens.contains(&t.text.as_str()) {
                    inside += 1;
                } else if closes.contains(&t.text.as_str()) {
                    inside = inside.saturating_sub(1);
                }
                out.push(t);
            }
            Tk::End => {
                while stack.len() > 1 {
                    stack.pop();
                    out.push(mark(Tk::BlockClose, t.line));
                }
                out.push(t);
            }
            _ => out.push(t),
        }
    }
    Ok(out)
}
