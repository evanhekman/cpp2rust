use crate::eval::Value;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Production {
    pub name: String,
    #[allow(dead_code)]
    pub nonterminal: String,
    pub children_spec: Vec<String>,
    pub rust_template: String,
    pub literal_value: Option<Value>, // pre-parsed for eval hot path
}

pub type Grammar = HashMap<String, Vec<Production>>;

// ── Slice type helpers ────────────────────────────────────────────────────────

/// Extract the element type from a slice type string.
/// "&[i32]" → "i32",  "&mut [i32]" → "i32",  "&[u8]" → "u8"
pub fn elem_type_of(rust_type: &str) -> Option<String> {
    let inner = if let Some(s) = rust_type.strip_prefix("&mut [") {
        s.strip_suffix(']')?
    } else if let Some(s) = rust_type.strip_prefix("&[") {
        s.strip_suffix(']')?
    } else {
        return None;
    };
    Some(inner.to_string())
}

pub fn is_slice_type(rust_type: &str) -> bool {
    rust_type.starts_with("&[") || rust_type.starts_with("&mut [")
}

pub fn is_mut_slice(rust_type: &str) -> bool {
    rust_type.starts_with("&mut [")
}

// ── Grammar builder ───────────────────────────────────────────────────────────

/// Build the base grammar given:
///   literals     — integer/bool symbol literals (from symbols.txt)
///   params       — function parameters (name, type)
///   local_vars   — local variables extracted from the C++ AST: (name, rust_type)
///   return_type  — Rust return type string; "()" means void
pub fn build_grammar(
    literals: &[String],
    params: &[crate::loader::Param],
    local_vars: &[(String, String)],
    return_type: &str,
) -> Grammar {
    let mut g: Grammar = HashMap::new();

    // ── Block ─────────────────────────────────────────────────────────────────
    g.entry("Block".into()).or_default().extend([
        prod("BlockSingle",  "Block", vec!["Stmt"],                               "{ {0} }"),
        prod("BlockSeq",     "Block", vec!["Stmt", "Stmt"],                       "{ {0} {1} }"),
        prod("BlockSeq3",    "Block", vec!["Stmt", "Stmt", "Stmt"],               "{ {0} {1} {2} }"),
        prod("BlockSeq4",    "Block", vec!["Stmt","Stmt","Stmt","Stmt"],           "{ {0} {1} {2} {3} }"),
        prod("BlockSeq5",    "Block", vec!["Stmt","Stmt","Stmt","Stmt","Stmt"],    "{ {0} {1} {2} {3} {4} }"),
    ]);

    // ── Stmt ──────────────────────────────────────────────────────────────────
    g.entry("Stmt".into()).or_default().extend([
        prod("StmtIf",       "Stmt", vec!["Expr_bool", "Block"],                   "if {0} {1}"),
        prod("StmtIfElse",   "Stmt", vec!["Expr_bool", "Block", "Block"],          "if {0} {1} else {2}"),
        prod("StmtWhile",    "Stmt", vec!["Expr_bool", "Block"],                   "while {0} {1}"),
    ]);

    // StmtReturn — only when function is non-void
    if return_type != "()" && !return_type.is_empty() {
        let ret_nt = format!("Expr_{}", return_type);
        g.entry("Stmt".into()).or_default().push(Production {
            name: "StmtReturn".into(),
            nonterminal: "Stmt".into(),
            children_spec: vec![ret_nt],
            rust_template: "return {0};".into(),
            literal_value: None,
        });
    }

    // ── Expr_bool ─────────────────────────────────────────────────────────────
    // i32 comparisons
    g.entry("Expr_bool".into()).or_default().extend([
        prod("ExprEq",  "Expr_bool", vec!["Expr_i32","Expr_i32"], "({0} == {1})"),
        prod("ExprNe",  "Expr_bool", vec!["Expr_i32","Expr_i32"], "({0} != {1})"),
        prod("ExprLt",  "Expr_bool", vec!["Expr_i32","Expr_i32"], "({0} < {1})"),
        prod("ExprGt",  "Expr_bool", vec!["Expr_i32","Expr_i32"], "({0} > {1})"),
        prod("ExprLe",  "Expr_bool", vec!["Expr_i32","Expr_i32"], "({0} <= {1})"),
        prod("ExprGe",  "Expr_bool", vec!["Expr_i32","Expr_i32"], "({0} >= {1})"),
    ]);
    // usize comparisons
    g.entry("Expr_bool".into()).or_default().extend([
        prod("ExprEq_usize", "Expr_bool", vec!["Expr_usize","Expr_usize"], "({0} == {1})"),
        prod("ExprNe_usize", "Expr_bool", vec!["Expr_usize","Expr_usize"], "({0} != {1})"),
        prod("ExprLt_usize", "Expr_bool", vec!["Expr_usize","Expr_usize"], "({0} < {1})"),
        prod("ExprGt_usize", "Expr_bool", vec!["Expr_usize","Expr_usize"], "({0} > {1})"),
        prod("ExprLe_usize", "Expr_bool", vec!["Expr_usize","Expr_usize"], "({0} <= {1})"),
        prod("ExprGe_usize", "Expr_bool", vec!["Expr_usize","Expr_usize"], "({0} >= {1})"),
    ]);
    // logical
    g.entry("Expr_bool".into()).or_default().extend([
        prod("ExprAnd",      "Expr_bool", vec!["Expr_bool","Expr_bool"], "({0} && {1})"),
        prod("ExprOr",       "Expr_bool", vec!["Expr_bool","Expr_bool"], "({0} || {1})"),
        prod("ExprNot",      "Expr_bool", vec!["Expr_bool"],             "(!{0})"),
        prod("ExprIfElse_bool", "Expr_bool",
             vec!["Expr_bool","Expr_bool","Expr_bool"], "if {0} { {1} } else { {2} }"),
    ]);

    // ── Expr_i32 ──────────────────────────────────────────────────────────────
    g.entry("Expr_i32".into()).or_default().extend([
        prod("ExprAdd",  "Expr_i32", vec!["Expr_i32","Expr_i32"], "({0} + {1})"),
        prod("ExprSub",  "Expr_i32", vec!["Expr_i32","Expr_i32"], "({0} - {1})"),
        prod("ExprMul",  "Expr_i32", vec!["Expr_i32","Expr_i32"], "({0} * {1})"),
        prod("ExprDiv",  "Expr_i32", vec!["Expr_i32","Expr_i32"], "({0} / {1})"),
        prod("ExprMod",  "Expr_i32", vec!["Expr_i32","Expr_i32"], "({0} % {1})"),
        prod("ExprIfElse_i32", "Expr_i32",
             vec!["Expr_bool","Expr_i32","Expr_i32"], "if {0} { {1} } else { {2} }"),
        // cast from usize (e.g. return loop index as i32)
        prod("ExprCast_i32", "Expr_i32", vec!["Expr_usize"], "({0} as i32)"),
    ]);

    // ── Expr_usize ────────────────────────────────────────────────────────────
    g.entry("Expr_usize".into()).or_default().extend([
        prod("ExprAdd_usize", "Expr_usize", vec!["Expr_usize","Expr_usize"], "({0} + {1})"),
        prod("ExprSub_usize", "Expr_usize", vec!["Expr_usize","Expr_usize"], "({0} - {1})"),
        prod("ExprMul_usize", "Expr_usize", vec!["Expr_usize","Expr_usize"], "({0} * {1})"),
    ]);

    // ── Expr_u32 ──────────────────────────────────────────────────────────────
    g.entry("Expr_u32".into()).or_default().extend([
        prod("ExprAdd_u32", "Expr_u32", vec!["Expr_u32","Expr_u32"], "({0} + {1})"),
        prod("ExprMul_u32", "Expr_u32", vec!["Expr_u32","Expr_u32"], "({0} * {1})"),
        // casts to u32
        prod("ExprCast_u32_u8",    "Expr_u32", vec!["Expr_u8"],    "({0} as u32)"),
        prod("ExprCast_u32_usize", "Expr_u32", vec!["Expr_usize"], "({0} as u32)"),
        prod("ExprCast_u32_i32",   "Expr_u32", vec!["Expr_i32"],   "({0} as u32)"),
    ]);

    // ── Literals → typed Expr leaves ──────────────────────────────────────────
    const BOOL_LITERALS: &[&str] = &["true", "false"];

    for (i, lit) in literals.iter().enumerate() {
        if BOOL_LITERALS.contains(&lit.as_str()) {
            let val = Some(Value::Bool(lit == "true"));
            g.entry("Expr_bool".into()).or_default().push(Production {
                name: format!("ExprLit_{}", i),
                nonterminal: "Expr_bool".into(),
                children_spec: vec![],
                rust_template: lit.clone(),
                literal_value: val,
            });
        } else if let Ok(n) = lit.parse::<i32>() {
            // i32 literal
            g.entry("Expr_i32".into()).or_default().push(Production {
                name: format!("ExprLit_{}", i),
                nonterminal: "Expr_i32".into(),
                children_spec: vec![],
                rust_template: lit.clone(),
                literal_value: Some(Value::Int(n)),
            });
            // Non-negative literals also serve as usize and u32
            if n >= 0 {
                let nu = n as usize;
                g.entry("Expr_usize".into()).or_default().push(Production {
                    name: format!("ExprLit_usize_{}", i),
                    nonterminal: "Expr_usize".into(),
                    children_spec: vec![],
                    rust_template: format!("{}usize", n),
                    literal_value: Some(Value::Usize(nu)),
                });
                let nu32 = n as u32;
                g.entry("Expr_u32".into()).or_default().push(Production {
                    name: format!("ExprLit_u32_{}", i),
                    nonterminal: "Expr_u32".into(),
                    children_spec: vec![],
                    rust_template: format!("{}u32", n),
                    literal_value: Some(Value::U32(nu32)),
                });
            }
        }
    }

    // ── Parameters ────────────────────────────────────────────────────────────
    for (i, param) in params.iter().enumerate() {
        if let Some(elem_ty) = elem_type_of(&param.ty) {
            // Slice parameter
            let elem_nt = format!("Expr_{}", elem_ty);

            // a[idx] → Expr_{elem}
            g.entry(elem_nt.clone()).or_default().push(Production {
                name: format!("ExprIndex_{}", param.name),
                nonterminal: elem_nt.clone(),
                children_spec: vec!["Expr_usize".into()],
                rust_template: format!("{}[{{0}}]", param.name),
                literal_value: None,
            });

            // a.len() → Expr_usize
            g.entry("Expr_usize".into()).or_default().push(Production {
                name: format!("ExprLen_{}", param.name),
                nonterminal: "Expr_usize".into(),
                children_spec: vec![],
                rust_template: format!("{}.len()", param.name),
                literal_value: None,
            });

            // Mutable slices: a[idx] = expr; → Stmt
            if is_mut_slice(&param.ty) {
                g.entry("Stmt".into()).or_default().push(Production {
                    name: format!("StmtSliceAssign_{}", param.name),
                    nonterminal: "Stmt".into(),
                    children_spec: vec!["Expr_usize".into(), elem_nt],
                    rust_template: format!("{}[{{0}}] = {{1}};", param.name),
                    literal_value: None,
                });
            }
        } else {
            // Scalar parameter (existing behaviour)
            let nt = match param.ty.as_str() {
                "&i32" => "Expr_i32".to_string(),
                "Option<&i32>" => "Expr_opt_i32".to_string(),
                other => format!("Expr_{}", other),
            };
            g.entry(nt.clone()).or_default().push(Production {
                name: format!("ExprIdent_{}", i),
                nonterminal: nt,
                children_spec: vec![],
                rust_template: param.name.clone(),
                literal_value: None,
            });
        }
    }

    // Option<&i32> operations (kept for dataset0 compat)
    if params.iter().any(|p| p.ty == "Option<&i32>") {
        g.entry("Expr_bool".into()).or_default().extend([
            Production {
                name: "ExprOptIsSome".into(), nonterminal: "Expr_bool".into(),
                children_spec: vec!["Expr_opt_i32".into()],
                rust_template: "{0}.is_some()".into(), literal_value: None,
            },
            Production {
                name: "ExprOptIsNone".into(), nonterminal: "Expr_bool".into(),
                children_spec: vec!["Expr_opt_i32".into()],
                rust_template: "{0}.is_none()".into(), literal_value: None,
            },
        ]);
        g.entry("Expr_i32".into()).or_default().push(Production {
            name: "ExprOptUnwrapOr".into(), nonterminal: "Expr_i32".into(),
            children_spec: vec!["Expr_opt_i32".into(), "Expr_i32".into()],
            rust_template: "{0}.unwrap_or({1})".into(), literal_value: None,
        });
    }

    // ── Local variables ───────────────────────────────────────────────────────
    for (idx, (name, ty)) in local_vars.iter().enumerate() {
        let nt = format!("Expr_{}", ty);

        // Read access
        g.entry(nt.clone()).or_default().push(Production {
            name: format!("ExprIdent_local_{}", idx),
            nonterminal: nt.clone(),
            children_spec: vec![],
            rust_template: name.clone(),
            literal_value: None,
        });

        // let mut name: ty = expr;
        g.entry("Stmt".into()).or_default().push(Production {
            name: format!("StmtLetMut_{}_{}", name, idx),
            nonterminal: "Stmt".into(),
            children_spec: vec![nt.clone()],
            rust_template: format!("let mut {}: {} = {{0}};", name, ty),
            literal_value: None,
        });

        // name = expr;
        g.entry("Stmt".into()).or_default().push(Production {
            name: format!("StmtAssign_{}_{}", name, idx),
            nonterminal: "Stmt".into(),
            children_spec: vec![nt.clone()],
            rust_template: format!("{} = {{0}};", name),
            literal_value: None,
        });

        // name += expr;
        g.entry("Stmt".into()).or_default().push(Production {
            name: format!("StmtCompoundPlus_{}_{}", name, idx),
            nonterminal: "Stmt".into(),
            children_spec: vec![nt.clone()],
            rust_template: format!("{} += {{0}};", name),
            literal_value: None,
        });

        // name -= expr;
        g.entry("Stmt".into()).or_default().push(Production {
            name: format!("StmtCompoundMinus_{}_{}", name, idx),
            nonterminal: "Stmt".into(),
            children_spec: vec![nt.clone()],
            rust_template: format!("{} -= {{0}};", name),
            literal_value: None,
        });

        // for name in 0..expr { block }  — only for usize variables
        if ty == "usize" {
            g.entry("Stmt".into()).or_default().push(Production {
                name: format!("StmtFor_{}_{}", name, idx),
                nonterminal: "Stmt".into(),
                children_spec: vec!["Expr_usize".into(), "Block".into()],
                rust_template: format!("for {} in 0..{{0}} {{1}}", name),
                literal_value: None,
            });
        }
    }

    g
}

