use crate::language::prelude::*;
// src/expr/variable.rs
//
// Variable reference expression: `x` or function call: `func(args)`.
// Builtin function names come from the definition; a call that names none
// of them reaches a user-defined function.

use std::rc::Rc;
use std::cell::RefCell;
use crate::kernel::ast::{ExprNode, StmtNode};
use crate::kernel::parser::Parser;
use crate::kernel::runtime::{Env, Value};
use crate::language::expressions::calls;
use crate::language::statements::functions;

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

/// A builtin taking one argument.
type Unary = fn(&Value) -> LumenResult<Value>;

/// The one-argument builtins, by the label that names each.
const UNARY_BUILTINS: &[(&str, Unary)] = &[
    ("builtin.emit", builtin_emit),
    ("builtin.len", builtin_len),
    ("builtin.ord", builtin_ord),
    ("builtin.chr", builtin_chr),
    ("builtin.error", builtin_error),
    ("builtin.typeof", builtin_kind),
    ("builtin.num", builtin_num),
    ("builtin.den", builtin_den),
    ("builtin.precision", builtin_precision),
    ("builtin.to_string", builtin_to_string),
    ("builtin.to_int", builtin_to_int),
    ("builtin.to_real", builtin_to_real),
];

impl ExprNode for FunctionCallExpr {
    fn eval(&self, env: &mut Env) -> LumenResult<Value> {
        let name = self.func_name.as_str();
        let mut values = Vec::with_capacity(self.args.len());
        for arg in &self.args {
            values.push(arg.eval(env)?);
        }
        if def().is_builtin(name) {
            return call_builtin(name, values);
        }
        call_user_function(name, values, env)
    }
}

/// Apply the builtin `name` spells to evaluated arguments.
pub(crate) fn call_builtin(name: &str, values: Vec<Value>) -> LumenResult<Value> {
    let d = def();
    if let Some((_, f)) = UNARY_BUILTINS.iter().find(|(label, _)| d.is(label, name)) {
        return match values.as_slice() {
            [x] => f(x),
            _ => Err(format!("{}() expects 1 argument, got {}", name, values.len())),
        };
    }
    if d.is("builtin.real", name) {
        return match values.as_slice() {
            [x] => builtin_real(x, 15),
            [x, precision] => {
                use crate::language::values::LumenNumber;
                use num_traits::ToPrimitive;
                let precision = match precision.as_any().downcast_ref::<LumenNumber>() {
                    Some(num) => num.value.to_u64().ok_or_else(|| "Precision must be a positive integer".to_string())? as usize,
                    None => return Err("Precision argument must be an integer".to_string()),
                };
                builtin_real(x, precision)
            }
            _ => Err(format!("{}() expects 1 or 2 arguments, got {}", name, values.len())),
        };
    }
    if d.is("builtin.char_at", name) {
        return match values.as_slice() {
            [s, i] => builtin_char_at(s, i),
            _ => Err(format!("{}() expects 2 arguments, got {}", name, values.len())),
        };
    }
    if d.is("builtin.range", name) {
        return Err(format!("{}() spells a range, which belongs in a for loop", name));
    }
    if d.is("builtin.print", name) {
        println!("{}", render(&values));
        return Ok(Box::new(crate::language::values::LumenNull));
    }
    if d.is("builtin.write", name) {
        print!("{}", render(&values));
        return Ok(Box::new(crate::language::values::LumenNull));
    }
    // Statement-form builtins (emit, push, extern) never reach a call node.
    Err(format!("'{}' is not callable as an expression", name))
}

/// Text for print and write: the values joined by spaces, or, when the
/// first value is a string holding placeholders (`builtin.print.placeholder`:
/// Rust's `{}`, C's `%d`) and more values follow, that string with each
/// placeholder filled in order.
fn render(values: &[Value]) -> String {
    use crate::language::values::as_string;
    let placeholders = def().list("builtin.print.placeholder");
    // The earliest placeholder in a piece of the template, with its width.
    let next_hole = |s: &str| {
        placeholders
            .iter()
            .filter_map(|p| s.find(p.as_str()).map(|at| (at, p.len())))
            .min()
    };
    if values.len() > 1 {
        if let Ok(template) = as_string(values[0].as_ref()) {
            if next_hole(&template.value).is_some() {
                let mut out = String::new();
                let mut rest = values[1..].iter();
                let mut s = template.value.as_str();
                while let Some((at, width)) = next_hole(s) {
                    out.push_str(&s[..at]);
                    match rest.next() {
                        Some(v) => out.push_str(&v.as_display_string()),
                        None => out.push_str(&s[at..at + width]),
                    }
                    s = &s[at + width..];
                }
                out.push_str(s);
                return out;
            }
        }
    }
    values.iter().map(|v| v.as_display_string()).collect::<Vec<_>>().join(" ")
}

