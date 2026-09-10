// Block structure: indentation becomes open and close tokens; line ends
// inside brackets are dropped.
//
// Brackets are also held to account against one another here, where the
// language gives words for one amiss. Nowhere else are all the brackets
// seen in the order they were written, and a complaint worth reading
// wants just that: which bracket was opened, and where. Say nothing in
// the definition and nothing is weighed, so the nine other languages go
// on being read exactly as before.

use crate::scan::{Shape, Token};
use crate::table::{Blocks, Table};

/// The lexemes that open and shut, paired. Blocks count among them when
/// a language marks them with signs: a block never shut is no more
/// readable than a group never shut. Word-marked blocks are words, and
/// words never come through here as signs.
fn paired(table: &Table) -> Vec<(String, String)> {
    let stems = ["syntax.group", "syntax.call", "syntax.array", "syntax.map", "op.index", "block"];
    let mut both = Vec::new();
    for stem in stems {
        let ends = match stem {
            "block" => ("block.open".to_string(), "block.close".to_string()),
            _ => (format!("{stem}.open"), format!("{stem}.close")),
        };
        if let (Some(a), Some(z)) = (table.single(&ends.0), table.single(&ends.1)) {
            both.push((a.to_string(), z.to_string()));
        }
    }
    both
}

/// What the language says of a bracket opened and never shut. Its line
/// is named only when the reading gave up on some other line, there
/// being no sense in naming the line twice over.
fn never_shut(table: &Table, opener: &str, from: u32, upto: u32) -> String {
    let (head, tail) = table.around("ext.system.reading.unclosed").expect("words for a bracket never shut");
    let named = match (from == upto, table.single("ext.system.reading.unclosed.line")) {
        (false, Some(word)) => format!(" {word} {from}"),
        _ => String::new(),
    };
    format!("{head}{opener}{tail}{named}")
}

/// The line count a text carries ahead of the program itself, put there
/// by the mark that opens code, belongs to no line the language names.
pub fn indent(tokens: Vec<Token>, table: &Table, ahead: u32) -> Result<Vec<Token>, (String, u32)> {
    let mut tokens = tokens;
    tokens.retain(|part| !matches!(part.shape, Shape::Caution));
    let by_indent = table.blocks == Blocks::Indented;
    let unit = table.count("block.indent_size").unwrap_or(4);
    let mut opens: Vec<&str> = ["syntax.group.open", "syntax.call.open", "syntax.array.open"].iter().filter_map(|k| table.single(k)).collect();
    let mut closes: Vec<&str> = ["syntax.group.close", "syntax.call.close", "syntax.array.close"].iter().filter_map(|k| table.single(k)).collect();
    if table.flag("ext.syntax.set") {
        if let (Some(open), Some(close)) = (table.single("syntax.map.open"), table.single("syntax.map.close")) {
            opens.push(open);
            closes.push(close);
        }
    }
    let ends = paired(table);
    let held = table.around("ext.system.reading.unclosed").is_some() && table.around("ext.system.reading.unmatched").is_some();
    // Each bracket still owed an answer: what opened it, what would
    // answer it, and where it stands.
    let mut owed: Vec<(&str, &str, u32)> = Vec::new();
    // The line the text has got to. A line end belongs to the line it
    // closes, so anything past one belongs to the line after.
    let mut got_to = 1u32;
    let mut out = Vec::with_capacity(tokens.len());
    let mut inside = 0usize;
    let mut stack = vec![0usize];
    let mark = |k: Shape, line: u32| Token { shape: k, lexeme: String::new(), span: 0, row: line };
    for t in tokens {
        if t.shape == Shape::LineEnd {
            got_to = t.row + 1;
        } else if t.shape != Shape::Finish {
            got_to = got_to.max(t.row);
        }
        match t.shape {
            Shape::Lead if by_indent && inside == 0 => {
                if t.span % unit != 0 {
                    return Err((format!("Invalid indentation at line {}", t.row), t.row));
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
                        return Err((format!("Indentation mismatch at line {}", t.row), t.row));
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
                let opener = ends.iter().find(|(a, _)| *a == t.lexeme).map(|(a, z)| (a.as_str(), z.as_str()));
                let shutter = ends.iter().any(|(_, z)| *z == t.lexeme);
                match (held, opener, shutter) {
                    (true, Some((a, z)), _) => owed.push((a, z, t.row)),
                    (true, None, true) => match owed.pop() {
                        // Nothing was opened for this one to answer.
                        None => {
                            let (head, tail) = table.around("ext.system.reading.unmatched").expect("words for a bracket answering none");
                            return Err((format!("{head}{}{tail}", t.lexeme), t.row));
                        }
                        // It answers, but not the bracket it found.
                        Some((a, z, on)) if z != t.lexeme => {
                            let mut said = never_shut(table, a, on.saturating_sub(ahead), t.row.saturating_sub(ahead));
                            if let Some((head, tail)) = table.around("ext.system.reading.unclosed.mismatch") {
                                said = format!("{said} {head}{}{tail}", t.lexeme);
                            }
                            return Err((said, t.row));
                        }
                        Some(_) => {}
                    },
                    _ => {}
                }
                out.push(t);
            }
            Shape::Finish => {
                // Of the brackets still owed answers, the innermost is
                // the one to tell of: the reading was within it when
                // the text gave out.
                if let (true, Some(&(a, _, on))) = (held, owed.last()) {
                    let said = never_shut(table, a, on.saturating_sub(ahead), got_to.saturating_sub(ahead));
                    return Err((said, got_to));
                }
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