// ── Utility constructors ──────────────────────────────────────────────────────

fn prod(name: &str, nt: &str, children: Vec<&str>, template: &str) -> Production {
    Production {
        name: name.into(),
        nonterminal: nt.into(),
        children_spec: children.into_iter().map(|s| s.to_string()).collect(),
        rust_template: template.into(),
        literal_value: None,
    }
}

// ── Lookup helpers ────────────────────────────────────────────────────────────

pub fn find_production<'a>(kind: &str, grammar: &'a Grammar) -> Option<&'a Production> {
    grammar.values().flatten().find(|p| p.name == kind)
}

/// Maps production name → nonterminal. Used by the neighborhood search to
/// determine what hole-type to insert when punching a subtree out of a program.
pub type ReverseMap = std::collections::HashMap<String, String>;

pub fn build_reverse_map(grammar: &Grammar) -> ReverseMap {
    grammar.values().flatten()
        .map(|p| (p.name.clone(), p.nonterminal.clone()))
        .collect()
}

// ── Function definition registration ─────────────────────────────────────────

pub fn register_fn_def_known(target: &crate::loader::Target, grammar: &mut Grammar) -> String {
    let param_str = target
        .params
        .iter()
        .map(|p| format!("{}: {}", p.name, p.ty))
        .collect::<Vec<_>>()
        .join(", ");

    let ret_part = if target.return_type == "()" || target.return_type.is_empty() {
        String::new()
    } else {
        format!(" -> {}", target.return_type)
    };

    let kind = format!("FnDefKnown_{}", target.name);
    let template = format!("pub fn {}({}){}  {{0}}", target.name, param_str, ret_part);

    let p = Production {
        name: kind.clone(),
        nonterminal: "FnDefKnown".into(),
        children_spec: vec!["Block".into()],
        rust_template: template,
        literal_value: None,
    };
    grammar.entry("FnDefKnown".into()).or_default().push(p.clone());
    grammar.insert(kind.clone(), vec![p]);
    kind
}

// ── Program counting ──────────────────────────────────────────────────────────

pub fn count_programs(grammar: &Grammar, nt: &str, max_depth: usize) -> u128 {
    let mut memo = std::collections::HashMap::new();
    _count(grammar, nt, max_depth, &mut memo)
}

fn _count(
    grammar: &Grammar,
    nt: &str,
    remaining: usize,
    memo: &mut HashMap<(String, usize), u128>,
) -> u128 {
    let key = (nt.to_string(), remaining);
    if let Some(&v) = memo.get(&key) {
        return v;
    }
    let total = grammar
        .get(nt)
        .map(|prods| {
            prods
                .iter()
                .map(|prod| {
                    if prod.children_spec.is_empty() {
                        1u128
                    } else if remaining > 0 {
                        prod.children_spec.iter().fold(1u128, |acc, child_nt| {
                            acc.saturating_mul(_count(grammar, child_nt, remaining - 1, memo))
                        })
                    } else {
                        0
                    }
                })
                .sum()
        })
        .unwrap_or(0);
    memo.insert(key, total);
    total
}
