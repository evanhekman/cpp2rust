//! Translate a C++ AST (processed/ format) into a Rust Node tree with Holes
//! where the translation is uncertain. The result seeds the synthesizer worklist
//! at a point far closer to the correct program than a single root Hole.
//!
//! All unrecognised constructs produce Hole nodes so top-down search can fill
//! them in using the grammar.

use crate::ast::{Child, Node};
use crate::eval::Value;
use crate::grammar::{elem_type_of, is_slice_type, Grammar, Production};
use crate::loader::Param;

struct Ctx<'a> {
    params:      &'a [Param],
    local_vars:  &'a [(String, String)],
    grammar:     &'a Grammar,
    return_type: &'a str,
    slice_names: Vec<String>,
}

impl<'a> Ctx<'a> {
    fn new(
        params:      &'a [Param],
        local_vars:  &'a [(String, String)],
        grammar:     &'a Grammar,
        return_type: &'a str,
    ) -> Self {
        let slice_names = params.iter()
            .filter(|p| is_slice_type(&p.ty))
            .map(|p| p.name.clone())
            .collect();
        Self { params, local_vars, grammar, return_type, slice_names }
    }

    fn return_nt(&self) -> String { format!("Expr_{}", self.return_type) }
}

// ── Public entry point ────────────────────────────────────────────────────────

/// Translate the top-level C++ statement list into a Rust Block Node.
/// Returns None when the AST is empty or completely untranslatable.
pub fn translate(
    ast:         &serde_json::Value,
    params:      &[Param],
    local_vars:  &[(String, String)],
    grammar:     &Grammar,
    return_type: &str,
) -> Option<Node> {
    let stmts = ast.as_array()?;
    if stmts.is_empty() { return None; }
    let ctx = Ctx::new(params, local_vars, grammar, return_type);
    match translate_block(stmts, &ctx) {
        Child::Node(n) => Some(*n),
        Child::Hole(_) => None,
    }
}

// ── Block ─────────────────────────────────────────────────────────────────────

fn translate_block(stmts: &[serde_json::Value], ctx: &Ctx) -> Child {
    let children: Vec<Child> = stmts.iter().flat_map(|s| translate_stmt(s, ctx)).collect();
    let block_kind = match children.len() {
        1 => "BlockSingle",
        2 => "BlockSeq",
        3 => "BlockSeq3",
        4 => "BlockSeq4",
        5 => "BlockSeq5",
        _ => return Child::Hole("Block".into()),
    };
    Child::Node(Box::new(Node::new(block_kind, children, 0)))
}

// ── Statement dispatch ────────────────────────────────────────────────────────

/// Returns 1 child normally, or 2 children when a C++ for-with-step expands to
/// `let mut var = 0; while var < N { body; var += step; }`.
fn translate_stmt(node: &serde_json::Value, ctx: &Ctx) -> Vec<Child> {
    // For loop: has "init" field
    if node.get("init").is_some() { return translate_for(node, ctx); }

    let child = if node.get("condition").is_some() && node.get("body").is_some() && node.get("then").is_none() {
        translate_while(node, ctx)
    } else if node.get("condition").is_some() && node.get("then").is_some() {
        translate_if(node, ctx)
    } else if node.get("catch").is_some() {
        if let Some(body) = node["body"].as_array() {
            translate_block(body, ctx)
        } else {
            Child::Hole("Stmt".into())
        }
    } else {
        let op   = match node.get("op").and_then(|v| v.as_str()) { Some(o) => o, None => return vec![Child::Hole("Stmt".into())] };
        let args = node["args"].as_array();
        match op {
            "let"        => translate_let(args, ctx),
            "return"
            | "throw"    => translate_return(args, ctx),
            "+="         => translate_compound(args, true,  ctx),
            "-="         => translate_compound(args, false, ctx),
            "++"         => translate_increment(args, true,  ctx),
            "--"         => translate_increment(args, false, ctx),
            "="          => translate_assign(args, ctx),
            _            => Child::Hole("Stmt".into()),
        }
    };
    vec![child]
}

