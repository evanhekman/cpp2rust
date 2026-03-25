use crate::ast::{Child, Node};
use crate::loader::CppFeatures;

/// Which heuristics are active. All enabled by default.
#[derive(Clone, Debug)]
pub struct HeuristicConfig {
    pub ordering:    bool,
    pub absent:      bool,
    pub required:    bool,
    pub structural:  bool,
    pub block_sizes: bool,
    pub vars:        bool,
    pub complete:    bool,
    pub holes:       bool,
}

impl Default for HeuristicConfig {
    fn default() -> Self {
        Self { ordering: true, absent: true, required: true, structural: true, block_sizes: true, vars: true, complete: true, holes: true }
    }
}

impl HeuristicConfig {
    pub fn from_disabled(disabled: &[String]) -> Self {
        let mut cfg = Self::default();
        for name in disabled {
            match name.as_str() {
                "ordering"    => cfg.ordering    = false,
                "absent"      => cfg.absent      = false,
                "required"    => cfg.required    = false,
                "structural"  => cfg.structural  = false,
                "block-sizes" => cfg.block_sizes = false,
                "vars"        => cfg.vars        = false,
                "complete"    => cfg.complete    = false,
                "holes"       => cfg.holes       = false,
                other => eprintln!("warning: unknown heuristic '{}' (valid: ordering, absent, required, structural, block-sizes, vars, complete, holes)", other),
            }
        }
        cfg
    }
}

pub fn score(node: &Node, features: Option<&CppFeatures>, ast_hints: Option<&[String]>, block_sizes: Option<&[usize]>, required_idents: Option<&[String]>, cfg: &HeuristicConfig) -> i64 {
    let mut cost = 0i64;
    if let Some(hints) = ast_hints {
        // AST-derived heuristic: prioritise the verbatim translation,
        // explore deviations proportional to their structural distance.
        if cfg.ordering   { cost += h_ast_ordering(node, hints); }
        if cfg.absent     { cost += h_ast_absent_penalty(node, hints); }
        if cfg.required   { cost += h_required_hint_penalty(node, hints); }
        if cfg.structural { cost += h_structural_checks(node, hints); }
        if cfg.block_sizes {
            if let Some(sizes) = block_sizes {
                cost += h_block_sizes(node, sizes);
            }
        }
    } else if let Some(f) = features {
        // Fallback: text-scanned operator heuristics (dataset0 compat)
        if cfg.ordering  { cost += h_ordering_match(node, f); }
        if cfg.absent    { cost += h_absent_penalty(node, f); }
        if cfg.absent    { cost += h_overcount_penalty(node, f); }
    }
    // Universal heuristics: apply regardless of dataset format
    if cfg.vars {
        if let Some(idents) = required_idents {
            cost += h_required_vars(node, idents);
        }
    }
    if cfg.complete { cost += h_complete_bonus(node); }
    if cfg.holes    { cost += h_hole_count_penalty(node); }
    cost
}

/// (-1) bonus for complete programs (no holes). Breaks score ties in favour of
/// evaluating a finished candidate over expanding another partial program.
/// Evaluation is cheap; this ensures we never defer a testable program behind
/// a partial with the same content-based score.
pub fn h_complete_bonus(node: &Node) -> i64 {
    if node.is_complete() { -1 } else { 0 }
}

/// (+1 per remaining hole) gradient toward completion. Partial programs with
/// fewer holes score better than deeply-partial ones with equal content score,
/// steering the search toward finishing programs rather than exploring new ones.
pub fn h_hole_count_penalty(node: &Node) -> i64 {
    count_holes(node) as i64
}

fn count_holes(node: &Node) -> usize {
    node.children.iter().map(|c| match c {
        Child::Hole(_) => 1,
        Child::Node(n) => count_holes(n),
    }).sum()
}

// ── AST-based heuristics ──────────────────────────────────────────────────────

/// Reward (-1 per matched feature) for the longest common prefix between the
/// expected statement sequence (derived from the C++ AST) and the candidate's
/// DFS pre-order statement sequence.  Only Stmt-level nodes are compared here;
/// expression nodes are handled by h_ast_absent_penalty only.
///
/// Limiting to statements avoids a false-scoring problem where an Expr node
/// (e.g. ExprIndex) that appears in hints later in the sequence artificially
/// improves a program that uses it at the wrong structural position (e.g.
/// inside a let-init rather than inside a compound-plus body).
pub fn h_ast_ordering(node: &Node, hints: &[String]) -> i64 {
    let mut rust_seq: Vec<String> = Vec::new();
    collect_stmt_sequence(node, &mut rust_seq);

    // Filter hints to only statement-level entries (same kind of nodes we collect)
    let stmt_hints: Vec<&str> = hints.iter()
        .filter(|h| h.starts_with("Stmt"))
        .map(|h| h.as_str())
        .collect();

    let lcp = stmt_hints
        .iter()
        .zip(rust_seq.iter())
        .take_while(|(hint, rust_op)| ast_feature_matches(rust_op, hint))
        .count();

    let excess = rust_seq.len().saturating_sub(stmt_hints.len()) as i64;
    let deficiency = stmt_hints.len().saturating_sub(rust_seq.len()) as i64;

    -(lcp as i64) + excess + deficiency
}

