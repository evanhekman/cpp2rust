use crate::ast::{Child, Node};
use crate::grammar::{find_production, Grammar};
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Int(i32),
    Bool(bool),
    Opt(Option<i32>),
}

impl Value {
    pub fn as_int(&self) -> Result<i32, EvalError> {
        match self {
            Value::Int(n) => Ok(*n),
            _ => Err(EvalError::TypeError),
        }
    }
    pub fn as_bool(&self) -> Result<bool, EvalError> {
        match self {
            Value::Bool(b) => Ok(*b),
            _ => Err(EvalError::TypeError),
        }
    }
    pub fn as_opt_i32(&self) -> Result<Option<i32>, EvalError> {
        match self {
            Value::Opt(o) => Ok(*o),
            _ => Err(EvalError::TypeError),
        }
    }
    pub fn matches_str(&self, s: &str) -> bool {
        match self {
            Value::Int(n) => s == n.to_string(),
            Value::Bool(b) => s == (if *b { "true" } else { "false" }),
            Value::Opt(None) => s == "None",
            Value::Opt(Some(n)) => s == format!("Some({})", n),
        }
    }
}

#[derive(Debug)]
pub enum EvalError {
    Return(Value),
    DivByZero,
    Overflow,
    TypeError,
    #[allow(dead_code)]
    UnknownKind(String),
}

pub type Env = HashMap<String, Value>;