// ── Statement translators ─────────────────────────────────────────────────────

fn translate_for(node: &serde_json::Value, ctx: &Ctx) -> Vec<Child> {
    let loop_var = node["init"]["args"].as_array()
        .and_then(|a| a.first())
        .and_then(|v| v.get("var"))
        .and_then(|v| v.as_str());
    let var_name = match loop_var { Some(v) => v, None => return vec![Child::Hole("Stmt".into())] };

    let body_stmts = match node["body"].as_array() { Some(s) => s, None => return vec![Child::Hole("Stmt".into())] };

    let upper = node["condition"]["args"].as_array().and_then(|a| a.get(1));
    let upper_child = match upper {
        Some(ub) => translate_upper_bound(ub, ctx),
        None     => Child::Hole("Expr_usize".into()),
    };

    // Detect the update step — ++ or += 1 → for loop; otherwise → while loop
    let step = for_update_step(node.get("update"));
    if step == Some(1) || step.is_none() && node.get("update").is_none() {
        // Standard for loop
        let prefix = format!("StmtFor_{}_", var_name);
        let prod   = match find_prefix(ctx.grammar, &prefix) { Some(p) => p.name.clone(), None => return vec![Child::Hole("Stmt".into())] };
        let body   = translate_block(body_stmts, ctx);
        return vec![Child::Node(Box::new(Node::new(&prod, vec![upper_child, body], 0)))];
    }

    // Non-unit step → emit: let mut var = 0; while var < upper { body; var += step; }
    let step_val = step.unwrap_or(1) as usize;

    // let mut var = 0usize
    let let_prd = match find_prefix(ctx.grammar, &format!("StmtLetMut_{}_", var_name)) {
        Some(p) => p.name.clone(), None => return vec![Child::Hole("Stmt".into())],
    };
    let zero = find_usize_lit(ctx.grammar, 0)
        .map(|n| Child::Node(Box::new(Node::new(&n, vec![], 0))))
        .unwrap_or(Child::Hole("Expr_usize".into()));
    let let_init = Child::Node(Box::new(Node::new(&let_prd, vec![zero], 0)));

    // while condition: var < upper
    let local_idx = ctx.local_vars.iter().position(|(n, _)| n == var_name);
    let var_child = match local_idx {
        Some(i) => Child::Node(Box::new(Node::new(&format!("ExprIdent_local_{}", i), vec![], 0))),
        None    => Child::Hole("Expr_usize".into()),
    };
    let cond = if ctx.grammar.values().flatten().any(|p| p.name == "ExprLt_usize") {
        Child::Node(Box::new(Node::new("ExprLt_usize", vec![var_child, upper_child], 0)))
    } else {
        Child::Hole("Expr_bool".into())
    };

    // update statement: var += step_val
    let step_prd = find_prefix(ctx.grammar, &format!("StmtCompoundPlus_{}_", var_name)).map(|p| p.name.clone());
    let step_lit = find_usize_lit(ctx.grammar, step_val)
        .map(|n| Child::Node(Box::new(Node::new(&n, vec![], 0))))
        .unwrap_or(Child::Hole("Expr_usize".into()));
    let update_child = match step_prd {
        Some(prd) => Child::Node(Box::new(Node::new(&prd, vec![step_lit], 0))),
        None      => Child::Hole("Stmt".into()),
    };

    // while body: original body stmts + update
    let mut body_children: Vec<Child> = body_stmts.iter()
        .flat_map(|s| translate_stmt(s, ctx))
        .collect();
    body_children.push(update_child);
    let body_kind = match body_children.len() {
        1 => "BlockSingle", 2 => "BlockSeq", 3 => "BlockSeq3", 4 => "BlockSeq4", 5 => "BlockSeq5",
        _ => return vec![Child::Hole("Stmt".into())],
    };
    let body = Child::Node(Box::new(Node::new(body_kind, body_children, 0)));

    let while_node = Child::Node(Box::new(Node::new("StmtWhile", vec![cond, body], 0)));
    vec![let_init, while_node]
}