/// Penalty (+3) for each scored Rust node whose kind has no corresponding
/// entry in the expected hint sequence.  Keeps the verbatim candidate at score
/// 0 while pushing structurally wrong candidates to the back of the queue.
pub fn h_ast_absent_penalty(node: &Node, hints: &[String]) -> i64 {
    let mut cost = 0i64;
    ast_absent_rec(node, hints, &mut cost);
    cost
}

fn ast_absent_rec(node: &Node, hints: &[String], cost: &mut i64) {
    if is_ast_scored_node(node) {
        let present = hints.iter().any(|h| ast_feature_matches(&node.kind, h));
        if !present {
            *cost += 3;
        }
    }
    for child in &node.children {
        if let Child::Node(n) = child {
            ast_absent_rec(n, hints, cost);
        }
    }
}

/// Penalty (+3) when a Block node's statement count doesn't match the expected
/// block size at that position in DFS pre-order.  Eliminates wrong BlockSeq
/// variants early without touching any other heuristic.
pub fn h_block_sizes(node: &Node, block_sizes: &[usize]) -> i64 {
    let mut idx = 0usize;
    let mut cost = 0i64;
    check_block_sizes(node, block_sizes, &mut idx, &mut cost);
    cost
}

fn check_block_sizes(node: &Node, expected: &[usize], idx: &mut usize, cost: &mut i64) {
    let is_block = node.kind.starts_with("BlockSingle") || node.kind.starts_with("BlockSeq");
    if is_block {
        if *idx < expected.len() {
            if node.children.len() != expected[*idx] {
                *cost += 3;
            }
            *idx += 1;
        }
    }
    for child in &node.children {
        if let Child::Node(n) = child {
            check_block_sizes(n, expected, idx, cost);
        }
    }
}

/// Structural position checks for for-loop expression choices.
/// Applies to both partial and complete programs: as soon as a StmtFor's
/// range or body is filled in with a non-Hole node, we can check position.
///
///  - If "ExprLen" is in hints: any StmtFor with a filled range that does NOT
///    contain ExprLen gets +3.  This immediately penalises `for i in 0..0 { … }`
///    vs `for i in 0..a.len() { … }` when the range hole is expanded.
///
///  - If "ExprIndex" is in hints: any StmtFor with a filled body that does NOT
///    contain ExprIndex gets +3.  This immediately penalises `s += 0` vs
///    `s += a[i]` when the body expression hole is expanded.
pub fn h_structural_checks(node: &Node, hints: &[String]) -> i64 {
    let needs_exprlen   = hints.iter().any(|h| h == "ExprLen");
    let needs_exprindex = hints.iter().any(|h| h == "ExprIndex");
    if !needs_exprlen && !needs_exprindex {
        return 0;
    }
    let mut cost = 0i64;
    structural_rec(node, needs_exprlen, needs_exprindex, &mut cost);
    cost
}

fn structural_rec(node: &Node, need_len: bool, need_idx: bool, cost: &mut i64) {
    if node.kind.starts_with("StmtFor") {
        // Range = first child; body block = second child
        if need_len {
            match node.children.first() {
                Some(Child::Node(range)) => {
                    if !subtree_has_prefix(range, "ExprLen") {
                        *cost += 3;
                    }
                }
                _ => {} // Hole → not yet decided, skip
            }
        }
        if need_idx {
            match node.children.get(1) {
                Some(Child::Node(block)) => {
                    if !subtree_has_prefix(block, "ExprIndex") {
                        *cost += 3;
                    }
                }
                _ => {} // Hole → not yet decided, skip
            }
        }
    }
    for child in &node.children {
        if let Child::Node(n) = child {
            structural_rec(n, need_len, need_idx, cost);
        }
    }
}

fn subtree_has_prefix(node: &Node, prefix: &str) -> bool {
    if node.kind.starts_with(prefix) {
        return true;
    }
    for child in &node.children {
        if let Child::Node(n) = child {
            if subtree_has_prefix(n, prefix) {
                return true;
            }
        }
    }
    false
}

/// Penalty (+3 per Expr hint) for complete programs that are entirely missing
/// a required Expr-type operator.  Applied only to complete programs (no holes);
/// partial programs return 0 because missing operators may still be filled in.
pub fn h_required_hint_penalty(node: &Node, hints: &[String]) -> i64 {
    if !node.is_complete() {
        return 0;
    }
    // Collect ALL node kind names in the program (including leaves)
    let mut all_kinds: Vec<String> = Vec::new();
    collect_all_kinds(node, &mut all_kinds);

    hints.iter()
        .filter(|h| h.starts_with("Expr"))
        .map(|hint| {
            let found = all_kinds.iter().any(|k| ast_feature_matches(k, hint));
            if found { 0i64 } else { 3 }
        })
        .sum()
}

