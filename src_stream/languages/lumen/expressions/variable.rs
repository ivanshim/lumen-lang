use crate::languages::lumen::prelude::*;
// src/expr/variable.rs
//
// Variable reference expression: `x` or function call: `func(args)`

use std::rc::Rc;
use std::cell::RefCell;
use crate::kernel::ast::{ExprNode, StmtNode};
use crate::kernel::parser::Parser;
use crate::languages::lumen::patterns::PatternSet;
use crate::kernel::runtime::{Env, Value};
use crate::languages::lumen::statements::functions;
use crate::languages::lumen::structure::structural::{LPAREN, RPAREN};

#[derive(Debug)]
struct VarExpr {
    name: String,
}

impl ExprNode for VarExpr {
    fn eval(&self, env: &mut Env) -> LumenResult<Value> {
        env.get(&self.name)
    }
}

#[derive(Debug)]
struct FunctionCallExpr {
    func_name: String,
    args: Vec<Box<dyn ExprNode>>,
}

impl ExprNode for FunctionCallExpr {
    fn eval(&self, env: &mut Env) -> LumenResult<Value> {
        // First, check if this is a built-in primitive function
        if self.args.len() == 1 {
            match self.func_name.as_str() {
                "emit" => {
                    // emit(string) - kernel primitive for I/O
                    return builtin_emit(&self.args[0].eval(env)?);
                }
                "int" => {
                    // int(x): convert string to integer
                    return builtin_int(&self.args[0].eval(env)?);
                }
                "str" => {
                    // str(x): convert any value to string
                    return builtin_str(&self.args[0].eval(env)?);
                }
                _ => {}
            }
        }

        // Get user-defined function definition
        let (params, body) = functions::get_function(&self.func_name)
            .ok_or_else(|| format!("Undefined function '{}'", self.func_name))?;

        // Check argument count
        if self.args.len() != params.len() {
            return Err(format!(
                "Function '{}' expects {} arguments, got {}",
                self.func_name,
                params.len(),
                self.args.len()
            ));
        }

        // Evaluate arguments
        let mut arg_values = Vec::new();
        for arg in &self.args {
            arg_values.push(arg.eval(env)?);
        }

        // ================================================================
        // MEMOIZATION: Gated by execution context (MEMOIZATION = true/false)
        // ================================================================
        // Cache operations are gated by env.memoization_enabled().
        // If MEMOIZATION = false (default): no cache lookup/storage
        // If MEMOIZATION = true: check cache before execution, store after
        //
        // The memoization state is dynamically scoped:
        // - Set by MEMOIZATION = true/false statements
        // - Inherited by function calls
        // - Automatically restored on scope exit
        //
        let arg_fingerprint = Env::fingerprint_args(&arg_values);
        if let Some(cached_result) = env.get_cached(&self.func_name, &arg_fingerprint) {
            // Cache hit: return cached result without executing function
            return Ok(cached_result);
        }

        // Execute function (cache lookup may have returned early)
        let result = self.execute_function(&params, &body, &arg_values, env)?;

        // Cache result if memoization is enabled
        env.cache_result(&self.func_name, &arg_fingerprint, result.clone());

        Ok(result)
    }
}

impl FunctionCallExpr {
    /// Execute function body and return result.
    /// This is factored out to be shared between cached and non-cached paths.
    fn execute_function(
        &self,
        params: &[String],
        body: &Rc<RefCell<Vec<Box<dyn StmtNode>>>>,
        arg_values: &[Value],
        env: &mut Env,
    ) -> LumenResult<Value> {
        // Create new scope for function
        env.push_scope();

        // Bind parameters to arguments
        for (param, arg_val) in params.iter().zip(arg_values) {
            env.define(param.clone(), arg_val.clone());
        }

        // Execute function body
        let mut result = Box::new(crate::languages::lumen::values::LumenNone) as Value;
        {
            let body_ref = body.borrow();
            for stmt in body_ref.iter() {
                let ctl = stmt.exec(env)?;
                match ctl {
                    crate::kernel::ast::Control::ExprValue(val) => {
                        // Expression statement value - keep as result but continue
                        result = val;
                    }
                    crate::kernel::ast::Control::Return(val) => {
                        // Explicit return - set result and break
                        result = val;
                        break;
                    }
                    crate::kernel::ast::Control::Break | crate::kernel::ast::Control::Continue => {
                        return Err("break/continue outside of loop".into());
                    }
                    crate::kernel::ast::Control::None => {}
                }
            }
        }

        // Exit function scope
        env.pop_scope();

        Ok(result)
    }
}

pub struct VariablePrefix;

impl ExprPrefix for VariablePrefix {
    fn matches(&self, parser: &Parser) -> bool {
        // Check if lexeme is a valid identifier (starts with letter or underscore)
        // Exclude only the registered statement keywords (if, else, while, break, continue, print, fn, let, mut, return)
        // Allow "and", "or", "not", "true", "false", "extern" to pass through - they'll be handled
        // by their own expression handlers (logic, literals, extern_expr)
        let lex = &parser.peek().lexeme;
        let is_identifier = lex.chars().next().map_or(false, |c| c.is_alphabetic() || c == '_');
        // Exclude statement keywords but allow builtin functions like emit, int, str
        let is_statement_keyword = matches!(lex.as_str(),
            "if" | "else" | "while" | "break" | "continue" | "fn" | "let" | "mut" | "return");
        is_identifier && !is_statement_keyword
    }