/// Returns the integer step of a C++ for-loop update, or None if unrecognised.
/// `++` and `+= 1` both return Some(1).
fn for_update_step(update: Option<&serde_json::Value>) -> Option<i64> {
    let u = update?;
    let op = u.get("op").and_then(|v| v.as_str())?;
    match op {
        "++" | "--" => Some(1),
        "+=" | "-=" => {
            u["args"].as_array()?.get(1)?.get("lit")?.as_str()?.parse::<i64>().ok()
        }
        _ => None,
    }
}

fn translate_while(node: &serde_json::Value, ctx: &Ctx) -> Child {
    let cond = translate_expr(&node["condition"], "Expr_bool", ctx);
    let body_stmts = match node["body"].as_array() { Some(s) => s, None => return Child::Hole("Stmt".into()) };
    let body = translate_block(body_stmts, ctx);
    Child::Node(Box::new(Node::new("StmtWhile", vec![cond, body], 0)))
}

fn translate_if(node: &serde_json::Value, ctx: &Ctx) -> Child {
    let cond = translate_expr(&node["condition"], "Expr_bool", ctx);
    let then_stmts = match node["then"].as_array() { Some(s) => s, None => return Child::Hole("Stmt".into()) };
    let then_block = translate_block(then_stmts, ctx);

    if let Some(else_stmts) = node.get("else").and_then(|v| v.as_array()) {
        let else_block = translate_block(else_stmts, ctx);
        return Child::Node(Box::new(Node::new("StmtIfElse", vec![cond, then_block, else_block], 0)));
    }
    Child::Node(Box::new(Node::new("StmtIf", vec![cond, then_block], 0)))
}

fn translate_let(args: Option<&Vec<serde_json::Value>>, ctx: &Ctx) -> Child {
    let args     = match args { Some(a) => a, None => return Child::Hole("Stmt".into()) };
    let var_name = match args.first().and_then(|v| v.get("var")).and_then(|v| v.as_str()) {
        Some(n) => n, None => return Child::Hole("Stmt".into())
    };
    let nt  = var_nt(var_name, ctx);
    let prd = match find_prefix(ctx.grammar, &format!("StmtLetMut_{}_", var_name)) {
        Some(p) => p.name.clone(), None => return Child::Hole("Stmt".into())
    };
    let init = if args.len() > 1 { translate_expr(&args[1], &nt, ctx) } else { Child::Hole(nt) };
    Child::Node(Box::new(Node::new(&prd, vec![init], 0)))
}

fn translate_return(args: Option<&Vec<serde_json::Value>>, ctx: &Ctx) -> Child {
    let ret_nt = ctx.return_nt();
    let expr = args.and_then(|a| a.first())
        .map(|e| translate_expr(e, &ret_nt, ctx))
        .unwrap_or(Child::Hole(ret_nt));
    Child::Node(Box::new(Node::new("StmtReturn", vec![expr], 0)))
}

fn translate_compound(args: Option<&Vec<serde_json::Value>>, is_add: bool, ctx: &Ctx) -> Child {
    let args     = match args { Some(a) => a, None => return Child::Hole("Stmt".into()) };
    let var_name = match args.first().and_then(|v| v.get("var")).and_then(|v| v.as_str()) {
        Some(n) => n, None => return Child::Hole("Stmt".into())
    };
    let kind = if is_add { "StmtCompoundPlus_" } else { "StmtCompoundMinus_" };
    let prd  = match find_prefix(ctx.grammar, &format!("{}{}_", kind, var_name)) {
        Some(p) => p.name.clone(), None => return Child::Hole("Stmt".into())
    };
    let nt  = var_nt(var_name, ctx);
    let rhs = if args.len() > 1 { translate_expr(&args[1], &nt, ctx) } else { Child::Hole(nt) };
    Child::Node(Box::new(Node::new(&prd, vec![rhs], 0)))
}

