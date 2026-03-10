use crate::ast::{Child, Node};
use crate::loader::CppFeatures;
use std::collections::HashMap;

pub fn score(node: &Node, features: Option<&CppFeatures>) -> i64 {
    let mut cost = 0i64;
    if let Some(f) = features {
        cost += h_cpp_similarity(node, f);
    }
    // cost += h_operator_reuse(node);
    // cost += h_duplicate_arg(node);
    cost
}

/// Reward operators that appear in the C++ source; penalize those that don't.
///
/// Matching is prefix-based: a feature string "ExprIfElse" matches node kinds
/// "ExprIfElse_i32" and "ExprIfElse_bool". Both operator-style nodes
/// (Expr*) and control-flow nodes (StmtIfElse) are considered.
pub fn h_cpp_similarity(node: &Node, features: &CppFeatures) -> i64 {
    let mut cost = 0i64;
    collect_similarity(node, features, &mut cost);
    cost
}

fn is_scored_node(node: &Node) -> bool {
    // Score operator expressions and control-flow statements (not leaves or wrappers)
    (node.kind.starts_with("Expr") || node.kind.starts_with("Stmt"))
        && !node.children.is_empty()
        && !node.kind.starts_with("StmtReturn")
        && !node.kind.starts_with("BlockSingle")
        && !node.kind.starts_with("FnDef")
}

fn matches_feature(kind: &str, features: &CppFeatures) -> bool {
    features.operators.iter().any(|op| kind.starts_with(op.as_str()))
}

fn collect_similarity(node: &Node, features: &CppFeatures, cost: &mut i64) {
    if is_scored_node(node) {
        if matches_feature(&node.kind, features) {
            *cost -= 2;
        } else {
            *cost += 3;
        }
    }
    for child in &node.children {
        if let Child::Node(n) = child {
            collect_similarity(n, features, cost);
        }
    }
}

pub fn h_operator_reuse(node: &Node) -> i64 {
    let mut counts: HashMap<&str, usize> = HashMap::new();
    collect_operators(node, &mut counts);
    counts
        .values()
        .map(|&c| if c == 1 { 1i64 } else { 2 * c as i64 })
        .sum()
}

pub fn h_duplicate_arg(node: &Node) -> i64 {
    duplicate_arg_penalty(node)
}

fn collect_operators<'a>(node: &'a Node, counts: &mut HashMap<&'a str, usize>) {
    if node.kind.starts_with("Expr") && !node.children.is_empty() {
        *counts.entry(&node.kind).or_default() += 1;
    }
    for child in &node.children {
        if let Child::Node(n) = child {
            collect_operators(n, counts);
        }
    }
}

fn duplicate_arg_penalty(node: &Node) -> i64 {
    let mut penalty = 0i64;
    if node.kind.starts_with("Expr") && node.children.len() == 2 {
        if let (Child::Node(a), Child::Node(b)) = (&node.children[0], &node.children[1]) {
            if a.is_complete() && b.is_complete() && structurally_equal(a, b) {
                penalty += 1;
            }
        }
    }
    for child in &node.children {
        if let Child::Node(n) = child {
            penalty += duplicate_arg_penalty(n);
        }
    }
    penalty
}

fn structurally_equal(a: &Node, b: &Node) -> bool {
    if a.kind != b.kind || a.children.len() != b.children.len() {
        return false;
    }
    a.children
        .iter()
        .zip(b.children.iter())
        .all(|(ca, cb)| match (ca, cb) {
            (Child::Node(na), Child::Node(nb)) => structurally_equal(na, nb),
            (Child::Hole(a), Child::Hole(b)) => a == b,
            _ => false,
        })
}
