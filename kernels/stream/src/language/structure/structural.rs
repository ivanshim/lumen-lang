// language/structure/structural.rs
//
// Structure the stream way: token-stream transformations. The kernel lexer
// leaves one token per registered lexeme or character; this pass folds
// case, drops the prologue and comments, and either synthesises INDENT and
// DEDENT tokens from indentation changes or leaves the definition's block
// delimiters in place. Which of those applies, and every marker involved,
// comes from the definition. The structural token names are this module's own.

use crate::kernel::ast::{Program, StmtNode};
use crate::kernel::lexer::{Span, SpannedToken, Token};
use crate::kernel::parser::Parser;
use crate::kernel::registry::{err_at, KernelResult as LumenResult};
use crate::language::definition::{def, BlockStyle};
use crate::language::registry::Registry;
use crate::language::{word_char, word_start};

// --------------------
// Structural tokens (synthesised; never lexemes of the language)
// --------------------

pub const NEWLINE: &str = "NEWLINE";
pub const INDENT: &str = "INDENT";
pub const DEDENT: &str = "DEDENT";
pub const EOF: &str = "EOF";

// --------------------
// Parsing helpers
// --------------------

/// Whether the current token ends a statement: a line end, a terminator,
/// a block close or the end of the file.
pub fn at_statement_end(parser: &Parser) -> bool {
    let lexeme = &parser.peek().lexeme;
    matches!(lexeme.as_str(), NEWLINE | DEDENT | EOF)
        || def().is("stmt.terminator", lexeme)
        || (def().block_style == BlockStyle::Braces && def().is("block.close", lexeme))
}

/// Consume line ends and statement terminators between statements.
pub fn consume_separators(parser: &mut Parser) {
    loop {
        let lexeme = &parser.peek().lexeme;
        if lexeme == NEWLINE || def().is("stmt.terminator", lexeme) {
            parser.advance();
        } else {
            break;
        }
    }
}

/// Parse a block after a statement header. An optional block-intro token
/// (Python's `:`) is dropped first; then either an indented run of
/// statements or a bracketed one, as the definition says.
pub fn parse_block(parser: &mut Parser, registry: &Registry) -> LumenResult<Vec<Box<dyn StmtNode>>> {
    let d = def();
    if d.is("block.intro", &parser.peek().lexeme) {
        parser.advance();
    }
    consume_separators(parser);

    let (open, close): (String, String) = match d.block_style {
        BlockStyle::Indentation => (INDENT.to_string(), DEDENT.to_string()),
        BlockStyle::Braces => (d.first("block.open").to_string(), d.first("block.close").to_string()),
    };

    if parser.advance().lexeme != open {
        return Err(err_at(parser, &format!("Expected '{}' to open a block", open)));
    }
    consume_separators(parser);

    let mut stmts = Vec::new();
    while parser.peek().lexeme != close && parser.peek().lexeme != EOF {
        let s = registry
            .find_stmt(parser)
            .ok_or_else(|| err_at(parser, "Unknown statement in block"))?
            .parse(parser, registry)?;
        stmts.push(s);
        consume_separators(parser);
    }

    if parser.advance().lexeme != close {
        return Err(err_at(parser, &format!("Expected '{}' to close a block", close)));
    }
    Ok(stmts)
}

/// Parse a whole program: statements separated by line ends and terminators.
pub fn parse_program(parser: &mut Parser, registry: &Registry) -> LumenResult<Program> {
    let mut stmts = Vec::new();
    consume_separators(parser);

    while parser.peek().lexeme != EOF {
        let stmt = registry
            .find_stmt(parser)
            .ok_or_else(|| err_at(parser, "Unknown statement"))?
            .parse(parser, registry)?;
        stmts.push(stmt);
        consume_separators(parser);
    }

    Ok(Program::new(stmts))
}

// --------------------
// Token-stream transformations
// --------------------

fn single(lexeme: &str) -> Option<char> {
    let mut chars = lexeme.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) => Some(c),
        _ => None,
    }
}

fn synthetic(lexeme: &str, line: usize, col: usize) -> SpannedToken {
    SpannedToken { tok: Token::new(lexeme.to_string(), Span::new(0, 0)), line, col }
}

/// Fold letter case where the definition asks for it. Words arrive as runs
/// of single-character tokens (or one reserved-word token). A run that
/// spells a reserved word in another case becomes that reserved word; other
/// runs are lowercased when identifiers are case-insensitive.
fn fold_case(tokens: Vec<SpannedToken>) -> Vec<SpannedToken> {
    let d = def();
    if !d.keywords_case_insensitive && !d.identifiers_case_insensitive {
        return tokens;
    }
    let reserved = d.reserved_words();
    let wordy = |t: &SpannedToken| !t.tok.lexeme.is_empty() && t.tok.lexeme.chars().all(word_char);

    let mut out = Vec::with_capacity(tokens.len());
    let mut i = 0;
    while i < tokens.len() {
        if !wordy(&tokens[i]) {
            out.push(tokens[i].clone());
            i += 1;
            continue;
        }
        let mut j = i;
        while j < tokens.len() && wordy(&tokens[j]) {
            j += 1;
        }
        let run = &tokens[i..j];
        let text: String = run.iter().map(|t| t.tok.lexeme.as_str()).collect();
        let lowered = text.to_lowercase();
        let starts_word = text.chars().next().map_or(false, word_start);
        if d.keywords_case_insensitive && starts_word && text != lowered && reserved.iter().any(|w| *w == lowered) {
            let first = &run[0];
            let end = run[run.len() - 1].tok.span.end;
            out.push(SpannedToken {
                tok: Token::new(lowered, Span::new(first.tok.span.start, end)),
                line: first.line,
                col: first.col,
            });
        } else if d.identifiers_case_insensitive && starts_word && !reserved.iter().any(|w| *w == text) {
            for t in run {
                let mut folded = t.clone();
                folded.tok.lexeme = folded.tok.lexeme.to_lowercase();
                out.push(folded);
            }
        } else {
            out.extend(run.iter().cloned());
        }
        i = j;
    }
    out
}

