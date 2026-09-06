use crate::language::prelude::*;
// Array indexed assignment: arr[i] = value

use crate::kernel::ast::{Control, ExprNode, StmtNode};
use crate::kernel::parser::Parser;
use crate::kernel::runtime::Env;

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
        let index_bigint = crate::language::values::as_number(index_val.as_ref())?;
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

        // Mutate the array in place; the array type is Lumen's, so the downcast lives here.
        let slot = env.get_mut(&self.name).ok_or_else(|| format!("Undefined variable '{}'", self.name))?;
        let arr = slot
            .as_any_mut()
            .downcast_mut::<crate::language::values::LumenArray>()
            .ok_or_else(|| format!("Variable '{}' is not an array", self.name))?;
        if idx >= arr.elements.len() {
            return Err("Array index out of bounds".to_string());
        }
        arr.elements[idx] = value;

        Ok(Control::None)
    }
}

pub struct ArrayAssignHandler;

impl StmtHandler for ArrayAssignHandler {
    fn matches(&self, parser: &Parser) -> bool {
        let d = def();
        if !parser.at_identifier() {
            return false;
        }

        // Look ahead: the rest of the identifier, the index brackets, then
        // the assignment sign.
        let mut i = 1;
        let mut found_bracket = false;
        while let Some(t) = parser.peek_n(i) {
            let lexeme = &t.lexeme;
            if lexeme.chars().count() == 1 {
                let ch = lexeme.chars().next().unwrap();
                if word_char(ch) || ch == ' ' || ch == '\t' {
                    i += 1;
                    continue;
                }
            }
            if d.is("op.index.open", lexeme) {
                found_bracket = true;
                i += 1;
            }
            break;
        }

        if !found_bracket {
            return false;
        }

        // Skip to the matching closing bracket and look for the assignment sign
        let mut bracket_depth = 1;
        while let Some(t) = parser.peek_n(i) {
            let lexeme = &t.lexeme;
            if d.is("op.index.open", lexeme) {
                bracket_depth += 1;
            } else if d.is("op.index.close", lexeme) {
                bracket_depth -= 1;
                if bracket_depth == 0 {
                    i += 1;
                    while let Some(t2) = parser.peek_n(i) {
                        let lex = &t2.lexeme;
                        if lex == " " || lex == "\t" {
                            i += 1;
                            continue;
                        }
                        return d.is("stmt.assign", lex);
                    }
                    return false;
                }
            }
            i += 1;
        }

        false
    }

    fn parse(&self, parser: &mut Parser, registry: &super::super::registry::Registry) -> LumenResult<Box<dyn StmtNode>> {
        let d = def();
        let name = parser.take_identifier().ok_or_else(|| err_at(parser, "Expected identifier"))?;
        parser.skip_tokens();

        // Expect the opening index bracket
        if !d.is("op.index.open", &parser.advance().lexeme) {
            return Err(err_at(parser, &format!("Expected '{}' in array assignment", d.first("op.index.open"))));
        }
        parser.skip_tokens();

        // Parse index expression
        let index_expr = parser.parse_expr(registry)?;
        parser.skip_tokens();

        // Expect the closing index bracket
        if !d.is("op.index.close", &parser.advance().lexeme) {
            return Err(err_at(parser, &format!("Expected '{}' in array assignment", d.first("op.index.close"))));
        }
        parser.skip_tokens();

        // Expect the assignment sign
        if !d.is("stmt.assign", &parser.advance().lexeme) {
            return Err(err_at(parser, &format!("Expected '{}' in array assignment", d.first("stmt.assign"))));
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

pub fn register(reg: &mut Registry) {
    reg.register_stmt(Box::new(ArrayAssignHandler));
}