fn translate_increment(args: Option<&Vec<serde_json::Value>>, is_inc: bool, ctx: &Ctx) -> Child {
    let args     = match args { Some(a) => a, None => return Child::Hole("Stmt".into()) };
    let var_name = match args.first().and_then(|v| v.get("var")).and_then(|v| v.as_str()) {
        Some(n) => n, None => return Child::Hole("Stmt".into())
    };
    let kind = if is_inc { "StmtCompoundPlus_" } else { "StmtCompoundMinus_" };
    let prd  = match find_prefix(ctx.grammar, &format!("{}{}_", kind, var_name)) {
        Some(p) => p.name.clone(), None => return Child::Hole("Stmt".into())
    };
    let nt      = var_nt(var_name, ctx);
    let one_prd = if nt == "Expr_usize" { find_usize_lit(ctx.grammar, 1) } else { find_i32_lit(ctx.grammar, 1) };
    let rhs     = match one_prd { Some(n) => Child::Node(Box::new(Node::new(&n, vec![], 0))), None => Child::Hole(nt) };
    Child::Node(Box::new(Node::new(&prd, vec![rhs], 0)))
}

fn translate_assign(args: Option<&Vec<serde_json::Value>>, ctx: &Ctx) -> Child {
    let args = match args { Some(a) => a, None => return Child::Hole("Stmt".into()) };
    let lhs  = match args.first() { Some(l) => l, None => return Child::Hole("Stmt".into()) };

    // Pointer dereference lvalue: *ptr = expr → a[ptr] = expr
    if lhs.get("op").and_then(|v| v.as_str()) == Some("*") {
        if let Some(ptr_var) = lhs["args"].as_array()
            .and_then(|a| a.first()).and_then(|v| v.get("var")).and_then(|v| v.as_str())
        {
            if let Some(local_idx) = ctx.local_vars.iter().position(|(n, _)| n == ptr_var) {
                // Find slice param whose elem type we can assign to
                if let Some(slice_p) = ctx.params.iter().find(|p| is_slice_type(&p.ty)) {
                    let elem_ty = elem_type_of(&slice_p.ty).unwrap_or("i32".into());
                    let elem_nt = format!("Expr_{}", elem_ty);
                    let prd     = format!("StmtSliceAssign_{}", slice_p.name);
                    if ctx.grammar.values().flatten().any(|p| p.name == prd) {
                        let idx_prd = format!("ExprIdent_local_{}", local_idx);
                        let idx     = Child::Node(Box::new(Node::new(&idx_prd, vec![], 0)));
                        let val     = args.get(1).map(|e| translate_expr(e, &elem_nt, ctx)).unwrap_or(Child::Hole(elem_nt));
                        return Child::Node(Box::new(Node::new(&prd, vec![idx, val], 0)));
                    }
                }
            }
        }
    }

    // Slice assignment: a[i] = expr
    if lhs.get("op").and_then(|v| v.as_str()) == Some("[]") {
        if let Some(arr_args) = lhs["args"].as_array() {
            if let Some(arr_name) = arr_args.first().and_then(|v| v.get("var")).and_then(|v| v.as_str()) {
                let prd = format!("StmtSliceAssign_{}", arr_name);
                if ctx.grammar.values().flatten().any(|p| p.name == prd) {
                    let elem_ty = ctx.params.iter().find(|p| p.name == arr_name)
                        .and_then(|p| elem_type_of(&p.ty)).unwrap_or("i32".into());
                    let elem_nt = format!("Expr_{}", elem_ty);
                    let idx = arr_args.get(1).map(|e| translate_expr(e, "Expr_usize", ctx)).unwrap_or(Child::Hole("Expr_usize".into()));
                    let val = args.get(1).map(|e| translate_expr(e, &elem_nt, ctx)).unwrap_or(Child::Hole(elem_nt));
                    return Child::Node(Box::new(Node::new(&prd, vec![idx, val], 0)));
                }
            }
        }
    }

    // Variable assignment
    if let Some(var_name) = lhs.get("var").and_then(|v| v.as_str()) {
        if let Some(prd) = find_prefix(ctx.grammar, &format!("StmtAssign_{}_", var_name)) {
            let prd = prd.name.clone();
            let nt  = var_nt(var_name, ctx);
            let rhs = args.get(1).map(|e| translate_expr(e, &nt, ctx)).unwrap_or(Child::Hole(nt));
            return Child::Node(Box::new(Node::new(&prd, vec![rhs], 0)));
        }
    }
    Child::Hole("Stmt".into())
}

