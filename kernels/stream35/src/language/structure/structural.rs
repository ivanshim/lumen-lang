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
use crate::language::prelude::LumenParserExt;
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
        || (def().block_style != BlockStyle::Indentation && def().is("block.close", lexeme))
}

/// Whether the current token is one of `stops`.
fn at_one_of(parser: &Parser, stops: &[String]) -> bool {
    stops.iter().any(|s| *s == parser.peek().lexeme)
}

/// Parse statements up to a token in `stops` (not consumed) or the end.
pub fn parse_body(parser: &mut Parser, registry: &Registry, stops: &[String]) -> LumenResult<Vec<Box<dyn StmtNode>>> {
    let mut stmts = Vec::new();
    consume_separators(parser);
    while !at_one_of(parser, stops) && parser.peek().lexeme != EOF {
        let s = registry
            .find_stmt(parser)
            .ok_or_else(|| err_at(parser, "Unknown statement in block"))?
            .parse(parser, registry)?;
        stmts.push(s);
        consume_separators(parser);
    }
    Ok(stmts)
}

/// Drop a block-intro token (Python's `:`, Lua's `then`) if present.
pub fn skip_block_intro(parser: &mut Parser) {
    if def().is("block.intro", &parser.peek().lexeme) {
        parser.advance();
    }
}

/// Consume any of the definition's block closers.
pub fn expect_close(parser: &mut Parser) -> LumenResult<()> {
    if def().is("block.close", &parser.peek().lexeme) {
        parser.advance();
        Ok(())
    } else {
        Err(err_at(parser, &format!("Expected '{}' to close a block", def().first("block.close"))))
    }
}

/// Consume line ends and statement terminators between statements, and
/// the spaces a postfix language keeps between its words.
pub fn consume_separators(parser: &mut Parser) {
    loop {
        parser.skip_tokens();
        let lexeme = &parser.peek().lexeme;
        if lexeme == NEWLINE || def().is("stmt.terminator", lexeme) {
            parser.advance();
        } else {
            break;
        }
    }
}

