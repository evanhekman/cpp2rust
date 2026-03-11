use crate::ast::{Child, Node};
use crate::loader::CppFeatures;

pub fn score(node: &Node, features: Option<&CppFeatures>) -> i64 {
    let mut cost = 0i64;
    if let Some(f) = features {
        // cost += h_count_match(node, f);
        // cost += h_ordering_match(node, f);
        cost += h_absent_penalty(node, f);
        cost += h_overcount_penalty(node, f);
    }
    // cost += h_operator_reuse(node);
    // cost += h_duplicate_arg(node);
    cost
}

/// (-1) for each C++ operator whose count in the Rust candidate exactly matches
/// its count in the C++ source. Encourages programs with the right operator
/// distribution without penalising programs that differ.
pub fn h_count_match(node: &Node, features: &CppFeatures) -> i64 {
    let rust_counts = collect_operator_counts(node);
    features
        .operator_counts
        .iter()
        .map(|(feature, &cpp_count)| {
            let rust_count: usize = rust_counts
                .iter()
                .filter(|(k, _)| feature_matches(k, feature))
                .map(|(_, &v)| v)
                .sum();
            if rust_count == cpp_count {
                -1
            } else {
                0
            }
        })
        .sum()
}

/// (-1) for each operator in the longest common prefix between the C++ operator
/// sequence (left-to-right scan) and the Rust AST operator sequence (DFS
/// pre-order). Encourages programs whose structural shape matches the C++.
///
/// For structured programs (if/then/else, arithmetic expressions) these two
/// orderings are naturally aligned: the outer construct appears first in both.
pub fn h_ordering_match(node: &Node, features: &CppFeatures) -> i64 {
    let mut rust_seq: Vec<String> = Vec::new();
    collect_operator_sequence(node, &mut rust_seq);

    let lcp = features
        .operator_sequence
        .iter()
        .zip(rust_seq.iter())
        .take_while(|(cpp_op, rust_op)| feature_matches(rust_op, cpp_op))
        .count();

    -(lcp as i64)
}

/// (+3) for each operator node in the Rust candidate that has no corresponding
/// entry in the C++ features. Keeps correct-operator programs at score 0
/// (preserving BFS order for them) while pushing wrong-operator programs to
/// the back of the queue.
///
/// This is strictly safer than reward-based heuristics: it cannot delay a
/// program that BFS would have found quickly, since those programs use
/// operators present in the C++ and remain at score 0.
pub fn h_absent_penalty(node: &Node, features: &CppFeatures) -> i64 {
    let mut cost = 0i64;
    absent_penalty_rec(node, features, &mut cost);
    cost
}

fn absent_penalty_rec(node: &Node, features: &CppFeatures, cost: &mut i64) {
    if is_scored_node(node) {
        let present = features
            .operator_counts
            .keys()
            .any(|f| feature_matches(&node.kind, f));
        if !present {
            *cost += 3;
        }
    }
    for child in &node.children {
        if let Child::Node(n) = child {
            absent_penalty_rec(n, features, cost);
        }
    }
}

/// (+3) per excess use of an operator that appears in the C++ but more times
/// in the Rust candidate than in the C++ source. Complements h_absent_penalty:
/// that heuristic penalises operators entirely absent from C++; this one
/// penalises operators that are present but overused.
pub fn h_overcount_penalty(node: &Node, features: &CppFeatures) -> i64 {
    let rust_counts = collect_operator_counts(node);
    features
        .operator_counts
        .iter()
        .map(|(feature, &cpp_count)| {
            let rust_count: usize = rust_counts
                .iter()
                .filter(|(k, _)| feature_matches(k, feature))
                .map(|(_, &v)| v)
                .sum();
            if rust_count > cpp_count {
                ((rust_count - cpp_count) * 3) as i64
            } else {
                0
            }
        })
        .sum()
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Returns true if a Rust AST node kind matches a C++ feature name.
/// "IfElse" is a special unified key that matches both ExprIfElse_* and StmtIfElse.
/// All other features use prefix matching (e.g. "ExprGt" matches "ExprGt").
fn feature_matches(rust_kind: &str, feature: &str) -> bool {
    if feature == "IfElse" {
        rust_kind.starts_with("ExprIfElse") || rust_kind == "StmtIfElse"
    } else {
        rust_kind.starts_with(feature)
    }
}

fn is_scored_node(node: &Node) -> bool {
    !node.children.is_empty()
        && !node.kind.starts_with("StmtReturn")
        && !node.kind.starts_with("BlockSingle")
        && !node.kind.starts_with("FnDef")
        && (node.kind.starts_with("Expr") || node.kind.starts_with("Stmt"))
}

fn collect_operator_counts(node: &Node) -> std::collections::HashMap<String, usize> {
    let mut counts = std::collections::HashMap::new();
    collect_counts_rec(node, &mut counts);
    counts
}

fn collect_counts_rec(node: &Node, counts: &mut std::collections::HashMap<String, usize>) {
    if is_scored_node(node) {
        *counts.entry(node.kind.clone()).or_default() += 1;
    }
    for child in &node.children {
        if let Child::Node(n) = child {
            collect_counts_rec(n, counts);
        }
    }
}

fn collect_operator_sequence(node: &Node, seq: &mut Vec<String>) {
    if is_scored_node(node) {
        seq.push(node.kind.clone());
    }
    for child in &node.children {
        if let Child::Node(n) = child {
            collect_operator_sequence(n, seq);
        }
    }
}

// ── Commented-out heuristics (kept for reference) ────────────────────────────

// pub fn h_operator_reuse(node: &Node) -> i64 {
//     let mut counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
//     _collect_ops(node, &mut counts);
//     counts.values().map(|&c| if c == 1 { 1i64 } else { 2 * c as i64 }).sum()
// }
//
// pub fn h_duplicate_arg(node: &Node) -> i64 { ... }