// ── Expression translator ─────────────────────────────────────────────────────

fn translate_expr(node: &serde_json::Value, expected_nt: &str, ctx: &Ctx) -> Child {
    if let Some(lit) = node.get("lit").and_then(|v| v.as_str()) {
        return translate_lit(lit, expected_nt, ctx);
    }
    if let Some(name) = node.get("var").and_then(|v| v.as_str()) {
        return translate_var(name, expected_nt, ctx);
    }
    let op    = match node.get("op").and_then(|v| v.as_str()) { Some(o) => o, None => return Child::Hole(expected_nt.into()) };
    let args  = node["args"].as_array();
    let arity = args.map(|a| a.len()).unwrap_or(0);

    match op {
        "[]"             => translate_index(args, expected_nt, ctx),
        "+"  if arity>1  => translate_binop(args, "Add",  expected_nt, ctx),
        "-"  if arity==2 => translate_binop(args, "Sub",  expected_nt, ctx),
        "-"  if arity==1 => {
            // Unary negation: -x → (0 - x)
            let zero = find_i32_lit(ctx.grammar, 0)
                .map(|n| Child::Node(Box::new(Node::new(&n, vec![], 0))))
                .unwrap_or(Child::Hole(expected_nt.into()));
            let operand = args.and_then(|a| a.first())
                .map(|e| translate_expr(e, expected_nt, ctx))
                .unwrap_or(Child::Hole(expected_nt.into()));
            if ctx.grammar.values().flatten().any(|p| p.name == "ExprSub") {
                Child::Node(Box::new(Node::new("ExprSub", vec![zero, operand], 0)))
            } else {
                Child::Hole(expected_nt.into())
            }
        }
        "*"  if arity==2 => translate_binop(args, "Mul",  expected_nt, ctx),
        "/"              => translate_binop(args, "Div",  expected_nt, ctx),
        "%"              => translate_binop(args, "Mod",  expected_nt, ctx),
        "*"  if arity==1 => {
            // Pointer dereference *ptr → ExprIndex_slice(ptr_ident)
            let ptr_var = args.and_then(|a| a.first())
                .and_then(|v| v.get("var")).and_then(|v| v.as_str());
            match ptr_var {
                Some(name) => translate_deref(name, expected_nt, ctx),
                None       => Child::Hole(expected_nt.into()),
            }
        }
        "<" | ">" | "<=" | ">=" | "==" | "!=" => translate_cmp(args, op, ctx),
        "?:" => translate_ternary(args, expected_nt, ctx),
        "&&" => two_bool("ExprAnd", args, ctx),
        "||" => two_bool("ExprOr",  args, ctx),
        "!"  => {
            let e = args.and_then(|a| a.first())
                .map(|x| translate_expr(x, "Expr_bool", ctx))
                .unwrap_or(Child::Hole("Expr_bool".into()));
            Child::Node(Box::new(Node::new("ExprNot", vec![e], 0)))
        }
        _ => Child::Hole(expected_nt.into()),
    }
}

fn translate_lit(lit: &str, expected_nt: &str, ctx: &Ctx) -> Child {
    let prod_name = match expected_nt {
        "Expr_usize" => lit.parse::<i64>().ok().filter(|&n| n >= 0).and_then(|n| find_usize_lit(ctx.grammar, n as usize)),
        "Expr_u32"   => lit.parse::<i64>().ok().filter(|&n| n >= 0).and_then(|n| find_u32_lit(ctx.grammar, n as u32)),
        "Expr_bool"  => {
            let val = lit == "true";
            ctx.grammar.get("Expr_bool").and_then(|ps| ps.iter().find(|p| p.literal_value == Some(Value::Bool(val)))).map(|p| p.name.clone())
        }
        _ => lit.parse::<i32>().ok().and_then(|n| find_i32_lit(ctx.grammar, n)).filter(|name| {
            ctx.grammar.get(expected_nt).map(|ps| ps.iter().any(|p| &p.name == name)).unwrap_or(false)
        }),
    };
    match prod_name {
        Some(n) => Child::Node(Box::new(Node::new(&n, vec![], 0))),
        None    => Child::Hole(expected_nt.into()),
    }
}

