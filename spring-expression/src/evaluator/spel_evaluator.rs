use crate::ast::expression_node::{BinaryOp, Expr, UnaryOp};
use std::collections::HashMap;

/// The runtime value of a SpEL expression.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    Null,
}

impl Value {
    /// Convert value to string (for bean injection).
    pub fn to_string_repr(&self) -> String {
        match self {
            Value::Int(i) => i.to_string(),
            Value::Float(f) => f.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Str(s) => s.clone(),
            Value::Null => "null".to_string(),
        }
    }

    fn as_bool(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Int(i) => *i != 0,
            Value::Float(f) => *f != 0.0,
            Value::Str(s) => !s.is_empty(),
            Value::Null => false,
        }
    }

    fn type_name(&self) -> &'static str {
        match self {
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::Bool(_) => "bool",
            Value::Str(_) => "string",
            Value::Null => "null",
        }
    }
}

/// Tree-walking evaluator.  The evaluation context is a plain `&HashMap<String, String>`
/// (same map used by `Environment`), so no extra types are needed.
pub struct SpelEvaluator<'a> {
    env: &'a HashMap<String, String>,
}

impl<'a> SpelEvaluator<'a> {
    pub fn new(env: &'a HashMap<String, String>) -> Self {
        SpelEvaluator { env }
    }

    pub fn eval(&self, expr: &Expr) -> Result<Value, String> {
        match expr {
            // ── literals ─────────────────────────────────────────────────
            Expr::IntLit(i) => Ok(Value::Int(*i)),
            Expr::FloatLit(f) => Ok(Value::Float(*f)),
            Expr::BoolLit(b) => Ok(Value::Bool(*b)),
            Expr::StringLit(s) => Ok(Value::Str(s.clone())),
            Expr::Null => Ok(Value::Null),

            // ── property placeholder ${key:default} ──────────────────────
            Expr::PropertyPlaceholder { key, default } => match self.env.get(key.as_str()) {
                Some(v) => Ok(Value::Str(v.clone())),
                None => match default {
                    Some(d) => Ok(Value::Str(d.clone())),
                    None => Ok(Value::Null),
                },
            },

            // ── identifier: env lookup ────────────────────────────────────
            Expr::Identifier(name) => match self.env.get(name.as_str()) {
                Some(v) => Ok(Value::Str(v.clone())),
                None => Ok(Value::Null),
            },

            // ── unary ─────────────────────────────────────────────────────
            Expr::Unary { op, expr } => {
                let val = self.eval(expr)?;
                match op {
                    UnaryOp::Neg => match val {
                        Value::Int(i) => Ok(Value::Int(-i)),
                        Value::Float(f) => Ok(Value::Float(-f)),
                        other => Err(format!("cannot negate {}", other.type_name())),
                    },
                    UnaryOp::Not => Ok(Value::Bool(!val.as_bool())),
                }
            }

            // ── binary ────────────────────────────────────────────────────
            Expr::Binary { op, left, right } => self.eval_binary(*op, left, right),

            // ── ternary ───────────────────────────────────────────────────
            Expr::Ternary {
                cond,
                then_e,
                else_e,
            } => {
                let cond_val = self.eval(cond)?;
                if cond_val.as_bool() {
                    self.eval(then_e)
                } else {
                    self.eval(else_e)
                }
            }

            // ── method call ───────────────────────────────────────────────
            Expr::MethodCall {
                target,
                method,
                args,
            } => {
                let target_val = self.eval(target)?;
                self.eval_method(target_val, method, args)
            }
        }
    }

    // ── binary ops ────────────────────────────────────────────────────────

