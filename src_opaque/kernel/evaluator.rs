// Evaluator for opaque kernel - executes AST with opaque analysis
// Language-specific behavior determined by opaque analysis from language modules

use crate::kernel::ast::*;
use std::any::Any;
use std::rc::Rc;
use std::cell::RefCell;
use std::sync::Arc;

/// Evaluator context with environment
pub struct Evaluator {
    env: Rc<RefCell<Environment>>,
}

impl Evaluator {
    pub fn new() -> Self {
        Evaluator {
            env: Rc::new(RefCell::new(Environment::new())),
        }
    }

    /// Evaluate a program
    pub fn eval_program(&mut self, program: &Program) -> Result<RuntimeValue, String> {
        let mut last_value = Arc::new(()) as RuntimeValue;

        for stmt in &program.statements {
            match self.eval_statement(stmt)? {
                ControlFlow::Normal(v) => last_value = v,
                ControlFlow::Return(v) => return Ok(v),
                ControlFlow::Break | ControlFlow::Continue => {
                    return Err("break/continue outside loop".to_string())
                }
            }
        }

        Ok(last_value)
    }

    /// Evaluate a statement
    fn eval_statement(&mut self, stmt: &StmtNode) -> Result<ControlFlow, String> {
        match stmt {
            StmtNode::Expr { expr } => {
                let value = self.eval_expr(expr)?;
                Ok(ControlFlow::Normal(value))
            }

            StmtNode::Let { name, value, .. } => {
                let val = if let Some(expr) = value {
                    self.eval_expr(expr)?
                } else {
                    Arc::new(())
                };
                self.env.borrow_mut().set(name.clone(), val);
                Ok(ControlFlow::Normal(Arc::new(())))
            }

            StmtNode::LetMut { name, value, .. } => {
                let val = if let Some(expr) = value {
                    self.eval_expr(expr)?
                } else {
                    Arc::new(())
                };
                self.env.borrow_mut().set(name.clone(), val);
                Ok(ControlFlow::Normal(Arc::new(())))
            }

            StmtNode::Assign { target, value, .. } => {
                let val = self.eval_expr(value)?;
                self.env.borrow_mut().set(target.clone(), val);
                Ok(ControlFlow::Normal(Arc::new(())))
            }

            StmtNode::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                let cond_val = self.eval_expr(condition)?;
                let is_true = is_truthy(&cond_val);

                if is_true {
                    self.eval_block(then_block)
                } else if let Some(else_stmts) = else_block {
                    self.eval_block(else_stmts)
                } else {
                    Ok(ControlFlow::Normal(Arc::new(())))
                }
            }

            StmtNode::While { condition, body, .. } => {
                loop {
                    let cond_val = self.eval_expr(condition)?;
                    if !is_truthy(&cond_val) {
                        break;
                    }

                    match self.eval_block(body)? {
                        ControlFlow::Break => break,
                        ControlFlow::Continue => continue,
                        ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)),
                        ControlFlow::Normal(_) => {}
                    }
                }
                Ok(ControlFlow::Normal(Arc::new(())))
            }

            StmtNode::Until { condition, body, .. } => {
                loop {
                    let cond_val = self.eval_expr(condition)?;
                    if is_truthy(&cond_val) {
                        break;
                    }

                    match self.eval_block(body)? {
                        ControlFlow::Break => break,
                        ControlFlow::Continue => continue,
                        ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)),
                        ControlFlow::Normal(_) => {}
                    }
                }
                Ok(ControlFlow::Normal(Arc::new(())))
            }

            StmtNode::For {
                variable,
                iterator,
                body,
                ..
            } => {
                let iter_val = self.eval_expr(iterator)?;

                // Handle range (start..end or start..=end)
                if let Some(range) = iter_val.downcast_ref::<(i64, i64, bool)>() {
                    let (start, end, inclusive) = *range;
                    let end = if inclusive { end + 1 } else { end };

                    for i in start..end {
                        self.env.borrow_mut().set(variable.clone(), Arc::new(i));

                        match self.eval_block(body)? {
                            ControlFlow::Break => break,
                            ControlFlow::Continue => continue,
                            ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)),
                            ControlFlow::Normal(_) => {}
                        }
                    }
                } else {
                    return Err("Iterator must be a range".to_string());
                }

                Ok(ControlFlow::Normal(Arc::new(())))
            }

            StmtNode::FnDef { name, params, body, .. } => {
                self.env.borrow_mut().define_function(name.clone(), params.clone(), body.clone());
                Ok(ControlFlow::Normal(Arc::new(())))
            }

            StmtNode::Print { arguments, .. } => {
                for (i, arg) in arguments.iter().enumerate() {
                    if i > 0 {
                        print!("\n");
                    }
                    let val = self.eval_expr(arg)?;
                    print_value(&val);
                }
                println!();
                Ok(ControlFlow::Normal(Arc::new(())))
            }

            StmtNode::Return { value, .. } => {
                let val = if let Some(expr) = value {
                    self.eval_expr(expr)?
                } else {
                    Arc::new(())
                };
                Ok(ControlFlow::Return(val))
            }

            StmtNode::Break => Ok(ControlFlow::Break),
            StmtNode::Continue => Ok(ControlFlow::Continue),
        }
    }

    /// Evaluate a block of statements
    fn eval_block(&mut self, stmts: &[StmtNode]) -> Result<ControlFlow, String> {
        let mut last_value = Arc::new(()) as RuntimeValue;

        for stmt in stmts {
            match self.eval_statement(stmt)? {
                ControlFlow::Normal(v) => last_value = v,
                flow @ (ControlFlow::Return(_) | ControlFlow::Break | ControlFlow::Continue) => {
                    return Ok(flow)
                }
            }
        }

        Ok(ControlFlow::Normal(last_value))
    }

    /// Evaluate an expression
    fn eval_expr(&mut self, expr: &ExprNode) -> Result<RuntimeValue, String> {
        match expr {
            ExprNode::Literal {
                lexeme,
                handler_type,
            } => {
                match handler_type.as_str() {
                    "number" => {
                        let num: i64 = lexeme.parse::<i64>().or_else(|_| {
                            lexeme.parse::<f64>().map(|f| f as i64)
                        }).map_err(|_| format!("Invalid number: {}", lexeme))?;
                        Ok(Arc::new(num))
                    }
                    "string" => Ok(Arc::new(lexeme.clone())),
                    "keyword_true" => Ok(Arc::new(true)),
                    "keyword_false" => Ok(Arc::new(false)),
                    "keyword_none" => Ok(Arc::new(())),
                    _ => Err(format!("Unknown literal type: {}", handler_type)),
                }
            }

            ExprNode::Identifier { name } => {
                self.env.borrow().get(name)
                    .ok_or_else(|| format!("Undefined variable: {}", name))
            }

            ExprNode::Prefix { operator, right, .. } => {
                let right_val = self.eval_expr(right)?;
                eval_prefix_op(operator, &right_val)
            }

            ExprNode::Infix {
                left,
                operator,
                right,
                ..
            } => {
                let left_val = self.eval_expr(left)?;
                let right_val = self.eval_expr(right)?;
                eval_infix_op(operator, &left_val, &right_val)
            }

            ExprNode::Call {
                function,
                arguments,
                ..
            } => {
                let func_expr = match &**function {
                    ExprNode::Identifier { name } => name.clone(),
                    _ => return Err("Function call expression not yet supported".to_string()),
                };

                // Evaluate arguments
                let mut arg_values = Vec::new();
                for arg in arguments {
                    arg_values.push(self.eval_expr(arg)?);
                }

                // Look up function
                let (params, body) = self.env.borrow()
                    .get_function(&func_expr)
                    .ok_or_else(|| format!("Undefined function: {}", func_expr))?;

                if params.len() != arg_values.len() {
                    return Err(format!(
                        "Function {} expects {} arguments, got {}",
                        func_expr,
                        params.len(),
                        arg_values.len()
                    ));
                }

                // Create new environment for function
                let parent_env = self.env.clone();
                self.env = Rc::new(RefCell::new(Environment::new()));

                // Bind parameters
                for (param, arg) in params.iter().zip(arg_values.iter()) {
                    self.env.borrow_mut().set(param.clone(), arg.clone());
                }

                // Execute function body
                let mut result = Arc::new(()) as RuntimeValue;
                for stmt in &body {
                    match self.eval_statement(stmt)? {
                        ControlFlow::Normal(v) => result = v,
                        ControlFlow::Return(v) => {
                            result = v;
                            break;
                        }
                        ControlFlow::Break | ControlFlow::Continue => {
                            return Err("break/continue in function".to_string())
                        }
                    }
                }

                // Restore environment
                self.env = parent_env;

                Ok(result)
            }

            ExprNode::Grouped { expr } => self.eval_expr(expr),
        }
    }
}

