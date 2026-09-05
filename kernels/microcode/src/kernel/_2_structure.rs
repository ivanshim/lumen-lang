// Stage 2: Structure — tokens → tokens with explicit block delimiters.
//
// Indentation-based languages get the schema's block delimiters synthesised
// from indentation changes; brace-based languages already carry them. Line
// ends and indentation inside brackets are dropped so grouping, calls and
// array literals may span lines. A block-introducing token that ends a
// header line (Python's `:`) is removed.

use super::_1_ingest::{Kind, Token};
use crate::schema::{BlockStyle, LanguageSchema};

pub fn process(tokens: Vec<Token>, schema: &LanguageSchema) -> Result<Vec<Token>, String> {
    let structure = &schema.structure;
    let opens: Vec<&str> = [&structure.group, &structure.call, &structure.array]
        .iter()
        .filter_map(|p| p.as_ref().map(|p| p.open.as_str()))
        .collect();
    let closes: Vec<&str> = [&structure.group, &structure.call, &structure.array]
        .iter()
        .filter_map(|p| p.as_ref().map(|p| p.close.as_str()))
        .collect();

    let mut out: Vec<Token> = Vec::with_capacity(tokens.len());
    let mut depth: usize = 0;
    let mut levels: Vec<usize> = vec![0];

    let mut i = 0;
    while i < tokens.len() {
        let tok = &tokens[i];
        match tok.kind {
            Kind::Indent => {
                if depth == 0 && structure.blocks == BlockStyle::Indentation {
                    if structure.indent_size == 0 || tok.width % structure.indent_size != 0 {
                        return Err(format!("Invalid indentation at line {}", tok.line));
                    }
                    let level = tok.width / structure.indent_size;
                    let current = *levels.last().unwrap();
                    if level > current {
                        levels.push(level);
                        out.push(synthetic(&structure.block_open, tok.line));
                    } else if level < current {
                        while *levels.last().unwrap() > level {
                            levels.pop();
                            out.push(synthetic(&structure.block_close, tok.line));
                        }
                        if *levels.last().unwrap() != level {
                            return Err(format!("Indentation mismatch at line {}", tok.line));
                        }
                    }
                }
            }
            Kind::Newline => {
                if depth == 0 {
                    out.push(tok.clone());
                }
            }
            Kind::Op => {
                let text = tok.text.as_str();
                if opens.contains(&text) {
                    depth += 1;
                } else if closes.contains(&text) {
                    depth = depth.saturating_sub(1);
                }
                let ends_header = structure.block_intro.as_deref() == Some(text)
                    && tokens.get(i + 1).map_or(false, |next| next.kind == Kind::Newline || next.kind == Kind::Eof);
                if !ends_header {
                    out.push(tok.clone());
                }
            }
            Kind::Eof => {
                while levels.len() > 1 {
                    levels.pop();
                    out.push(synthetic(&structure.block_close, tok.line));
                }
                out.push(tok.clone());
            }
            _ => out.push(tok.clone()),
        }
        i += 1;
    }
    Ok(out)
}

fn synthetic(text: &str, line: usize) -> Token {
    Token { kind: Kind::Op, text: text.to_string(), width: 0, line, col: 0 }
}