    fn parse(&self, parser: &mut Parser, registry: &super::super::registry::Registry) -> LumenResult<Box<dyn ExprNode>> {
        // Consume the first character of the identifier
        let mut name = parser.advance().lexeme;

        // Since the kernel lexer is agnostic, multi-character identifiers are split into single chars
        // Continue consuming identifier characters
        loop {
            if parser.peek().lexeme.len() == 1 {
                let ch = parser.peek().lexeme.as_bytes()[0];
                if ch.is_ascii_alphanumeric() || ch == b'_' {
                    name.push_str(&parser.advance().lexeme);
                    continue;
                }
            }
            break;
        }

        // Keywords like "and", "or", "not", "true", "false", "extern" will be collected as identifiers
        // but their own expression handlers should match first (they're registered with higher priority)
        // If we get here with one of those, it means it wasn't handled by a higher-priority handler,
        // so we try to treat it as a variable name
        // Check if this is a function call (name followed by '(')
        parser.skip_tokens();
        if parser.peek().lexeme == LPAREN {
            parser.advance(); // consume '('
            parser.skip_tokens();

            let mut args = Vec::new();

            // Parse arguments
            while parser.peek().lexeme != RPAREN {
                let arg = parser.parse_expr(registry)?;
                args.push(arg);

                parser.skip_tokens();
                if parser.peek().lexeme == "," {
                    parser.advance();
                    parser.skip_tokens();
                } else if parser.peek().lexeme != RPAREN {
                    return Err("Expected ',' or ')' after argument".into());
                }
            }

            if parser.advance().lexeme != RPAREN {
                return Err("Expected ')' after arguments".into());
            }

            return Ok(Box::new(FunctionCallExpr {
                func_name: name,
                args,
            }));
        }

        Ok(Box::new(VarExpr { name }))
    }
}

// ============================================================================
// BUILT-IN CONVERSION FUNCTIONS
// ============================================================================

/// Built-in function: int(x) - Numeric projection to integer
/// - Integer → return unchanged
/// - Rational → truncate toward zero (discard fractional part)
/// - String → parse as base-10 integer (backward compatibility)
/// This is an explicit, lossy conversion for when integer semantics are needed.
fn builtin_int(value: &Value) -> LumenResult<Value> {
    use crate::languages::lumen::values::{LumenString, LumenNumber, LumenRational};
    use num_bigint::BigInt;

    // If it's a Rational, truncate toward zero
    if let Some(rational) = value.as_any().downcast_ref::<LumenRational>() {
        // Truncate toward zero: integer division of numerator by denominator
        let truncated = &rational.numerator / &rational.denominator;
        return Ok(Box::new(LumenNumber::new(truncated)));
    }

    // If it's already a Number (integer), return unchanged
    if let Some(number) = value.as_any().downcast_ref::<LumenNumber>() {
        return Ok(Box::new(number.clone()));
    }

    // If it's a String, parse as decimal integer (backward compatibility)
    if let Some(string_val) = value.as_any().downcast_ref::<LumenString>() {
        let bigint = string_val.value.trim().parse::<BigInt>()
            .map_err(|_| format!("int(): cannot parse '{}' as integer", string_val.value))?;
        return Ok(Box::new(LumenNumber::new(bigint)));
    }

    Err("int() requires a number, rational, or string argument".to_string())
}

/// Built-in function: str(x) - Convert any value to string
/// Returns the exact string representation of the value.
/// Works with any value type (numbers, rationals, strings, etc.)
fn builtin_str(value: &Value) -> LumenResult<Value> {
    use crate::languages::lumen::values::LumenString;

    // Convert any value to its string representation
    let string = value.as_display_string();
    Ok(Box::new(LumenString::new(string)))
}

/// Built-in function: emit(string) - Kernel primitive for I/O
/// Writes a string directly to stdout without any formatting.
/// This is the only I/O side-effect in the kernel.
/// Accepts a string only - no implicit conversion.
fn builtin_emit(value: &Value) -> LumenResult<Value> {
    use crate::languages::lumen::values::LumenString;

    // Extract string value - require explicit string input
    let string_val = value.as_any()
        .downcast_ref::<LumenString>()
        .ok_or_else(|| "emit() requires a string argument".to_string())?;

    // Write to stdout
    print!("{}", string_val.value);

    // Return None (null value)
    Ok(Box::new(crate::languages::lumen::values::LumenNone))
}

// --------------------
// Pattern Declaration
// --------------------

/// Declare what patterns this module recognizes
pub fn patterns() -> PatternSet {
    PatternSet::new()
        .with_char_classes(vec!["ident_start", "ident_char"])
}

// --------------------
// Registration
// --------------------

pub fn register(reg: &mut Registry) {
    // No tokens to register (identifiers are recognized by lexer)
    // Register handlers
    reg.register_prefix(Box::new(VariablePrefix));
}