fn translate_var(name: &str, expected_nt: &str, ctx: &Ctx) -> Child {
    // Slice param used directly as a variable → it's a pointer, not usable as usize/value.
    // Only unknown variables (e.g. 'n') should fall through to translate_len_var.
    if ctx.slice_names.contains(&name.to_string()) {
        return Child::Hole(expected_nt.into());
    }
    // Local variable
    for (idx, (vname, _)) in ctx.local_vars.iter().enumerate() {
        if vname == name {
            let prd = format!("ExprIdent_local_{}", idx);
            if ctx.grammar.values().flatten().any(|p| p.name == prd) {
                return Child::Node(Box::new(Node::new(&prd, vec![], 0)));
            }
        }
    }
    // Scalar parameter
    for (i, p) in ctx.params.iter().enumerate() {
        if p.name == name && !is_slice_type(&p.ty) {
            let prd = format!("ExprIdent_{}", i);
            if ctx.grammar.values().flatten().any(|p2| p2.name == prd) {
                return Child::Node(Box::new(Node::new(&prd, vec![], 0)));
            }
        }
    }
    // Unknown variable in usize context → try ExprLen of first slice
    if expected_nt == "Expr_usize" {
        return translate_len_var(ctx);
    }
    Child::Hole(expected_nt.into())
}

fn translate_len_var(ctx: &Ctx) -> Child {
    for slice_name in &ctx.slice_names {
        let prd = format!("ExprLen_{}", slice_name);
        if ctx.grammar.values().flatten().any(|p| p.name == prd) {
            return Child::Node(Box::new(Node::new(&prd, vec![], 0)));
        }
    }
    Child::Hole("Expr_usize".into())
}

fn translate_upper_bound(node: &serde_json::Value, ctx: &Ctx) -> Child {
    if let Some(name) = node.get("var").and_then(|v| v.as_str()) {
        return translate_var(name, "Expr_usize", ctx);
    }
    if let Some(lit) = node.get("lit").and_then(|v| v.as_str()) {
        return translate_lit(lit, "Expr_usize", ctx);
    }
    translate_expr(node, "Expr_usize", ctx)
}

fn translate_index(args: Option<&Vec<serde_json::Value>>, expected_nt: &str, ctx: &Ctx) -> Child {
    let args     = match args { Some(a) => a, None => return Child::Hole(expected_nt.into()) };
    let arr_name = match args.first().and_then(|v| v.get("var")).and_then(|v| v.as_str()) {
        Some(n) => n, None => return Child::Hole(expected_nt.into())
    };
    let prd = format!("ExprIndex_{}", arr_name);
    // Check the production exists and produces the expected type
    let prod_nt = ctx.grammar.values().flatten().find(|p| p.name == prd).map(|p| p.nonterminal.as_str());
    match prod_nt {
        None => return Child::Hole(expected_nt.into()),
        Some(nt) if nt != expected_nt => return Child::Hole(expected_nt.into()),
        _ => {}
    }
    let idx = args.get(1).map(|e| translate_expr(e, "Expr_usize", ctx)).unwrap_or(Child::Hole("Expr_usize".into()));
    Child::Node(Box::new(Node::new(&prd, vec![idx], 0)))
}