    fn eval_binary(&self, op: BinaryOp, left: &Expr, right: &Expr) -> Result<Value, String> {
        let lv = self.eval(left)?;
        let rv = self.eval(right)?;

        // Logical short-circuit (already evaluated but fine for now)
        match op {
            BinaryOp::And => return Ok(Value::Bool(lv.as_bool() && rv.as_bool())),
            BinaryOp::Or => return Ok(Value::Bool(lv.as_bool() || rv.as_bool())),
            _ => {}
        }

        // Arithmetic / comparison
        match (&lv, &rv) {
            (Value::Int(a), Value::Int(b)) => self.int_op(op, *a, *b),
            (Value::Float(a), Value::Float(b)) => self.float_op(op, *a, *b),
            (Value::Int(a), Value::Float(b)) => self.float_op(op, *a as f64, *b),
            (Value::Float(a), Value::Int(b)) => self.float_op(op, *a, *b as f64),
            (Value::Str(a), Value::Str(b)) => {
                // Try auto-coerce both to numbers for arithmetic/comparison ops
                if let (Ok(ai), Ok(bi)) = (a.parse::<i64>(), b.parse::<i64>()) {
                    return self.int_op(op, ai, bi);
                }
                if let (Ok(af), Ok(bf)) = (a.parse::<f64>(), b.parse::<f64>()) {
                    return self.float_op(op, af, bf);
                }
                self.str_op(op, a, b)
            }
            // String auto-coerce against number
            (Value::Str(s), Value::Int(b)) => {
                if let Ok(i) = s.parse::<i64>() {
                    return self.int_op(op, i, *b);
                }
                if let Ok(f) = s.parse::<f64>() {
                    return self.float_op(op, f, *b as f64);
                }
                if op == BinaryOp::Add {
                    return Ok(Value::Str(format!("{}{}", s, b)));
                }
                Err(format!(
                    "cannot apply {:?} between '{}' (string) and int",
                    op, s
                ))
            }
            (Value::Int(a), Value::Str(s)) => {
                if let Ok(i) = s.parse::<i64>() {
                    return self.int_op(op, *a, i);
                }
                if let Ok(f) = s.parse::<f64>() {
                    return self.float_op(op, *a as f64, f);
                }
                if op == BinaryOp::Add {
                    return Ok(Value::Str(format!("{}{}", a, s)));
                }
                Err(format!(
                    "cannot apply {:?} between int and '{}' (string)",
                    op, s
                ))
            }
            (Value::Str(s), Value::Float(b)) => {
                if let Ok(f) = s.parse::<f64>() {
                    return self.float_op(op, f, *b);
                }
                Err(format!(
                    "cannot apply {:?} between '{}' (string) and float",
                    op, s
                ))
            }
            (Value::Float(a), Value::Str(s)) => {
                if let Ok(f) = s.parse::<f64>() {
                    return self.float_op(op, *a, f);
                }
                Err(format!(
                    "cannot apply {:?} between float and '{}' (string)",
                    op, s
                ))
            }
            // String + anything → string concatenation
            (Value::Str(a), _) if op == BinaryOp::Add => {
                Ok(Value::Str(format!("{}{}", a, rv.to_string_repr())))
            }
            (_, Value::Str(b)) if op == BinaryOp::Add => {
                Ok(Value::Str(format!("{}{}", lv.to_string_repr(), b)))
            }
            // Equality works across all types
            (_, _) if op == BinaryOp::Eq => Ok(Value::Bool(lv == rv)),
            (_, _) if op == BinaryOp::Ne => Ok(Value::Bool(lv != rv)),
            _ => Err(format!(
                "operator {:?} not supported between {} and {}",
                op,
                lv.type_name(),
                rv.type_name()
            )),
        }
    }

    fn int_op(&self, op: BinaryOp, a: i64, b: i64) -> Result<Value, String> {
        Ok(match op {
            BinaryOp::Add => Value::Int(a + b),
            BinaryOp::Sub => Value::Int(a - b),
            BinaryOp::Mul => Value::Int(a * b),
            BinaryOp::Div => {
                if b == 0 {
                    return Err("division by zero".into());
                }
                Value::Int(a / b)
            }
            BinaryOp::Rem => {
                if b == 0 {
                    return Err("remainder by zero".into());
                }
                Value::Int(a % b)
            }
            BinaryOp::Eq => Value::Bool(a == b),
            BinaryOp::Ne => Value::Bool(a != b),
            BinaryOp::Lt => Value::Bool(a < b),
            BinaryOp::Le => Value::Bool(a <= b),
            BinaryOp::Gt => Value::Bool(a > b),
            BinaryOp::Ge => Value::Bool(a >= b),
            _ => unreachable!(),
        })
    }

