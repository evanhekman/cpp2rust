use crate::ast::{Child, Node};
use std::collections::HashMap;

pub fn score(_node: &Node) -> i64 {
    // cost += h_operator_reuse(node);
    // cost += h_duplicate_arg(node);
    0i64
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
    if is_operator(node) {
        *counts.entry(&node.kind).or_default() += 1;
    }
    for child in &node.children {
        if let Child::Node(n) = child {
            collect_operators(n, counts);
        }
    }
}

fn is_operator(node: &Node) -> bool {
    node.kind.starts_with("Expr") && !node.children.is_empty()
}

fn duplicate_arg_penalty(node: &Node) -> i64 {
    let mut penalty = 0i64;
    if is_operator(node) && node.children.len() == 2 {
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
