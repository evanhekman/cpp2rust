use crate::ast::{Child, Node};
use crate::grammar::{find_production, Grammar};
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Int(i32),
    U32(u32),
    U8(u8),
    Usize(usize),
    Bool(bool),
    Opt(Option<i32>),
    SliceI32(Vec<i32>),
    SliceU8(Vec<u8>),
    SliceMutI32(Vec<i32>),
    Unit,
}

impl Value {
    pub fn as_int(&self) -> Result<i32, EvalError> {
        match self {
            Value::Int(n) => Ok(*n),
            _ => Err(EvalError::TypeError),
        }
    }
    pub fn as_u32(&self) -> Result<u32, EvalError> {
        match self {
            Value::U32(n) => Ok(*n),
            _ => Err(EvalError::TypeError),
        }
    }
    pub fn as_u8(&self) -> Result<u8, EvalError> {
        match self {
            Value::U8(n) => Ok(*n),
            _ => Err(EvalError::TypeError),
        }
    }
    pub fn as_usize(&self) -> Result<usize, EvalError> {
        match self {
            Value::Usize(n) => Ok(*n),
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
    #[allow(dead_code)]
    pub fn as_slice_i32(&self) -> Result<&[i32], EvalError> {
        match self {
            Value::SliceI32(v) | Value::SliceMutI32(v) => Ok(v.as_slice()),
            _ => Err(EvalError::TypeError),
        }
    }
    #[allow(dead_code)]
    pub fn as_slice_u8(&self) -> Result<&[u8], EvalError> {
        match self {
            Value::SliceU8(v) => Ok(v.as_slice()),
            _ => Err(EvalError::TypeError),
        }
    }
    pub fn matches_str(&self, s: &str) -> bool {
        match self {
            Value::Int(n) => s == n.to_string(),
            Value::U32(n) => s == n.to_string(),
            Value::U8(n)  => s == n.to_string(),
            Value::Usize(n) => s == n.to_string(),
            Value::Bool(b) => s == (if *b { "true" } else { "false" }),
            Value::Opt(None) => s == "None",
            Value::Opt(Some(n)) => s == format!("Some({})", n),
            Value::Unit => s == "()",
            Value::SliceI32(v) | Value::SliceMutI32(v) => {
                let rendered = format!("[{}]", v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","));
                s == rendered
            }
            Value::SliceU8(v) => {
                let rendered = format!("[{}]", v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","));
                s == rendered
            }
        }
    }
}

#[derive(Debug)]
pub enum EvalError {
    Return(Value),
    DivByZero,
    Overflow,
    TypeError,
    IndexOutOfBounds,
    InfiniteLoop,
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

        // usize comparisons
        "ExprLt_usize" => {
            let (l, r) = eval_binary_usize(node, env, grammar)?;
            Ok(Value::Bool(l < r))
        }
        "ExprGt_usize" => {
            let (l, r) = eval_binary_usize(node, env, grammar)?;
            Ok(Value::Bool(l > r))
        }
        "ExprLe_usize" => {
            let (l, r) = eval_binary_usize(node, env, grammar)?;
            Ok(Value::Bool(l <= r))
        }
        "ExprGe_usize" => {
            let (l, r) = eval_binary_usize(node, env, grammar)?;
            Ok(Value::Bool(l >= r))
        }
        "ExprEq_usize" => {
            let (l, r) = eval_binary_usize(node, env, grammar)?;
            Ok(Value::Bool(l == r))
        }
        "ExprNe_usize" => {
            let (l, r) = eval_binary_usize(node, env, grammar)?;
            Ok(Value::Bool(l != r))
        }

        // usize arithmetic
        "ExprAdd_usize" => {
            let (l, r) = eval_binary_usize(node, env, grammar)?;
            l.checked_add(r).map(Value::Usize).ok_or(EvalError::Overflow)
        }
        "ExprSub_usize" => {
            let (l, r) = eval_binary_usize(node, env, grammar)?;
            l.checked_sub(r).map(Value::Usize).ok_or(EvalError::Overflow)
        }
        "ExprMul_usize" => {
            let (l, r) = eval_binary_usize(node, env, grammar)?;
            l.checked_mul(r).map(Value::Usize).ok_or(EvalError::Overflow)
        }

        // u32 arithmetic
        "ExprAdd_u32" => {
            let (l, r) = eval_binary_u32(node, env, grammar)?;
            l.checked_add(r).map(Value::U32).ok_or(EvalError::Overflow)
        }
        "ExprMul_u32" => {
            let (l, r) = eval_binary_u32(node, env, grammar)?;
            l.checked_mul(r).map(Value::U32).ok_or(EvalError::Overflow)
        }