fn translate_binop(args: Option<&Vec<serde_json::Value>>, op: &str, expected_nt: &str, ctx: &Ctx) -> Child {
    // Pointer arithmetic (slice + offset or slice - offset) → translate as ExprLen_slice
    if expected_nt == "Expr_usize" && matches!(op, "Add" | "Sub") {
        if args.map(|a| a.iter().any(|x| is_ptr_arg(x, ctx))).unwrap_or(false) {
            return translate_len_var(ctx);
        }
    }
    let suffix = match expected_nt { "Expr_i32" => "", "Expr_usize" => "_usize", "Expr_u32" => "_u32", _ => return Child::Hole(expected_nt.into()) };
    let prd    = format!("Expr{}{}", op, suffix);
    if !ctx.grammar.values().flatten().any(|p| p.name == prd) { return Child::Hole(expected_nt.into()); }
    let l = args.and_then(|a| a.first()).map(|e| translate_expr(e, expected_nt, ctx)).unwrap_or(Child::Hole(expected_nt.into()));
    let r = args.and_then(|a| a.get(1)).map(|e| translate_expr(e, expected_nt, ctx)).unwrap_or(Child::Hole(expected_nt.into()));
    Child::Node(Box::new(Node::new(&prd, vec![l, r], 0)))
}

fn translate_ternary(args: Option<&Vec<serde_json::Value>>, expected_nt: &str, ctx: &Ctx) -> Child {
    let suffix = expected_nt.strip_prefix("Expr_").unwrap_or("i32");
    let kind = format!("ExprIfElse_{}", suffix);
    if !ctx.grammar.values().flatten().any(|p| p.name == kind) {
        return Child::Hole(expected_nt.into());
    }
    let cond      = args.and_then(|a| a.first()).map(|e| translate_expr(e, "Expr_bool", ctx)).unwrap_or(Child::Hole("Expr_bool".into()));
    let then_val  = args.and_then(|a| a.get(1)).map(|e| translate_expr(e, expected_nt, ctx)).unwrap_or(Child::Hole(expected_nt.into()));
    let else_val  = args.and_then(|a| a.get(2)).map(|e| translate_expr(e, expected_nt, ctx)).unwrap_or(Child::Hole(expected_nt.into()));
    Child::Node(Box::new(Node::new(&kind, vec![cond, then_val, else_val], 0)))
}

fn translate_cmp(args: Option<&Vec<serde_json::Value>>, op: &str, ctx: &Ctx) -> Child {
    let operand_nt = infer_operand_nt(args.and_then(|a| a.first()), ctx);
    let suffix     = match operand_nt.as_str() { "Expr_i32" => "", "Expr_usize" => "_usize", _ => return Child::Hole("Expr_bool".into()) };
    let prd        = match op { "<" => format!("ExprLt{}", suffix), ">" => format!("ExprGt{}", suffix), "<=" => format!("ExprLe{}", suffix), ">=" => format!("ExprGe{}", suffix), "==" => format!("ExprEq{}", suffix), "!=" => format!("ExprNe{}", suffix), _ => return Child::Hole("Expr_bool".into()) };
    if !ctx.grammar.values().flatten().any(|p| p.name == prd) { return Child::Hole("Expr_bool".into()); }
    let l = args.and_then(|a| a.first()).map(|e| translate_expr(e, &operand_nt, ctx)).unwrap_or(Child::Hole(operand_nt.clone()));
    let r = args.and_then(|a| a.get(1)).map(|e| translate_expr(e, &operand_nt, ctx)).unwrap_or(Child::Hole(operand_nt));
    Child::Node(Box::new(Node::new(&prd, vec![l, r], 0)))
}

fn two_bool(kind: &str, args: Option<&Vec<serde_json::Value>>, ctx: &Ctx) -> Child {
    let l = args.and_then(|a| a.first()).map(|e| translate_expr(e, "Expr_bool", ctx)).unwrap_or(Child::Hole("Expr_bool".into()));
    let r = args.and_then(|a| a.get(1)).map(|e| translate_expr(e, "Expr_bool", ctx)).unwrap_or(Child::Hole("Expr_bool".into()));
    Child::Node(Box::new(Node::new(kind, vec![l, r], 0)))
}