/// Call a user-defined function by name with evaluated arguments, in a
/// fresh scope, honouring memoization. Shared by calls, the pipe operator
/// and the entry function.
pub fn call_user_function(name: &str, arg_values: Vec<Value>, env: &mut Env) -> LumenResult<Value> {
    let (params, body) = functions::get_function(name).ok_or_else(|| format!("Undefined function '{}'", name))?;

    if arg_values.len() != params.len() {
        return Err(format!("Function '{}' expects {} arguments, got {}", name, params.len(), arg_values.len()));
    }

    // Memoization is a language feature layered on the environment; see memo.rs.
    if let Some(cached) = crate::language::memo::lookup(env, name, &arg_values) {
        return Ok(cached);
    }

    let result = execute_function(name, &params, &body, &arg_values, env)?;
    crate::language::memo::store(env, name, &arg_values, &result);
    Ok(result)
}

/// Execute a function body in a fresh scope that is popped on every exit path.
fn execute_function(
    name: &str,
    params: &[String],
    body: &Rc<RefCell<Vec<Box<dyn StmtNode>>>>,
    arg_values: &[Value],
    env: &mut Env,
) -> LumenResult<Value> {
    env.with_scope(|env| {
        for (param, arg_val) in params.iter().zip(arg_values) {
            env.define(param.clone(), arg_val.clone());
        }

        let mut result = Box::new(crate::language::values::LumenNull) as Value;
        let mut returned = false;
        let body_ref = body.borrow();
        for stmt in body_ref.iter() {
            match stmt.exec(env)? {
                crate::kernel::ast::Control::ExprValue(val) => result = val,
                crate::kernel::ast::Control::Return(val) => {
                    result = val;
                    returned = true;
                    break;
                }
                crate::kernel::ast::Control::Break | crate::kernel::ast::Control::Continue => {
                    return Err("break/continue outside of loop".into());
                }
                crate::kernel::ast::Control::None => {}
            }
        }
        // Pascal: a body that ends without returning yields what it assigned
        // to the function's own name.
        if !returned && def().result_by_name {
            if let Ok(named) = env.get(name) {
                result = named;
            }
        }
        Ok(result)
    })
}

pub struct VariablePrefix;

impl ExprPrefix for VariablePrefix {
    fn matches(&self, parser: &Parser) -> bool {
        // An identifier that is not a reserved word. Reserved words with
        // expression meaning (literals, logic, extern) have their own
        // handlers registered ahead of this one.
        parser.at_identifier() && !def().is_reserved(&parser.peek().lexeme)
    }