        // Casts
        "ExprCast_i32" => {
            let v = eval_child(node, 0, env, grammar)?.as_usize()?;
            Ok(Value::Int(v as i32))
        }
        "ExprCast_i32_u8" => {
            let v = eval_child(node, 0, env, grammar)?.as_u8()?;
            Ok(Value::Int(v as i32))
        }
        "ExprCast_u32_u8" => {
            let v = eval_child(node, 0, env, grammar)?.as_u8()?;
            Ok(Value::U32(v as u32))
        }
        "ExprCast_u32_usize" => {
            let v = eval_child(node, 0, env, grammar)?.as_usize()?;
            Ok(Value::U32(v as u32))
        }
        "ExprCast_u32_i32" => {
            let v = eval_child(node, 0, env, grammar)?.as_int()?;
            Ok(Value::U32(v as u32))
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
                // Scalar ident: look up by name stored in template
                if kind.starts_with("ExprIdent_") {
                    let name = &prod.rust_template;
                    return env
                        .get(name)
                        .cloned()
                        .ok_or(EvalError::UnknownKind(name.clone()));
                }
                // Slice length: ExprLen_{param} → param.len()
                if kind.starts_with("ExprLen_") {
                    let param_name = kind.strip_prefix("ExprLen_").unwrap();
                    let val = env.get(param_name)
                        .ok_or_else(|| EvalError::UnknownKind(param_name.to_string()))?;
                    let len = match val {
                        Value::SliceI32(v) | Value::SliceMutI32(v) => v.len(),
                        Value::SliceU8(v) => v.len(),
                        _ => return Err(EvalError::TypeError),
                    };
                    return Ok(Value::Usize(len));
                }
                // Slice index: ExprIndex_{param}[usize] — handled via children
                if kind.starts_with("ExprIndex_") {
                    let param_name = kind.strip_prefix("ExprIndex_").unwrap();
                    let idx = eval_child(node, 0, env, grammar)?.as_usize()?;
                    let val = env.get(param_name)
                        .ok_or_else(|| EvalError::UnknownKind(param_name.to_string()))?;
                    return match val {
                        Value::SliceI32(v) | Value::SliceMutI32(v) => {
                            v.get(idx).copied().map(Value::Int)
                                .ok_or(EvalError::IndexOutOfBounds)
                        }
                        Value::SliceU8(v) => {
                            v.get(idx).copied().map(Value::U8)
                                .ok_or(EvalError::IndexOutOfBounds)
                        }
                        _ => Err(EvalError::TypeError),
                    };
                }
            }
            Err(EvalError::UnknownKind(kind.to_string()))
        }
    }
}

pub fn eval_fn(node: &Node, env: &Env, grammar: &Grammar) -> Option<(Value, Env)> {
    // node is FnDefKnown_X with one Block child
    let block = match &node.children[0] {
        Child::Node(n) => n,
        _ => return None,
    };
    let mut env = env.clone();
    let val = match eval_block(block, &mut env, grammar) {
        Ok(_) => Value::Unit,
        Err(EvalError::Return(v)) => v,
        _ => return None,
    };
    Some((val, env))
}

/// Evaluate a Block node, updating env in place.
/// Returns Ok(()) on fallthrough, Err(Return(v)) on explicit return.
pub fn eval_block(node: &Node, env: &mut Env, grammar: &Grammar) -> Result<(), EvalError> {
    for child in &node.children {
        match child {
            Child::Node(stmt) => eval_stmt(stmt, env, grammar)?,
            Child::Hole(_) => return Err(EvalError::UnknownKind("hole".into())),
        }
    }
    Ok(())
}