/// Peek at an expression to infer its nonterminal type for comparison operands.
fn infer_operand_nt(node: Option<&serde_json::Value>, ctx: &Ctx) -> String {
    let node = match node { Some(n) => n, None => return "Expr_i32".into() };
    if let Some(name) = node.get("var").and_then(|v| v.as_str()) {
        if let Some((_, ty)) = ctx.local_vars.iter().find(|(n, _)| n == name) {
            return format!("Expr_{}", ty);
        }
        for p in ctx.params.iter() {
            if p.name == name && !is_slice_type(&p.ty) {
                return format!("Expr_{}", p.ty.trim_start_matches('&'));
            }
        }
        return "Expr_usize".into(); // unknown var likely a loop index
    }
    if node.get("op").and_then(|v| v.as_str()) == Some("[]") {
        if let Some(arr_name) = node["args"].as_array()
            .and_then(|a| a.first()).and_then(|v| v.get("var")).and_then(|v| v.as_str())
        {
            if let Some(p) = ctx.params.iter().find(|p| p.name == arr_name) {
                if let Some(elem) = elem_type_of(&p.ty) { return format!("Expr_{}", elem); }
            }
        }
    }
    "Expr_i32".into()
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Return the expected nonterminal for a variable by looking up local_vars then params.
fn var_nt(name: &str, ctx: &Ctx) -> String {
    if let Some((_, ty)) = ctx.local_vars.iter().find(|(n, _)| n == name) {
        return format!("Expr_{}", ty);
    }
    for p in ctx.params.iter() {
        if p.name == name && !is_slice_type(&p.ty) {
            return format!("Expr_{}", p.ty.trim_start_matches('&'));
        }
    }
    format!("Expr_{}", ctx.return_type)
}

fn find_prefix<'a>(grammar: &'a Grammar, prefix: &str) -> Option<&'a Production> {
    grammar.values().flatten().find(|p| p.name.starts_with(prefix))
}

fn find_i32_lit(grammar: &Grammar, val: i32) -> Option<String> {
    grammar.get("Expr_i32")?.iter()
        .find(|p| p.literal_value == Some(Value::Int(val)))
        .map(|p| p.name.clone())
}

fn find_usize_lit(grammar: &Grammar, val: usize) -> Option<String> {
    grammar.get("Expr_usize")?.iter()
        .find(|p| p.literal_value == Some(Value::Usize(val)))
        .map(|p| p.name.clone())
}

fn find_u32_lit(grammar: &Grammar, val: u32) -> Option<String> {
    grammar.get("Expr_u32")?.iter()
        .find(|p| p.literal_value == Some(Value::U32(val)))
        .map(|p| p.name.clone())
}

/// Returns true if `node` refers to a slice parameter (a C++ pointer arg).
fn is_ptr_arg(node: &serde_json::Value, ctx: &Ctx) -> bool {
    node.get("var")
        .and_then(|v| v.as_str())
        .map(|name| ctx.slice_names.contains(&name.to_string()))
        .unwrap_or(false)
}

/// Translate a pointer dereference `*ptr` where `ptr` is a local variable.
/// Maps to `ExprIndex_{slice}(ExprIdent_local_{idx})`.
fn translate_deref(ptr_name: &str, expected_nt: &str, ctx: &Ctx) -> Child {
    // Find the local variable index for ptr_name
    let local_idx = match ctx.local_vars.iter().position(|(n, _)| n == ptr_name) {
        Some(i) => i,
        None => return Child::Hole(expected_nt.into()),
    };
    // Find a slice param whose element type matches expected_nt
    for slice_p in ctx.params.iter().filter(|p| is_slice_type(&p.ty)) {
        let elem_ty = match elem_type_of(&slice_p.ty) { Some(t) => t, None => continue };
        let elem_nt = format!("Expr_{}", elem_ty);
        if elem_nt != expected_nt { continue; }
        let prd = format!("ExprIndex_{}", slice_p.name);
        if ctx.grammar.values().flatten().any(|p| p.name == prd) {
            let idx_prd = format!("ExprIdent_local_{}", local_idx);
            let idx = Child::Node(Box::new(Node::new(&idx_prd, vec![], 0)));
            return Child::Node(Box::new(Node::new(&prd, vec![idx], 0)));
        }
    }
    Child::Hole(expected_nt.into())
}
