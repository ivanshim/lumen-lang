use crate::languages::lumen::prelude::*;
// Array indexed assignment: arr[i] = value

use crate::kernel::ast::{Control, ExprNode, StmtNode};
use crate::kernel::parser::Parser;
use crate::languages::lumen::patterns::PatternSet;
use crate::languages::lumen::structure::structural::LBRACKET;
use crate::kernel::runtime::{Env, Value};

#[derive(Debug)]
pub struct ArrayAssignStmt {
    name: String,
    index_expr: Box<dyn ExprNode>,
    value_expr: Box<dyn ExprNode>,
}

impl StmtNode for ArrayAssignStmt {
    fn exec(&self, env: &mut Env) -> LumenResult<Control> {
        // Evaluate the index
        let index_val = self.index_expr.eval(env)?;
        let index_bigint = crate::languages::lumen::values::as_number(index_val.as_ref())?;
        let (sign, digits) = index_bigint.value.to_u32_digits();

        // Check for negative index
        use num_bigint::Sign;
        if sign == Sign::Minus {
            return Err("Array index cannot be negative".to_string());
        }

        // Get the index value (0 if digits is empty, otherwise digits[0])
        let idx = if digits.is_empty() {
            0usize
        } else if digits.len() == 1 {
            digits[0] as usize
        } else {
            return Err("Array index out of bounds".to_string());
        };

        // Evaluate the value to assign
        let value = self.value_expr.eval(env)?;

        // Get mutable reference to the array and mutate it
        env.mutate_array(&self.name, idx, value)?;

        Ok(Control::None)
    }
}

pub struct ArrayAssignHandler;

impl StmtHandler for ArrayAssignHandler {
    fn matches(&self, parser: &Parser) -> bool {
        // Check if current token is identifier
        let curr = &parser.peek().lexeme;
        let is_ident_start = curr.chars().next().map_or(false, |c| c.is_alphabetic() || c == '_');

        if !is_ident_start {
            return false;
        }

        // Look ahead for '[' followed by ']' and then '='
        let mut i = 1;
        let mut found_bracket = false;

        // Skip identifier characters
        while let Some(t) = parser.peek_n(i) {
            let lexeme = &t.lexeme;
            if lexeme.len() == 1 {
                let ch = lexeme.as_bytes()[0];
                if ch.is_ascii_alphanumeric() || ch == b'_' {
                    i += 1;
                    continue;
                }
                if ch == b' ' || ch == b'\t' {
                    i += 1;
                    continue;
                }
                if lexeme == LBRACKET {
                    found_bracket = true;
                    i += 1;
                    break;
                }
            }
            return false;
        }

        if !found_bracket {
            return false;
        }

        // Now skip to the closing bracket and look for '='
        let mut bracket_depth = 1;
        while let Some(t) = parser.peek_n(i) {
            let lexeme = &t.lexeme;
            if lexeme == LBRACKET {
                bracket_depth += 1;
            } else if lexeme == "]" {
                bracket_depth -= 1;
                if bracket_depth == 0 {
                    // Found matching ], now look for =
                    i += 1;
                    while let Some(t2) = parser.peek_n(i) {
                        let lex = &t2.lexeme;
                        if lex.len() == 1 {
                            let ch = lex.as_bytes()[0];
                            if ch == b' ' || ch == b'\t' {
                                i += 1;
                                continue;
                            }
                        }
                        return lex == "=";
                    }
                    return false;
                }
            }
            i += 1;
        }

        false
    }

    fn parse(&self, parser: &mut Parser, registry: &super::super::registry::Registry) -> LumenResult<Box<dyn StmtNode>> {
        // Parse identifier
        let mut name = parser.advance().lexeme;
        parser.skip_tokens();

        // Continue consuming identifier characters if split across tokens
        loop {
            if parser.peek().lexeme.len() == 1 {
                let ch = parser.peek().lexeme.as_bytes()[0];
                if ch.is_ascii_alphanumeric() || ch == b'_' {
                    name.push_str(&parser.advance().lexeme);
                    parser.skip_tokens();
                    continue;
                }
            }
            break;
        }

        // Expect '['
        if parser.advance().lexeme != LBRACKET {
            return Err(err_at(parser, "Expected '[' in array assignment"));
        }
        parser.skip_tokens();

        // Parse index expression
        let index_expr = parser.parse_expr(registry)?;
        parser.skip_tokens();

        // Expect ']'
        if parser.advance().lexeme != "]" {
            return Err(err_at(parser, "Expected ']' in array assignment"));
        }
        parser.skip_tokens();

        // Expect '='
        if parser.advance().lexeme != "=" {
            return Err(err_at(parser, "Expected '=' in array assignment"));
        }
        parser.skip_tokens();

        // Parse value expression
        let value_expr = parser.parse_expr(registry)?;

        Ok(Box::new(ArrayAssignStmt {
            name,
            index_expr,
            value_expr,
        }))
    }
}

// --------------------
// Pattern Declaration
// --------------------

pub fn patterns() -> PatternSet {
    PatternSet::new()
        .with_literals(vec!["[", "]", "="])
        .with_char_classes(vec!["ident_start"])
}

// --------------------
// Registration
// --------------------

pub fn register(reg: &mut Registry) {
    reg.register_stmt(Box::new(ArrayAssignHandler));
}