/// Parse a block after a statement header. An optional block-intro token
/// (Python's `:`, Lua's `then`) is dropped first. Then, as the definition
/// says: an indented run of statements; a bracketed one, where the opener
/// at position i pairs with the closer at position i; or, for the keyword
/// style, a run of statements up to any closer.
pub fn parse_block(parser: &mut Parser, registry: &Registry) -> LumenResult<Vec<Box<dyn StmtNode>>> {
    let d = def();
    skip_block_intro(parser);
    consume_separators(parser);

    let (open, close): (String, String) = match d.block_style {
        BlockStyle::Indentation => (INDENT.to_string(), DEDENT.to_string()),
        BlockStyle::Braces => {
            let opens = d.list("block.open");
            let i = opens
                .iter()
                .position(|o| *o == parser.peek().lexeme)
                .ok_or_else(|| err_at(parser, &format!("Expected '{}' to open a block", opens[0])))?;
            (opens[i].clone(), d.list("block.close")[i].clone())
        }
        BlockStyle::Keyword | BlockStyle::Postfix => {
            let stmts = parse_body(parser, registry, d.list("block.close"))?;
            expect_close(parser)?;
            return Ok(stmts);
        }
    };

    if parser.advance().lexeme != open {
        return Err(err_at(parser, &format!("Expected '{}' to open a block", open)));
    }
    let stmts = parse_body(parser, registry, std::slice::from_ref(&close))?;
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
    let quotes = d.chars("lexical.string_quotes");
    let wordy = |t: &SpannedToken| !t.tok.lexeme.is_empty() && t.tok.lexeme.chars().all(word_char);

    let mut out = Vec::with_capacity(tokens.len());
    let mut i = 0;
    while i < tokens.len() {
        // A string is copied through untouched, up to its closing quote.
        if let Some(quote) = single(&tokens[i].tok.lexeme).filter(|c| quotes.contains(c)) {
            out.push(tokens[i].clone());
            i += 1;
            let mut escaped = false;
            while i < tokens.len() {
                let lexeme = &tokens[i].tok.lexeme;
                out.push(tokens[i].clone());
                i += 1;
                if escaped {
                    escaped = false;
                } else if lexeme == "\\" {
                    escaped = true;
                } else if single(lexeme) == Some(quote) {
                    break;
                }
            }
            continue;
        }
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
/// outside strings. Each line ends with NEWLINE, which separates statements
/// in every block style (Swift writes no semicolons), except inside array
/// brackets and parentheses, where line ends are whitespace. For an
/// indentation language, INDENT and DEDENT tokens are synthesised from
/// indentation changes at bracket depth zero; for the other styles the
/// block delimiters stay as they are.
pub fn process(source: &str, raw_tokens: Vec<SpannedToken>) -> LumenResult<Vec<SpannedToken>> {
    let d = def();
    let raw_tokens = fold_case(raw_tokens);
    let indentation = d.block_style == BlockStyle::Indentation;
    let postfix = d.block_style == BlockStyle::Postfix;
    let indent_size = d.indent_size;
    let comment_markers = d.list("lexical.comment_line");
    let block_opens = d.list("lexical.comment_block.open");
    let block_closes = d.list("lexical.comment_block.close");
    let prologue = d.list("lexical.prologue").first();
    let quotes = d.chars("lexical.string_quotes");

    let mut out = Vec::new();
    let mut indents = vec![0usize];
    let mut line_no = 1usize;
    let mut bracket_depth_global = 0i32; // brackets and parentheses suspend line structure
    let opens_bracket = |lexeme: &str| {
        d.is("syntax.array.open", lexeme) || d.is("syntax.group.open", lexeme) || d.is("syntax.call.open", lexeme)
    };
    let closes_bracket = |lexeme: &str| {
        d.is("syntax.array.close", lexeme) || d.is("syntax.group.close", lexeme) || d.is("syntax.call.close", lexeme)
    };
    // Which comment pair is open, if any; its closer ends it.
    let mut in_block_comment: Option<usize> = None;
    let mut seen_code = false;

    for raw in source.lines() {
        // Count leading spaces
        let spaces = raw.chars().take_while(|&c| c == ' ').count();
        let rest = &raw[spaces..];

        // Blank lines and comment-only lines contribute nothing.
        let opens_block = block_opens.iter().any(|o| rest.starts_with(o.as_str()));
        let comment_only = !opens_block && comment_markers.iter().any(|m| rest.starts_with(m.as_str()));
        if in_block_comment.is_none() && (rest.trim().is_empty() || comment_only) {
            line_no += 1;
            continue;
        }

        // Indentation handling, at bracket depth zero, for indentation languages
        if indentation && bracket_depth_global == 0 && in_block_comment.is_none() {
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
            if let Some(pair) = in_block_comment {
                if *lexeme == block_closes[pair] {
                    in_block_comment = None;
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
            if let Some(pair) = block_opens.iter().position(|o| o == lexeme) {
                in_block_comment = Some(pair);
                continue;
            }

            if opens_bracket(lexeme) {
                bracket_depth_line += 1;
                bracket_depth_global += 1;
                out.push(raw_tok.clone());
            } else if closes_bracket(lexeme) {
                bracket_depth_line -= 1;
                bracket_depth_global -= 1;
                out.push(raw_tok.clone());
            } else if one.map_or(false, |c| quotes.contains(&c)) {
                in_string = one;
                out.push(raw_tok.clone());
            } else if bracket_depth_line > 0 {
                // Inside brackets: line ends are whitespace
                if one == Some('\n') || one == Some('\r') {
                    continue;
                }
                out.push(raw_tok.clone());
            } else {
                // Outside strings and arrays: whitespace is dropped, except
                // in a postfix language, where a space separates two words
                // (`5 3` is two numbers) and stays for the parser to skip.
                if matches!(one, Some('\n') | Some('\r')) || (!postfix && matches!(one, Some(' ') | Some('\t'))) {
                    continue;
                }
                out.push(raw_tok.clone());
            }
        }

        // Every line ends with NEWLINE, outside brackets and block comments.
        if bracket_depth_global == 0 && in_block_comment.is_none() {
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
