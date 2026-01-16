use crate::languages::lumen::prelude::*;
// src/expr/variable.rs
//
// Variable reference expression: `x` or function call: `func(args)`

use std::rc::Rc;
use std::cell::RefCell;
use num_bigint::BigInt;
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
                "real" => {
                    // real(x): convert to real with default precision 15
                    return builtin_real(&self.args[0].eval(env)?, 15);
                }
                "len" => {
                    // len(x): return length of string or array
                    return builtin_len(&self.args[0].eval(env)?);
                }
                "ord" => {
                    // ord(s): return decimal integer value of first character
                    return builtin_ord(&self.args[0].eval(env)?);
                }
                "chr" => {
                    // chr(n): return single-character string for decimal integer
                    return builtin_chr(&self.args[0].eval(env)?);
                }
                "kind" => {
                    // kind(x): return symbolic constant representing value category
                    return builtin_kind(&self.args[0].eval(env)?);
                }
                "num" => {
                    // num(x): return numerator of rational (errors on non-rational)
                    return builtin_num(&self.args[0].eval(env)?);
                }
                "den" => {
                    // den(x): return denominator of rational (errors on non-rational)
                    return builtin_den(&self.args[0].eval(env)?);
                }
                "int" => {
                    // int(x): return integer part of real (errors on non-real)
                    return builtin_int_part(&self.args[0].eval(env)?);
                }
                "frac" => {
                    // frac(x): return fractional part of real (errors on non-real)
                    return builtin_frac(&self.args[0].eval(env)?);
                }
                "int_to_string" => {
                    // int_to_string(x): convert integer to string (mechanical primitive)
                    return builtin_int_to_string(&self.args[0].eval(env)?);
                }
                "real_to_string" => {
                    // real_to_string(x): convert real to string (mechanical primitive)
                    return builtin_real_to_string(&self.args[0].eval(env)?);
                }
                "rational_to_string" => {
                    // rational_to_string(x): convert rational to string (mechanical primitive)
                    return builtin_rational_to_string(&self.args[0].eval(env)?);
                }
                _ => {}
            }
        } else if self.args.len() == 2 {
            match self.func_name.as_str() {
                "real" => {
                    // real(x, y): convert to real with precision y
                    use crate::languages::lumen::values::LumenNumber;
                    use num_traits::ToPrimitive;
                    let x_val = self.args[0].eval(env)?;
                    let y_val = self.args[1].eval(env)?;
                    // Extract precision from y_val
                    let precision = match y_val.as_any().downcast_ref::<LumenNumber>() {
                        Some(num) => {
                            num.value.to_u64()
                                .ok_or_else(|| "Precision must be a positive integer".to_string())? as usize
                        }
                        None => return Err("Precision argument must be an integer".to_string()),
                    };
                    return builtin_real(&x_val, precision);
                }
                "char_at" => {
                    // char_at(string, index): return character at index
                    let str_val = self.args[0].eval(env)?;
                    let idx_val = self.args[1].eval(env)?;
                    return builtin_char_at(&str_val, &idx_val);
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

/// Built-in function: real(x, precision) - Numeric projection to real with configurable precision
/// - Integer → real (exact)
/// - Rational → real (stored as exact rational with precision hint for display)
/// - Real → real (unchanged, or with new precision)
/// Precision is in significant digits (default 15)
fn builtin_real(value: &Value, precision: usize) -> LumenResult<Value> {
    use crate::languages::lumen::values::{LumenNumber, LumenRational, LumenReal};
    use num_bigint::BigInt;

    // If it's a Real, return with new precision
    if let Some(real) = value.as_any().downcast_ref::<LumenReal>() {
        return Ok(Box::new(LumenReal::new(
            real.numerator.clone(),
            real.denominator.clone(),
            precision,
        )));
    }

    // If it's a Rational, convert to Real with precision
    if let Some(rational) = value.as_any().downcast_ref::<LumenRational>() {
        return Ok(Box::new(LumenReal::new(
            rational.numerator.clone(),
            rational.denominator.clone(),
            precision,
        )));
    }

    // If it's a Number (integer), convert to Real
    if let Some(number) = value.as_any().downcast_ref::<LumenNumber>() {
        return Ok(Box::new(LumenReal::new(
            number.value.clone(),
            BigInt::from(1),
            precision,
        )));
    }

    Err("real() requires a number, rational, or real argument".to_string())
}

/// Built-in function: int_to_string(x) - Convert integer to string (mechanical primitive)
/// Assumes input is an INTEGER. No type branching. No semantic decisions.
fn builtin_int_to_string(value: &Value) -> LumenResult<Value> {
    use crate::languages::lumen::values::{LumenString, LumenNumber};

    let number = value.as_any()
        .downcast_ref::<LumenNumber>()
        .ok_or_else(|| "int_to_string() requires an integer argument".to_string())?;

    Ok(Box::new(LumenString::new(number.value.to_string())))
}

/// Built-in function: real_to_string(x) - Convert real to string (mechanical primitive)
/// Assumes input is a REAL. No type branching. No semantic decisions.
fn builtin_real_to_string(value: &Value) -> LumenResult<Value> {
    use crate::languages::lumen::values::{LumenString, LumenReal};

    let real = value.as_any()
        .downcast_ref::<LumenReal>()
        .ok_or_else(|| "real_to_string() requires a real argument".to_string())?;

    Ok(Box::new(LumenString::new(real.as_decimal_string())))
}

/// Built-in function: rational_to_string(x) - Convert rational to string (mechanical primitive)
/// Assumes input is a RATIONAL. No type branching. No semantic decisions.
fn builtin_rational_to_string(value: &Value) -> LumenResult<Value> {
    use crate::languages::lumen::values::{LumenString, LumenRational};

    let rational = value.as_any()
        .downcast_ref::<LumenRational>()
        .ok_or_else(|| "rational_to_string() requires a rational argument".to_string())?;

    let string = if rational.is_integer() {
        rational.numerator.to_string()
    } else {
        format!("{}/{}", rational.numerator, rational.denominator)
    };

    Ok(Box::new(LumenString::new(string)))
}

/// Built-in function: len(x) - Return length of string or array
/// Returns the number of characters in a string or elements in an array.
/// For strings, counts UTF-8 characters (not bytes).
fn builtin_len(value: &Value) -> LumenResult<Value> {
    use crate::languages::lumen::values::{LumenString, LumenNumber, LumenArray};
    use num_bigint::BigInt;

    // Check if it's a string
    if let Some(string_val) = value.as_any().downcast_ref::<LumenString>() {
        let len = string_val.value.chars().count();
        return Ok(Box::new(LumenNumber::new(BigInt::from(len))));
    }

    // Check if it's an array
    if let Some(array_val) = value.as_any().downcast_ref::<LumenArray>() {
        let len = array_val.elements.len();
        return Ok(Box::new(LumenNumber::new(BigInt::from(len))));
    }

    Err("len() requires a string or array argument".to_string())
}

/// Built-in function: char_at(string, index) - Return character at index
/// Returns the character at the given zero-based index.
/// Characters are UTF-8 characters (not bytes).
/// Returns none if index is out of bounds or negative.
fn builtin_char_at(string_val: &Value, index_val: &Value) -> LumenResult<Value> {
    use crate::languages::lumen::values::{LumenString, LumenNumber, LumenNone};
    use num_traits::ToPrimitive;

    // Extract string
    let string = string_val.as_any()
        .downcast_ref::<LumenString>()
        .ok_or_else(|| "char_at() first argument must be a string".to_string())?;

    // Extract index
    let index_num = index_val.as_any()
        .downcast_ref::<LumenNumber>()
        .ok_or_else(|| "char_at() second argument must be an integer".to_string())?;

    // Convert index to usize
    let index = match index_num.value.to_usize() {
        Some(i) => i,
        None => {
            // Negative or too large index
            return Ok(Box::new(LumenNone));
        }
    };

    // Get character at index
    match string.value.chars().nth(index) {
        Some(ch) => Ok(Box::new(LumenString::new(ch.to_string()))),
        None => Ok(Box::new(LumenNone)), // Out of bounds
    }
}

/// Built-in function: ord(s) - Return decimal integer value of first character
/// Returns the UTF-8 code point of the first character in the string.
/// Errors if the argument is not a string or if the string is empty.
fn builtin_ord(value: &Value) -> LumenResult<Value> {
    use crate::languages::lumen::values::{LumenString, LumenNumber};
    use num_bigint::BigInt;

    // Extract string value
    let string_val = value.as_any()
        .downcast_ref::<LumenString>()
        .ok_or_else(|| "ord() requires a string argument".to_string())?;

    // Check if string is empty
    if string_val.value.is_empty() {
        return Err("ord() requires a non-empty string".to_string());
    }

    // Get first character and convert to Unicode code point (u32)
    let first_char = string_val.value.chars().next().unwrap();
    let code_point = first_char as u32;

    // Return as decimal integer
    Ok(Box::new(LumenNumber::new(BigInt::from(code_point))))
}

/// Built-in function: chr(n) - Return single-character string for decimal integer
/// Returns a string containing the character corresponding to the given Unicode code point.
/// Errors if the argument is not an integer, is negative, or is not a valid Unicode code point.
fn builtin_chr(value: &Value) -> LumenResult<Value> {
    use crate::languages::lumen::values::{LumenString, LumenNumber};
    use num_traits::ToPrimitive;

    // Extract integer value
    let number_val = value.as_any()
        .downcast_ref::<LumenNumber>()
        .ok_or_else(|| "chr() requires an integer argument".to_string())?;

    // Convert to u32 for char conversion
    let code_point = number_val.value.to_u32()
        .ok_or_else(|| "chr() argument must be a non-negative integer within valid Unicode range".to_string())?;

    // Convert to char (validates Unicode code point)
    let character = char::from_u32(code_point)
        .ok_or_else(|| format!("chr() argument {} is not a valid Unicode code point", code_point))?;

    // Return as single-character string
    Ok(Box::new(LumenString::new(character.to_string())))
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

/// Built-in function: kind(x) - Return kind meta-value representing value category
/// Returns one of the predefined kind constants: INTEGER, RATIONAL, REAL, ARRAY, STRING, BOOLEAN, NONE
/// This is a pure introspection function with no side effects.
fn builtin_kind(value: &Value) -> LumenResult<Value> {
    use crate::languages::lumen::values::{
        LumenNumber, LumenRational, LumenReal, LumenArray,
        LumenString, LumenBool, LumenNone, LumenKind, KindValue
    };

    // Check value type and return appropriate kind meta-value
    if value.as_any().downcast_ref::<LumenNumber>().is_some() {
        return Ok(Box::new(LumenKind::new(KindValue::INTEGER)));
    }

    if value.as_any().downcast_ref::<LumenRational>().is_some() {
        return Ok(Box::new(LumenKind::new(KindValue::RATIONAL)));
    }

    if value.as_any().downcast_ref::<LumenReal>().is_some() {
        return Ok(Box::new(LumenKind::new(KindValue::REAL)));
    }

    if value.as_any().downcast_ref::<LumenArray>().is_some() {
        return Ok(Box::new(LumenKind::new(KindValue::ARRAY)));
    }

    if value.as_any().downcast_ref::<LumenString>().is_some() {
        return Ok(Box::new(LumenKind::new(KindValue::STRING)));
    }

    if value.as_any().downcast_ref::<LumenBool>().is_some() {
        return Ok(Box::new(LumenKind::new(KindValue::BOOLEAN)));
    }

    if value.as_any().downcast_ref::<LumenNone>().is_some() {
        return Ok(Box::new(LumenKind::new(KindValue::NONE)));
    }

    // Unknown value type
    Err("kind(): unknown value type".to_string())
}

/// Built-in function: num(x) - Extract numerator from rational
/// Valid only for RATIONAL values. Returns the numerator as an INTEGER.
/// Errors on all other kinds.
fn builtin_num(value: &Value) -> LumenResult<Value> {
    use crate::languages::lumen::values::{LumenNumber, LumenRational};

    // Check if it's a Rational
    if let Some(rational) = value.as_any().downcast_ref::<LumenRational>() {
        return Ok(Box::new(LumenNumber::new(rational.numerator.clone())));
    }

    Err("num() requires a rational argument".to_string())
}

/// Built-in function: den(x) - Extract denominator from rational
/// Valid only for RATIONAL values. Returns the denominator as an INTEGER.
/// Errors on all other kinds.
fn builtin_den(value: &Value) -> LumenResult<Value> {
    use crate::languages::lumen::values::{LumenNumber, LumenRational};

    // Check if it's a Rational
    if let Some(rational) = value.as_any().downcast_ref::<LumenRational>() {
        return Ok(Box::new(LumenNumber::new(rational.denominator.clone())));
    }

    Err("den() requires a rational argument".to_string())
}

/// Built-in function: int(x) - Extract integer part from real
/// Valid only for REAL values. Returns the integer part as an INTEGER.
/// Must satisfy: int(x) + frac(x) == x
/// Errors on all other kinds.
fn builtin_int_part(value: &Value) -> LumenResult<Value> {
    use crate::languages::lumen::values::{LumenNumber, LumenReal};

    // Check if it's a Real
    if let Some(real) = value.as_any().downcast_ref::<LumenReal>() {
        // Integer part: truncate toward zero (integer division)
        let int_part = &real.numerator / &real.denominator;
        return Ok(Box::new(LumenNumber::new(int_part)));
    }

    Err("int() requires a real argument".to_string())
}

/// Built-in function: frac(x) - Extract fractional part from real
/// Valid only for REAL values. Returns the fractional part as a REAL.
/// Must satisfy: int(x) + frac(x) == x
/// Preserves precision exactly.
/// Errors on all other kinds.
fn builtin_frac(value: &Value) -> LumenResult<Value> {
    use crate::languages::lumen::values::{LumenReal};

    // Check if it's a Real
    if let Some(real) = value.as_any().downcast_ref::<LumenReal>() {
        // Fractional part: x - int(x)
        // If x = numerator/denominator, then:
        // int(x) = numerator / denominator (integer division)
        // frac(x) = x - int(x) = numerator/denominator - (numerator / denominator)
        //         = (numerator - (numerator / denominator) * denominator) / denominator
        let int_part = &real.numerator / &real.denominator;
        let frac_numerator = &real.numerator - (&int_part * &real.denominator);

        // Return as REAL with same precision, preserving exact structure
        return Ok(Box::new(LumenReal::new(
            frac_numerator,
            real.denominator.clone(),
            real.precision,
        )));
    }

    Err("frac() requires a real argument".to_string())
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