/// Penalty (+3 per missing ident) for complete programs that don't use a
/// parameter or local variable that appears in the C++ AST.
///
/// `required_idents` is a list of ExprIdent production name prefixes derived
/// from the C++ AST (e.g. ["ExprIdent_0", "ExprIdent_1"]).  Any complete
/// program that doesn't contain a node whose kind starts with one of these
/// prefixes gets penalised.  This prevents the search from wasting time on
/// programs that ignore an expected parameter entirely.
pub fn h_required_vars(node: &Node, required_idents: &[String]) -> i64 {
    if !node.is_complete() || required_idents.is_empty() {
        return 0;
    }
    let mut all_kinds: Vec<String> = Vec::new();
    collect_all_kinds(node, &mut all_kinds);
    required_idents.iter()
        .map(|ident| {
            let found = all_kinds.iter().any(|k| k.starts_with(ident.as_str()));
            if found { 0i64 } else { 3 }
        })
        .sum()
}

fn collect_all_kinds(node: &Node, names: &mut Vec<String>) {
    names.push(node.kind.clone());
    for child in &node.children {
        if let Child::Node(n) = child {
            collect_all_kinds(n, names);
        }
    }
}

/// True if a Rust AST node kind satisfies an expected hint prefix.
///
/// Rules:
///  - "StmtFor" matches StmtFor_* AND StmtWhile (both are iteration)
///  - "ExprAdd"/"ExprSub"/… match the typed variants (ExprAdd_usize, ExprAdd_u32, …)
///  - "ExprIndex" matches ExprIndex_* (any slice param)
///  - "ExprLen"   matches ExprLen_*
///  - "StmtLetMut" / "StmtAssign" / "StmtCompoundPlus" / etc. match their suffixed variants
///  - All other hints use simple prefix matching
fn ast_feature_matches(rust_kind: &str, hint: &str) -> bool {
    match hint {
        // Generic "StmtFor" (no variable) matches both StmtFor_* and StmtWhile
        "StmtFor" => rust_kind.starts_with("StmtFor") || rust_kind == "StmtWhile",
        // Variable-specific "StmtFor_i" only matches StmtFor_i* (not StmtWhile)
        _ => rust_kind.starts_with(hint),
    }
}


/// Collect only Stmt-level nodes in DFS pre-order.
/// Used by h_ast_ordering so that Expr nodes in wrong structural positions
/// (e.g. ExprIndex inside a let-init) don't distort the ordering score.
fn collect_stmt_sequence(node: &Node, seq: &mut Vec<String>) {
    if is_stmt_scored_node(node) {
        seq.push(node.kind.clone());
    }
    for child in &node.children {
        if let Child::Node(n) = child {
            collect_stmt_sequence(n, seq);
        }
    }
}

fn is_stmt_scored_node(node: &Node) -> bool {
    !node.children.is_empty() && node.kind.starts_with("Stmt")
}

fn is_ast_scored_node(node: &Node) -> bool {
    !node.children.is_empty()
        && !node.kind.starts_with("BlockSingle")
        && !node.kind.starts_with("BlockSeq")
        && !node.kind.starts_with("FnDef")
        && !node.kind.starts_with("ExprCast")  // casts are implicit in C++; don't penalise
        && (node.kind.starts_with("Expr") || node.kind.starts_with("Stmt"))
}

// h_count_match: NOT WIRED IN — reward-based heuristics can delay programs BFS would find
// quickly by boosting wrong candidates that happen to match counts by coincidence.
// The penalty side of this idea is already covered by h_absent_penalty + h_overcount_penalty.
//
// pub fn h_count_match(node: &Node, features: &CppFeatures) -> i64 {
//     let rust_counts = collect_operator_counts(node);
//     features
//         .operator_counts
//         .iter()
//         .map(|(feature, &cpp_count)| {
//             let rust_count: usize = rust_counts
//                 .iter()
//                 .filter(|(k, _)| feature_matches(k, feature))
//                 .map(|(_, &v)| v)
//                 .sum();
//             if rust_count == cpp_count { -1 } else { 0 }
//         })
//         .sum()
// }

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
        rust_kind.starts_with("ExprIfElse") || rust_kind == "StmtIfElse" || rust_kind == "StmtIf"
    } else {
        rust_kind.starts_with(feature)
    }
}

fn is_scored_node(node: &Node) -> bool {
    !node.children.is_empty()
        && !node.kind.starts_with("StmtReturn")
        && !node.kind.starts_with("BlockSingle")
        && !node.kind.starts_with("BlockSeq")
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
