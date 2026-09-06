// Stage 2, structure: where blocks begin and end.
//
// An indented language gets Open and Close tokens from its indentation;
// the others carry their delimiters as words or symbols. Inside brackets,
// line ends are dropped.

use crate::ingest::{Kind, Token};
use crate::spec::{Spec, Style};

pub fn blocks(tokens: Vec<Token>, spec: &Spec) -> Result<Vec<Token>, String> {
    let indented = spec.style == Style::Indent;
    let unit = spec.count("block.indent_size").unwrap_or(4);
    let opens: Vec<&str> = ["syntax.group.open", "syntax.call.open", "syntax.array.open"]
        .iter()
        .filter_map(|l| spec.first(l))
        .collect();
    let closes: Vec<&str> = ["syntax.group.close", "syntax.call.close", "syntax.array.close"]
        .iter()
        .filter_map(|l| spec.first(l))
        .collect();
    let mut out = Vec::with_capacity(tokens.len());
    let mut nesting = 0usize;
    let mut levels = vec![0usize];
    let marker = |kind: Kind, line: u32| Token { kind, text: String::new(), width: 0, line };
    for tok in tokens {
        match tok.kind {
            Kind::Indent if indented && nesting == 0 => {
                if tok.width % unit != 0 {
                    return Err(format!("Invalid indentation at line {}", tok.line));
                }
                let level = tok.width / unit;
                let current = *levels.last().unwrap();
                if level > current {
                    levels.push(level);
                    out.push(marker(Kind::Open, tok.line));
                } else {
                    while *levels.last().unwrap() > level {
                        levels.pop();
                        out.push(marker(Kind::Close, tok.line));
                    }
                    if *levels.last().unwrap() != level {
                        return Err(format!("Indentation mismatch at line {}", tok.line));
                    }
                }
            }
            Kind::Indent => {}
            Kind::Newline if nesting > 0 => {}
            Kind::Symbol => {
                if opens.contains(&tok.text.as_str()) {
                    nesting += 1;
                } else if closes.contains(&tok.text.as_str()) {
                    nesting = nesting.saturating_sub(1);
                }
                out.push(tok);
            }
            Kind::End => {
                while levels.len() > 1 {
                    levels.pop();
                    out.push(marker(Kind::Close, tok.line));
                }
                out.push(tok);
            }
            _ => out.push(tok),
        }
    }
    Ok(out)
}