    fn parse(&self, parser: &mut Parser, registry: &super::super::registry::Registry) -> LumenResult<Box<dyn ExprNode>> {
        // A compound builtin name (println!, console.log) arrives as one token.
        let name = parser.take_identifier().ok_or_else(|| err_at(parser, "Expected identifier"))?;

        // A call is a name followed by the call bracket
        parser.skip_tokens();
        if calls::at_call_open(parser) {
            let args = calls::parse_arguments(parser, registry, "in call")?;
            return Ok(Box::new(FunctionCallExpr { func_name: name, args }));
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
    use crate::language::values::{LumenNumber, LumenRational, LumenReal};
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

/// precision(x): the significant digits a real carries
fn builtin_precision(value: &Value) -> LumenResult<Value> {
    use crate::language::values::{LumenNumber, LumenReal};
    let real = value
        .as_any()
        .downcast_ref::<LumenReal>()
        .ok_or_else(|| "precision() requires a real argument".to_string())?;
    Ok(Box::new(LumenNumber::new(num_bigint::BigInt::from(real.precision))))
}

/// to_string(x): any value as the text print would show. The polymorphic
/// conversion of languages that have one (`str`, `String`); Lumen renders
/// in its library instead.
fn builtin_to_string(value: &Value) -> LumenResult<Value> {
    use crate::language::values::LumenString;
    Ok(Box::new(LumenString::new(value.as_display_string())))
}

/// to_int(x): the integer part of any number
fn builtin_to_int(value: &Value) -> LumenResult<Value> {
    use crate::language::values::{LumenNumber, LumenRational, LumenReal};
    if let Some(number) = value.as_any().downcast_ref::<LumenNumber>() {
        return Ok(Box::new(LumenNumber::new(number.value.clone())));
    }
    if let Some(rational) = value.as_any().downcast_ref::<LumenRational>() {
        return Ok(Box::new(LumenNumber::new(&rational.numerator / &rational.denominator)));
    }
    if let Some(real) = value.as_any().downcast_ref::<LumenReal>() {
        return Ok(Box::new(LumenNumber::new(&real.numerator / &real.denominator)));
    }
    Err("to_int() requires a number argument".to_string())
}

/// to_real(x): any number as a real, at its own precision if it is one
fn builtin_to_real(value: &Value) -> LumenResult<Value> {
    use crate::language::values::LumenReal;
    if let Some(real) = value.as_any().downcast_ref::<LumenReal>() {
        return Ok(Box::new(real.clone()));
    }
    builtin_real(value, 15)
}

/// Built-in function: len(x) - Return length of string or array
/// Returns the number of characters in a string or elements in an array.
/// For strings, counts UTF-8 characters (not bytes).
fn builtin_len(value: &Value) -> LumenResult<Value> {
    use crate::language::values::{LumenString, LumenNumber, LumenArray};
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
/// Errors if index is out of bounds or negative (strict, truth-preserving semantics).
fn builtin_char_at(string_val: &Value, index_val: &Value) -> LumenResult<Value> {
    use crate::language::values::{LumenString, LumenNumber};
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
            return Err("char_at index out of bounds".to_string());
        }
    };

    // Get character at index
    match string.value.chars().nth(index) {
        Some(ch) => Ok(Box::new(LumenString::new(ch.to_string()))),
        None => Err("char_at index out of bounds".to_string()), // Out of bounds
    }
}

/// Built-in function: ord(s) - Return decimal integer value of first character
/// Returns the UTF-8 code point of the first character in the string.
/// Errors if the argument is not a string or if the string is empty.
fn builtin_ord(value: &Value) -> LumenResult<Value> {
    use crate::language::values::{LumenString, LumenNumber};
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
    use crate::language::values::{LumenString, LumenNumber};
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

/// Built-in function: error(message) - Abort execution with error message
/// Immediately propagates the error message to halt execution.
/// This is a kernel primitive for unified error handling.
/// No I/O is performed - the error is propagated via Result.
fn builtin_error(msg_val: &Value) -> LumenResult<Value> {
    use crate::language::values::LumenString;

    // Extract string message
    let msg = msg_val.as_any()
        .downcast_ref::<LumenString>()
        .ok_or_else(|| "error() argument must be a string".to_string())?;

    // Return error to abort execution (no I/O)
    Err(msg.value.clone())
}

/// Built-in function: emit(string) - Kernel primitive for I/O
/// Writes a string directly to stdout without any formatting.
/// This is the only I/O side-effect in the kernel.
/// Accepts a string only - no implicit conversion.
fn builtin_emit(value: &Value) -> LumenResult<Value> {
    use crate::language::values::LumenString;

    // Extract string value - require explicit string input
    let string_val = value.as_any()
        .downcast_ref::<LumenString>()
        .ok_or_else(|| "emit() requires a string argument".to_string())?;

    // Write to stdout
    print!("{}", string_val.value);

    // Return null value
    Ok(Box::new(crate::language::values::LumenNull))
}

/// Built-in function: kind(x) - Return kind meta-value representing value category
/// Returns one of the predefined kind constants: INTEGER, RATIONAL, REAL, ARRAY, STRING, BOOLEAN, NULL
/// This is a pure introspection function with no side effects.
fn builtin_kind(value: &Value) -> LumenResult<Value> {
    use crate::language::values::{
        LumenNumber, LumenRational, LumenReal, LumenArray,
        LumenString, LumenBool, LumenNull, LumenKind, KindValue
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

    if value.as_any().downcast_ref::<LumenNull>().is_some() {
        return Ok(Box::new(LumenKind::new(KindValue::NULL)));
    }

    if value.as_any().downcast_ref::<LumenKind>().is_some() {
        // KIND is a meta-value representing types - return a special KIND marker
        // This allows kind(INTEGER) to work, returning a kind-of-kind meta-value
        return Ok(Box::new(LumenKind::new(KindValue::NULL))); // Use NULL as placeholder for KIND-of-KIND
    }

    // Unknown value type
    Err("kind(): unknown value type".to_string())
}

/// Built-in function: num(x) - Extract numerator from rational
/// Valid only for RATIONAL values. Returns the numerator as an INTEGER.
/// Errors on all other kinds.
fn builtin_num(value: &Value) -> LumenResult<Value> {
    use crate::language::values::{LumenNumber, LumenRational, LumenReal};

    // Any number has a numerator: an integer is its own, a real carries one.
    if let Some(number) = value.as_any().downcast_ref::<LumenNumber>() {
        return Ok(Box::new(LumenNumber::new(number.value.clone())));
    }
    if let Some(rational) = value.as_any().downcast_ref::<LumenRational>() {
        return Ok(Box::new(LumenNumber::new(rational.numerator.clone())));
    }
    if let Some(real) = value.as_any().downcast_ref::<LumenReal>() {
        return Ok(Box::new(LumenNumber::new(real.numerator.clone())));
    }

    Err("num() requires a number argument".to_string())
}

/// Built-in function: den(x) - Extract denominator from rational
/// Valid only for RATIONAL values. Returns the denominator as an INTEGER.
/// Errors on all other kinds.
fn builtin_den(value: &Value) -> LumenResult<Value> {
    use crate::language::values::{LumenNumber, LumenRational, LumenReal};

    // Any number has a denominator: an integer's is 1.
    if value.as_any().downcast_ref::<LumenNumber>().is_some() {
        return Ok(Box::new(LumenNumber::new(num_bigint::BigInt::from(1))));
    }
    if let Some(rational) = value.as_any().downcast_ref::<LumenRational>() {
        return Ok(Box::new(LumenNumber::new(rational.denominator.clone())));
    }
    if let Some(real) = value.as_any().downcast_ref::<LumenReal>() {
        return Ok(Box::new(LumenNumber::new(real.denominator.clone())));
    }

    Err("den() requires a number argument".to_string())
}

pub fn register(reg: &mut Registry) {
    reg.register_prefix(Box::new(VariablePrefix));
}
