// Stage 3: Reduction
//
// Convert token stream to instruction tree per schema.
// Schema tables guide all parsing decisions.
// No semantic assumptions.

use super::ingest::Token;
use super::primitives::Instruction;
use crate::schema::LanguageSchema;

/// Reduce token stream to instruction tree per schema
pub fn parse(tokens: &[Token], schema: &LanguageSchema) -> Result<Instruction, String> {
    let mut parser = Parser {
        tokens,
        pos: 0,
        schema,
    };

    parser.parse_program()
}

struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
    schema: &'a LanguageSchema,
}

impl<'a> Parser<'a> {
    fn parse_program(&mut self) -> Result<Instruction, String> {
        let mut instructions = Vec::new();

        while !self.is_at_end() && self.peek().lexeme != "EOF" {
            // Skip whitespace
            self.skip_whitespace();

            if self.is_at_end() || self.peek().lexeme == "EOF" {
                break;
            }

            let instr = self.parse_statement()?;
            instructions.push(instr);

            // Skip trailing whitespace
            self.skip_whitespace();
        }

        Ok(Instruction::sequence(instructions))
    }

    fn parse_statement(&mut self) -> Result<Instruction, String> {
        let tok = self.peek();

        // Dispatch based on schema statement rules
        // TODO: Implement based on schema statement mappings

        Err(format!("Unknown statement: {}", tok.lexeme))
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn advance(&mut self) {
        if !self.is_at_end() {
            self.pos += 1;
        }
    }

    fn is_at_end(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    fn skip_whitespace(&mut self) {
        while !self.is_at_end() {
            let lexeme = &self.peek().lexeme;
            if lexeme.len() == 1 {
                let ch = lexeme.as_bytes()[0];
                if ch == b' ' || ch == b'\t' || ch == b'\n' || ch == b'\r' {
                    self.advance();
                    continue;
                }
            }
            break;
        }
    }
}