    fn float_op(&self, op: BinaryOp, a: f64, b: f64) -> Result<Value, String> {
        Ok(match op {
            BinaryOp::Add => Value::Float(a + b),
            BinaryOp::Sub => Value::Float(a - b),
            BinaryOp::Mul => Value::Float(a * b),
            BinaryOp::Div => Value::Float(a / b),
            BinaryOp::Rem => Value::Float(a % b),
            BinaryOp::Eq => Value::Bool(a == b),
            BinaryOp::Ne => Value::Bool(a != b),
            BinaryOp::Lt => Value::Bool(a < b),
            BinaryOp::Le => Value::Bool(a <= b),
            BinaryOp::Gt => Value::Bool(a > b),
            BinaryOp::Ge => Value::Bool(a >= b),
            _ => unreachable!(),
        })
    }

    fn str_op(&self, op: BinaryOp, a: &str, b: &str) -> Result<Value, String> {
        Ok(match op {
            BinaryOp::Add => Value::Str(format!("{}{}", a, b)),
            BinaryOp::Eq => Value::Bool(a == b),
            BinaryOp::Ne => Value::Bool(a != b),
            BinaryOp::Lt => Value::Bool(a < b),
            BinaryOp::Le => Value::Bool(a <= b),
            BinaryOp::Gt => Value::Bool(a > b),
            BinaryOp::Ge => Value::Bool(a >= b),
            other => Err(format!("operator {:?} not supported for strings", other))?,
        })
    }

    // ── method calls ──────────────────────────────────────────────────────

    fn eval_method(&self, target: Value, method: &str, args: &[Expr]) -> Result<Value, String> {
        match &target {
            Value::Str(s) => match method {
                "toUpperCase" | "upper" => Ok(Value::Str(s.to_uppercase())),
                "toLowerCase" | "lower" => Ok(Value::Str(s.to_lowercase())),
                "length" | "len" => Ok(Value::Int(s.chars().count() as i64)),
                "trim" => Ok(Value::Str(s.trim().to_string())),
                "isEmpty" => Ok(Value::Bool(s.is_empty())),
                "contains" => {
                    let arg = self.eval_arg(args, 0, "contains")?;
                    Ok(Value::Bool(s.contains(arg.to_string_repr().as_str())))
                }
                "startsWith" => {
                    let arg = self.eval_arg(args, 0, "startsWith")?;
                    Ok(Value::Bool(s.starts_with(arg.to_string_repr().as_str())))
                }
                "endsWith" => {
                    let arg = self.eval_arg(args, 0, "endsWith")?;
                    Ok(Value::Bool(s.ends_with(arg.to_string_repr().as_str())))
                }
                _ => Err(format!("unknown string method '{}'", method)),
            },
            Value::Int(i) => match method {
                "toString" => Ok(Value::Str(i.to_string())),
                _ => Err(format!("unknown int method '{}'", method)),
            },
            Value::Float(f) => match method {
                "toString" => Ok(Value::Str(f.to_string())),
                _ => Err(format!("unknown float method '{}'", method)),
            },
            other => Err(format!(
                "cannot call method '{}' on {}",
                method,
                other.type_name()
            )),
        }
    }

    fn eval_arg(&self, args: &[Expr], idx: usize, method: &str) -> Result<Value, String> {
        args.get(idx)
            .ok_or_else(|| format!("method '{}' requires argument {}", method, idx))
            .and_then(|e| self.eval(e))
    }
}