/// Turn the lexer's tokens into the stream the parser reads.
///
/// Comments are dropped from the marker to the end of the line, or from a
/// block opener to its closer, outside strings. Whitespace is dropped
/// outside strings. For an indentation language, INDENT and DEDENT tokens
/// are synthesised from indentation changes at bracket depth zero and each
/// line ends with NEWLINE; for a brace language the block delimiters stay
/// as they are and line ends are insignificant, so no NEWLINE is emitted.
pub fn process(source: &str, raw_tokens: Vec<SpannedToken>) -> LumenResult<Vec<SpannedToken>> {
    let d = def();
    let raw_tokens = fold_case(raw_tokens);
    let indentation = d.block_style == BlockStyle::Indentation;
    let indent_size = d.indent_size;
    let comment_markers = d.list("lexical.comment_line");
    let block_open = d.list("lexical.comment_block.open").first();
    let block_close = d.list("lexical.comment_block.close").first();
    let prologue = d.list("lexical.prologue").first();
    let quotes = d.chars("lexical.string_quotes");

    let mut out = Vec::new();
    let mut indents = vec![0usize];
    let mut line_no = 1usize;
    let mut bracket_depth_global = 0i32; // array brackets suspend line structure
    let mut in_block_comment = false;
    let mut seen_code = false;

    for raw in source.lines() {
        // Count leading spaces
        let spaces = raw.chars().take_while(|&c| c == ' ').count();
        let rest = &raw[spaces..];

        // Blank lines and comment-only lines contribute nothing.
        let comment_only = comment_markers.iter().any(|m| rest.starts_with(m.as_str()));
        if in_block_comment || rest.trim().is_empty() || comment_only {
            if in_block_comment {
                // The block may close on this line; the token loop below handles it.
            } else {
                line_no += 1;
                continue;
            }
        }

        // Indentation handling, at bracket depth zero, for indentation languages
        if indentation && bracket_depth_global == 0 && !in_block_comment {
            let current = *indents.last().unwrap();
            if spaces > current {
                if (spaces - current) % indent_size != 0 {
                    return Err(format!("Invalid indentation at line {line_no}"));
                }
                indents.push(spaces);
                out.push(synthetic(INDENT, line_no, 1));
            } else if spaces < current {
                while *indents.last().unwrap() > spaces {
                    indents.pop();
                    out.push(synthetic(DEDENT, line_no, 1));
                }
                if *indents.last().unwrap() != spaces {
                    return Err(format!("Indentation mismatch at line {line_no}"));
                }
            }
        }

        let mut in_string: Option<char> = None;
        let mut escaped = false;
        let mut in_line_comment = false;
        let mut bracket_depth_line = bracket_depth_global;

        for raw_tok in raw_tokens.iter().filter(|t| t.line == line_no) {
            let lexeme = &raw_tok.tok.lexeme;
            let one = single(lexeme);

            if in_line_comment {
                continue;
            }
            if in_block_comment {
                if Some(lexeme) == block_close {
                    in_block_comment = false;
                }
                continue;
            }

            if let Some(quote) = in_string {
                // Inside a string: keep everything, including whitespace
                out.push(raw_tok.clone());
                if escaped {
                    escaped = false;
                } else if one == Some('\\') {
                    escaped = true;
                } else if one == Some(quote) {
                    in_string = None;
                }
                continue;
            }

            // The prologue, if the file opens with it, is dropped
            if !seen_code {
                if Some(lexeme) == prologue {
                    continue;
                }
                if !lexeme.trim().is_empty() {
                    seen_code = true;
                }
            }

            if comment_markers.iter().any(|m| m == lexeme) {
                in_line_comment = true;
                continue;
            }
            if Some(lexeme) == block_open {
                in_block_comment = true;
                continue;
            }

            if d.is("syntax.array.open", lexeme) {
                bracket_depth_line += 1;
                bracket_depth_global += 1;
                out.push(raw_tok.clone());
            } else if d.is("syntax.array.close", lexeme) {
                bracket_depth_line -= 1;
                bracket_depth_global -= 1;
                out.push(raw_tok.clone());
            } else if one.map_or(false, |c| quotes.contains(&c)) {
                in_string = one;
                out.push(raw_tok.clone());
            } else if bracket_depth_line > 0 {
                // Inside an array literal: line ends are whitespace
                if one == Some('\n') || one == Some('\r') {
                    continue;
                }
                out.push(raw_tok.clone());
            } else {
                // Outside strings and arrays: whitespace is dropped
                if matches!(one, Some(' ') | Some('\t') | Some('\n') | Some('\r')) {
                    continue;
                }
                out.push(raw_tok.clone());
            }
        }

        // An indentation language ends each line with NEWLINE, outside arrays
        if indentation && bracket_depth_global == 0 && !in_block_comment {
            out.push(synthetic(NEWLINE, line_no, spaces + rest.len() + 1));
        }

        line_no += 1;
    }

    // Close any open blocks
    while indents.len() > 1 {
        indents.pop();
        out.push(synthetic(DEDENT, line_no, 1));
    }

    out.push(synthetic(EOF, line_no, 1));
    Ok(out)
}

pub fn register(_reg: &mut Registry) {
    // No token registration needed; the dispatcher registers every lexeme
    // and this pass synthesises the structural tokens.
}
