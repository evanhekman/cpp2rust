use crate::eval::Value;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Production {
    pub name: String,
    pub nonterminal: String,
    pub children_spec: Vec<String>,
    pub rust_template: String,
    pub literal_value: Option<Value>, // pre-parsed for eval hot path
}

pub type Grammar = HashMap<String, Vec<Production>>;

const BOOL_LITERALS: &[&str] = &["true", "false"];

pub fn build_grammar(
    literals: &[String],
    params: &[crate::loader::Param],
    return_type: &str,
) -> Grammar {
    let mut g: Grammar = HashMap::new();

    // Block
    g.entry("Block".into()).or_default().push(Production {
        name: "BlockSingle".into(),
        nonterminal: "Block".into(),
        children_spec: vec!["Stmt".into()],
        rust_template: "{ {0} }".into(),
        literal_value: None,
    });

    // Stmt
    let expr_nt = format!("Expr_{}", return_type);
    g.entry("Stmt".into()).or_default().extend([
        Production {
            name: "StmtReturn".into(),
            nonterminal: "Stmt".into(),
            children_spec: vec![expr_nt.clone()],
            rust_template: "return {0};".into(),
            literal_value: None,
        },
        Production {
            name: "StmtIf".into(),
            nonterminal: "Stmt".into(),
            children_spec: vec!["Expr_bool".into(), "Block".into()],
            rust_template: "if {0} {1}".into(),
            literal_value: None,
        },
        Production {
            name: "StmtIfElse".into(),
            nonterminal: "Stmt".into(),
            children_spec: vec!["Expr_bool".into(), "Block".into(), "Block".into()],
            rust_template: "if {0} {1} else {2}".into(),
            literal_value: None,
        },
    ]);

    // Expr_i32
    g.entry("Expr_i32".into()).or_default().extend([
        Production {
            name: "ExprAdd".into(),
            nonterminal: "Expr_i32".into(),
            children_spec: vec!["Expr_i32".into(), "Expr_i32".into()],
            rust_template: "({0} + {1})".into(),
            literal_value: None,
        },
        Production {
            name: "ExprSub".into(),
            nonterminal: "Expr_i32".into(),
            children_spec: vec!["Expr_i32".into(), "Expr_i32".into()],
            rust_template: "({0} - {1})".into(),
            literal_value: None,
        },
        Production {
            name: "ExprMul".into(),
            nonterminal: "Expr_i32".into(),
            children_spec: vec!["Expr_i32".into(), "Expr_i32".into()],
            rust_template: "({0} * {1})".into(),
            literal_value: None,
        },
        Production {
            name: "ExprDiv".into(),
            nonterminal: "Expr_i32".into(),
            children_spec: vec!["Expr_i32".into(), "Expr_i32".into()],
            rust_template: "({0} / {1})".into(),
            literal_value: None,
        },
        Production {
            name: "ExprMod".into(),
            nonterminal: "Expr_i32".into(),
            children_spec: vec!["Expr_i32".into(), "Expr_i32".into()],
            rust_template: "({0} % {1})".into(),
            literal_value: None,
        },
        Production {
            name: "ExprIfElse_i32".into(),
            nonterminal: "Expr_i32".into(),
            children_spec: vec!["Expr_bool".into(), "Expr_i32".into(), "Expr_i32".into()],
            rust_template: "if {0} { {1} } else { {2} }".into(),
            literal_value: None,
        },
    ]);

    // Expr_bool
    g.entry("Expr_bool".into()).or_default().extend([
        Production {
            name: "ExprEq".into(),
            nonterminal: "Expr_bool".into(),
            children_spec: vec!["Expr_i32".into(), "Expr_i32".into()],
            rust_template: "({0} == {1})".into(),
            literal_value: None,
        },
        Production {
            name: "ExprNe".into(),
            nonterminal: "Expr_bool".into(),
            children_spec: vec!["Expr_i32".into(), "Expr_i32".into()],
            rust_template: "({0} != {1})".into(),
            literal_value: None,
        },
        Production {
            name: "ExprLt".into(),
            nonterminal: "Expr_bool".into(),
            children_spec: vec!["Expr_i32".into(), "Expr_i32".into()],
            rust_template: "({0} < {1})".into(),
            literal_value: None,
        },
        Production {
            name: "ExprGt".into(),
            nonterminal: "Expr_bool".into(),
            children_spec: vec!["Expr_i32".into(), "Expr_i32".into()],
            rust_template: "({0} > {1})".into(),
            literal_value: None,
        },
        Production {
            name: "ExprLe".into(),
            nonterminal: "Expr_bool".into(),
            children_spec: vec!["Expr_i32".into(), "Expr_i32".into()],
            rust_template: "({0} <= {1})".into(),
            literal_value: None,
        },
        Production {
            name: "ExprGe".into(),
            nonterminal: "Expr_bool".into(),
            children_spec: vec!["Expr_i32".into(), "Expr_i32".into()],
            rust_template: "({0} >= {1})".into(),
            literal_value: None,
        },
        Production {
            name: "ExprAnd".into(),
            nonterminal: "Expr_bool".into(),
            children_spec: vec!["Expr_bool".into(), "Expr_bool".into()],
            rust_template: "({0} && {1})".into(),
            literal_value: None,
        },
        Production {
            name: "ExprOr".into(),
            nonterminal: "Expr_bool".into(),
            children_spec: vec!["Expr_bool".into(), "Expr_bool".into()],
            rust_template: "({0} || {1})".into(),
            literal_value: None,
        },
        Production {
            name: "ExprNot".into(),
            nonterminal: "Expr_bool".into(),
            children_spec: vec!["Expr_bool".into()],
            rust_template: "(!{0})".into(),
            literal_value: None,
        },
        Production {
            name: "ExprIfElse_bool".into(),
            nonterminal: "Expr_bool".into(),
            children_spec: vec!["Expr_bool".into(), "Expr_bool".into(), "Expr_bool".into()],
            rust_template: "if {0} { {1} } else { {2} }".into(),
            literal_value: None,
        },
    ]);

    // Literals → typed Expr leaves
    for (i, lit) in literals.iter().enumerate() {
        let is_bool = BOOL_LITERALS.contains(&lit.as_str());
        let nt = if is_bool { "Expr_bool" } else { "Expr_i32" };
        let lit_val = if is_bool {
            Some(Value::Bool(lit == "true"))
        } else {
            lit.parse::<i32>().ok().map(Value::Int)
        };
        g.entry(nt.into()).or_default().push(Production {
            name: format!("ExprLit_{}", i),
            nonterminal: nt.into(),
            children_spec: vec![],
            rust_template: lit.clone(),
            literal_value: lit_val,
        });
    }

    // Idents → typed Expr leaves
    for (i, param) in params.iter().enumerate() {
        let nt = format!("Expr_{}", param.ty);
        g.entry(nt.clone()).or_default().push(Production {
            name: format!("ExprIdent_{}", i),
            nonterminal: nt,
            children_spec: vec![],
            rust_template: param.name.clone(),
            literal_value: None, // resolved from env at eval time
        });
    }

    g
}

pub fn find_production<'a>(kind: &str, grammar: &'a Grammar) -> Option<&'a Production> {
    grammar.values().flatten().find(|p| p.name == kind)
}

pub fn register_fn_def_known(target: &crate::loader::Target, grammar: &mut Grammar) -> String {
    let param_str = target
        .params
        .iter()
        .map(|p| format!("{}: {}", p.name, p.ty))
        .collect::<Vec<_>>()
        .join(", ");
    let kind = format!("FnDefKnown_{}", target.name);
    let prod = Production {
        name: kind.clone(),
        nonterminal: "FnDefKnown".into(),
        children_spec: vec!["Block".into()],
        rust_template: format!(
            "pub fn {}({}) -> {} ",
            target.name, param_str, target.return_type
        ) + "{0}",
        literal_value: None,
    };
    grammar
        .entry("FnDefKnown".into())
        .or_default()
        .push(prod.clone());
    grammar.insert(kind.clone(), vec![prod]);
    kind
}

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