pub fn eval(node: &Node, env: &Env, grammar: &Grammar) -> Result<Value, EvalError> {
    match node.kind.as_str() {
        // Arithmetic
        "ExprAdd" => {
            let (l, r) = eval_binary_i32(node, env, grammar)?;
            l.checked_add(r).map(Value::Int).ok_or(EvalError::Overflow)
        }
        "ExprSub" => {
            let (l, r) = eval_binary_i32(node, env, grammar)?;
            l.checked_sub(r).map(Value::Int).ok_or(EvalError::Overflow)
        }
        "ExprMul" => {
            let (l, r) = eval_binary_i32(node, env, grammar)?;
            l.checked_mul(r).map(Value::Int).ok_or(EvalError::Overflow)
        }
        "ExprDiv" => {
            let (l, r) = eval_binary_i32(node, env, grammar)?;
            if r == 0 {
                Err(EvalError::DivByZero)
            } else {
                Ok(Value::Int(l / r))
            }
        }
        "ExprMod" => {
            let (l, r) = eval_binary_i32(node, env, grammar)?;
            if r == 0 {
                Err(EvalError::DivByZero)
            } else {
                Ok(Value::Int(l % r))
            }
        }

        // Comparisons
        "ExprEq" => {
            let (l, r) = eval_binary_i32(node, env, grammar)?;
            Ok(Value::Bool(l == r))
        }
        "ExprNe" => {
            let (l, r) = eval_binary_i32(node, env, grammar)?;
            Ok(Value::Bool(l != r))
        }
        "ExprLt" => {
            let (l, r) = eval_binary_i32(node, env, grammar)?;
            Ok(Value::Bool(l < r))
        }
        "ExprGt" => {
            let (l, r) = eval_binary_i32(node, env, grammar)?;
            Ok(Value::Bool(l > r))
        }
        "ExprLe" => {
            let (l, r) = eval_binary_i32(node, env, grammar)?;
            Ok(Value::Bool(l <= r))
        }
        "ExprGe" => {
            let (l, r) = eval_binary_i32(node, env, grammar)?;
            Ok(Value::Bool(l >= r))
        }

        // Logical
        "ExprAnd" => {
            let (l, r) = eval_binary_bool(node, env, grammar)?;
            Ok(Value::Bool(l && r))
        }
        "ExprOr" => {
            let (l, r) = eval_binary_bool(node, env, grammar)?;
            Ok(Value::Bool(l || r))
        }
        "ExprNot" => {
            let child = eval_child(node, 0, env, grammar)?;
            Ok(Value::Bool(!child.as_bool()?))
        }

        // Option operations
        "ExprOptIsSome" => Ok(Value::Bool(eval_child(node, 0, env, grammar)?.as_opt_i32()?.is_some())),
        "ExprOptIsNone" => Ok(Value::Bool(eval_child(node, 0, env, grammar)?.as_opt_i32()?.is_none())),
        "ExprOptUnwrapOr" => {
            let opt = eval_child(node, 0, env, grammar)?.as_opt_i32()?;
            let default = eval_child(node, 1, env, grammar)?.as_int()?;
            Ok(Value::Int(opt.unwrap_or(default)))
        }

        // If expressions
        "ExprIfElse_i32" | "ExprIfElse_bool" => {
            let cond = eval_child(node, 0, env, grammar)?.as_bool()?;
            if cond {
                eval_child(node, 1, env, grammar)
            } else {
                eval_child(node, 2, env, grammar)
            }
        }

        // Block / Stmt
        "BlockSingle" => eval_child(node, 0, env, grammar),
        "BlockSeq" => match eval_child(node, 0, env, grammar) {
            Ok(_) => eval_child(node, 1, env, grammar),
            Err(e) => Err(e),
        },
        "StmtReturn" => {
            let val = eval_child(node, 0, env, grammar)?;
            Err(EvalError::Return(val))
        }
        "StmtIf" => {
            let cond = eval_child(node, 0, env, grammar)?.as_bool()?;
            if cond {
                eval_child(node, 1, env, grammar)?;
            }
            Ok(Value::Bool(false)) // fallthrough — value unused
        }
        "StmtIfElse" => {
            let cond = eval_child(node, 0, env, grammar)?.as_bool()?;
            if cond {
                eval_child(node, 1, env, grammar)?;
            } else {
                eval_child(node, 2, env, grammar)?;
            }
            Ok(Value::Bool(false)) // fallthrough — value unused
        }

        // Leaf: literal or ident
        kind => {
            // Try literal_value first
            if let Some(prod) = find_production(kind, grammar) {
                if let Some(val) = &prod.literal_value {
                    return Ok(val.clone());
                }
                // Ident: look up param name from rust_template
                if kind.starts_with("ExprIdent_") {
                    let name = &prod.rust_template;
                    return env
                        .get(name)
                        .cloned()
                        .ok_or(EvalError::UnknownKind(name.clone()));
                }
            }
            Err(EvalError::UnknownKind(kind.to_string()))
        }
    }
}

pub fn eval_fn(node: &Node, env: &Env, grammar: &Grammar) -> Option<Value> {
    // node is FnDefKnown_X with one Block child
    let block = match &node.children[0] {
        Child::Node(n) => n,
        _ => return None,
    };
    match eval(block, env, grammar) {
        Ok(v) => Some(v),
        Err(EvalError::Return(v)) => Some(v),
        _ => None,
    }
}

fn eval_child(node: &Node, idx: usize, env: &Env, grammar: &Grammar) -> Result<Value, EvalError> {
    match &node.children[idx] {
        Child::Node(n) => eval(n, env, grammar),
        Child::Hole(nt) => Err(EvalError::UnknownKind(format!("hole:{}", nt))),
    }
}

fn eval_binary_i32(node: &Node, env: &Env, grammar: &Grammar) -> Result<(i32, i32), EvalError> {
    let l = eval_child(node, 0, env, grammar)?.as_int()?;
    let r = eval_child(node, 1, env, grammar)?.as_int()?;
    Ok((l, r))
}

fn eval_binary_bool(node: &Node, env: &Env, grammar: &Grammar) -> Result<(bool, bool), EvalError> {
    let l = eval_child(node, 0, env, grammar)?.as_bool()?;
    let r = eval_child(node, 1, env, grammar)?.as_bool()?;
    Ok((l, r))
}