/// Check if a value is truthy
fn is_truthy(val: &RuntimeValue) -> bool {
    if let Some(b) = val.downcast_ref::<bool>() {
        return *b;
    }
    if let Some(n) = val.downcast_ref::<i64>() {
        return *n != 0;
    }
    if let Some(s) = val.downcast_ref::<String>() {
        return !s.is_empty();
    }
    true
}

/// Evaluate prefix operators
fn eval_prefix_op(op: &str, right: &RuntimeValue) -> Result<RuntimeValue, String> {
    match op {
        "-" => {
            if let Some(n) = right.downcast_ref::<i64>() {
                Ok(Arc::new(-n))
            } else {
                Err(format!("Cannot negate non-number"))
            }
        }
        "!" | "not" => Ok(Arc::new(!is_truthy(right))),
        _ => Err(format!("Unknown prefix operator: {}", op)),
    }
}

/// Evaluate infix operators
fn eval_infix_op(op: &str, left: &RuntimeValue, right: &RuntimeValue) -> Result<RuntimeValue, String> {
    // Try numeric operations
    if let (Some(l), Some(r)) = (left.downcast_ref::<i64>(), right.downcast_ref::<i64>()) {
        return match op {
            "+" => Ok(Arc::new(l + r)),
            "-" => Ok(Arc::new(l - r)),
            "*" => Ok(Arc::new(l * r)),
            "/" => {
                if *r == 0 {
                    Err("Division by zero".to_string())
                } else {
                    Ok(Arc::new(l / r))
                }
            }
            "%" => {
                if *r == 0 {
                    Err("Modulo by zero".to_string())
                } else {
                    Ok(Arc::new(l % r))
                }
            }
            "==" => Ok(Arc::new(l == r)),
            "!=" => Ok(Arc::new(l != r)),
            "<" => Ok(Arc::new(l < r)),
            ">" => Ok(Arc::new(l > r)),
            "<=" => Ok(Arc::new(l <= r)),
            ">=" => Ok(Arc::new(l >= r)),
            _ => Err(format!("Unknown operator: {}", op)),
        };
    }

    // String concatenation
    if let (Some(l), Some(r)) = (left.downcast_ref::<String>(), right.downcast_ref::<String>()) {
        return match op {
            "+" => Ok(Arc::new(format!("{}{}", l, r))),
            "==" => Ok(Arc::new(l == r)),
            "!=" => Ok(Arc::new(l != r)),
            _ => Err(format!("Cannot apply {} to strings", op)),
        };
    }

    // Logical operations
    match op {
        "and" => Ok(Arc::new(is_truthy(left) && is_truthy(right))),
        "or" => Ok(Arc::new(is_truthy(left) || is_truthy(right))),
        _ => Err(format!("Type mismatch for operator {}", op)),
    }
}

/// Print a runtime value
pub fn print_value(val: &RuntimeValue) {
    if let Some(s) = val.downcast_ref::<String>() {
        print!("{}", s);
    } else if let Some(n) = val.downcast_ref::<i64>() {
        print!("{}", n);
    } else if let Some(b) = val.downcast_ref::<bool>() {
        print!("{}", b);
    } else if val.downcast_ref::<()>().is_some() {
        // Print nothing for unit type
    } else {
        print!("<value>");
    }
}