/// Evaluate a statement node, mutating env.
/// Propagates Err(Return(v)) for explicit returns.
pub fn eval_stmt(node: &Node, env: &mut Env, grammar: &Grammar) -> Result<(), EvalError> {
    match node.kind.as_str() {
        // ── Existing statement kinds ─────────────────────────────────────────
        "BlockSingle" | "BlockSeq" | "BlockSeq3" | "BlockSeq4" | "BlockSeq5" => {
            eval_block(node, env, grammar)
        }

        "StmtReturn" => {
            let val = eval_expr_child(node, 0, env, grammar)?;
            Err(EvalError::Return(val))
        }

        "StmtIf" => {
            let cond = eval_expr_child(node, 0, env, grammar)?.as_bool()?;
            if cond {
                let body = child_node(node, 1)?;
                eval_block(body, env, grammar)?;
            }
            Ok(())
        }

        "StmtIfElse" => {
            let cond = eval_expr_child(node, 0, env, grammar)?.as_bool()?;
            if cond {
                let body = child_node(node, 1)?;
                eval_block(body, env, grammar)?;
            } else {
                let body = child_node(node, 2)?;
                eval_block(body, env, grammar)?;
            }
            Ok(())
        }

        "StmtWhile" => {
            const MAX_ITERS: usize = 10_000;
            for _ in 0..MAX_ITERS {
                let cond = eval_expr_child(node, 0, env, grammar)?.as_bool()?;
                if !cond { return Ok(()); }
                let body = child_node(node, 1)?;
                eval_block(body, env, grammar)?;
            }
            Err(EvalError::InfiniteLoop)
        }

        // ── Let / assign / compound assign ───────────────────────────────────
        kind if kind.starts_with("StmtLetMut_") => {
            // Template: "let mut NAME: TYPE = {0};"
            // Extract variable name from template
            let var_name = letmut_var_name(&find_production(kind, grammar)
                .ok_or_else(|| EvalError::UnknownKind(kind.to_string()))?.rust_template);
            let val = eval_expr_child(node, 0, env, grammar)?;
            env.insert(var_name, val);
            Ok(())
        }

        kind if kind.starts_with("StmtAssign_") => {
            // Template: "NAME = {0};"
            let var_name = assign_var_name(&find_production(kind, grammar)
                .ok_or_else(|| EvalError::UnknownKind(kind.to_string()))?.rust_template);
            let val = eval_expr_child(node, 0, env, grammar)?;
            env.insert(var_name, val);
            Ok(())
        }

        kind if kind.starts_with("StmtCompoundPlus_") => {
            // Template: "NAME += {0};"
            let var_name = compound_var_name(&find_production(kind, grammar)
                .ok_or_else(|| EvalError::UnknownKind(kind.to_string()))?.rust_template);
            let rhs = eval_expr_child(node, 0, env, grammar)?;
            let cur = env.get(&var_name).cloned()
                .ok_or_else(|| EvalError::UnknownKind(var_name.clone()))?;
            let new_val = add_values(cur, rhs)?;
            env.insert(var_name, new_val);
            Ok(())
        }

        kind if kind.starts_with("StmtCompoundMinus_") => {
            // Template: "NAME -= {0};"
            let var_name = compound_var_name(&find_production(kind, grammar)
                .ok_or_else(|| EvalError::UnknownKind(kind.to_string()))?.rust_template);
            let rhs = eval_expr_child(node, 0, env, grammar)?;
            let cur = env.get(&var_name).cloned()
                .ok_or_else(|| EvalError::UnknownKind(var_name.clone()))?;
            let new_val = sub_values(cur, rhs)?;
            env.insert(var_name, new_val);
            Ok(())
        }

        // ── For loop: StmtFor_{var}_{idx} ────────────────────────────────────
        kind if kind.starts_with("StmtFor_") => {
            let prod = find_production(kind, grammar)
                .ok_or_else(|| EvalError::UnknownKind(kind.to_string()))?;
            // Template: "for VAR in 0..{0} {1}"
            let var_name = for_loop_var(&prod.rust_template);
            let limit = eval_expr_child(node, 0, env, grammar)?.as_usize()?;
            let body = child_node(node, 1)?;
            for i in 0..limit {
                env.insert(var_name.clone(), Value::Usize(i));
                eval_block(body, env, grammar)?;
            }
            Ok(())
        }

        // ── Mutable slice assignment: StmtSliceAssign_{param} ────────────────
        kind if kind.starts_with("StmtSliceAssign_") => {
            let param_name = kind.strip_prefix("StmtSliceAssign_").unwrap().to_string();
            let idx = eval_expr_child(node, 0, env, grammar)?.as_usize()?;
            let rhs = eval_expr_child(node, 1, env, grammar)?;
            let slice = env.get_mut(&param_name)
                .ok_or_else(|| EvalError::UnknownKind(param_name.clone()))?;
            match (slice, rhs) {
                (Value::SliceMutI32(v), Value::Int(x)) => {
                    *v.get_mut(idx).ok_or(EvalError::IndexOutOfBounds)? = x;
                }
                _ => return Err(EvalError::TypeError),
            }
            Ok(())
        }

        _ => {
            // Fallback: treat as pure expression statement (ignore result)
            eval(node, env, grammar).map(|_| ())
        }
    }
}

// ── Template parsing helpers ──────────────────────────────────────────────────

fn letmut_var_name(template: &str) -> String {
    // "let mut NAME: TYPE = {0};"  →  NAME
    let rest = template.strip_prefix("let mut ").unwrap_or(template);
    rest.split(|c| c == ':' || c == ' ').next().unwrap_or("").to_string()
}

fn assign_var_name(template: &str) -> String {
    // "NAME = {0};"  →  NAME
    template.split_whitespace().next().unwrap_or("").to_string()
}

fn compound_var_name(template: &str) -> String {
    // "NAME += {0};"  or  "NAME -= {0};"  →  NAME
    template.split_whitespace().next().unwrap_or("").to_string()
}

fn for_loop_var(template: &str) -> String {
    // "for NAME in 0..{0} {1}"  →  NAME
    let rest = template.strip_prefix("for ").unwrap_or(template);
    rest.split_whitespace().next().unwrap_or("").to_string()
}

// ── Value arithmetic helpers ──────────────────────────────────────────────────

fn add_values(l: Value, r: Value) -> Result<Value, EvalError> {
    match (l, r) {
        (Value::Int(a),   Value::Int(b))   => a.checked_add(b).map(Value::Int).ok_or(EvalError::Overflow),
        (Value::U32(a),   Value::U32(b))   => a.checked_add(b).map(Value::U32).ok_or(EvalError::Overflow),
        (Value::Usize(a), Value::Usize(b)) => a.checked_add(b).map(Value::Usize).ok_or(EvalError::Overflow),
        _ => Err(EvalError::TypeError),
    }
}

fn sub_values(l: Value, r: Value) -> Result<Value, EvalError> {
    match (l, r) {
        (Value::Int(a),   Value::Int(b))   => a.checked_sub(b).map(Value::Int).ok_or(EvalError::Overflow),
        (Value::U32(a),   Value::U32(b))   => a.checked_sub(b).map(Value::U32).ok_or(EvalError::Overflow),
        (Value::Usize(a), Value::Usize(b)) => a.checked_sub(b).map(Value::Usize).ok_or(EvalError::Overflow),
        _ => Err(EvalError::TypeError),
    }
}

fn eval_child(node: &Node, idx: usize, env: &Env, grammar: &Grammar) -> Result<Value, EvalError> {
    match &node.children[idx] {
        Child::Node(n) => eval(n, env, grammar),
        Child::Hole(nt) => Err(EvalError::UnknownKind(format!("hole:{}", nt))),
    }
}

/// Evaluate a child expression in a stateful context (env is read-only here).
fn eval_expr_child(node: &Node, idx: usize, env: &Env, grammar: &Grammar) -> Result<Value, EvalError> {
    match &node.children[idx] {
        Child::Node(n) => eval(n, env, grammar),
        Child::Hole(nt) => Err(EvalError::UnknownKind(format!("hole:{}", nt))),
    }
}

fn child_node(node: &Node, idx: usize) -> Result<&Node, EvalError> {
    match &node.children[idx] {
        Child::Node(n) => Ok(n),
        Child::Hole(nt) => Err(EvalError::UnknownKind(format!("hole:{}", nt))),
    }
}

fn eval_binary_i32(node: &Node, env: &Env, grammar: &Grammar) -> Result<(i32, i32), EvalError> {
    let l = eval_child(node, 0, env, grammar)?.as_int()?;
    let r = eval_child(node, 1, env, grammar)?.as_int()?;
    Ok((l, r))
}

fn eval_binary_usize(node: &Node, env: &Env, grammar: &Grammar) -> Result<(usize, usize), EvalError> {
    let l = eval_child(node, 0, env, grammar)?.as_usize()?;
    let r = eval_child(node, 1, env, grammar)?.as_usize()?;
    Ok((l, r))
}

fn eval_binary_u32(node: &Node, env: &Env, grammar: &Grammar) -> Result<(u32, u32), EvalError> {
    let l = eval_child(node, 0, env, grammar)?.as_u32()?;
    let r = eval_child(node, 1, env, grammar)?.as_u32()?;
    Ok((l, r))
}

fn eval_binary_bool(node: &Node, env: &Env, grammar: &Grammar) -> Result<(bool, bool), EvalError> {
    let l = eval_child(node, 0, env, grammar)?.as_bool()?;
    let r = eval_child(node, 1, env, grammar)?.as_bool()?;
    Ok((l, r))
}
