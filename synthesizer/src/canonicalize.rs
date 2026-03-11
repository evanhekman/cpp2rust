use crate::ast::{Child, Node};

const COMMUTATIVE_OPS: &[&str] = &[
    "ExprAdd", "ExprMul", "ExprEq", "ExprNe", "ExprAnd", "ExprOr",
];

pub fn node_rank(node: &Node) -> (u8, &str) {
    if node.children.is_empty() {
        if node.kind.starts_with("ExprLit") {
            return (0, &node.kind);
        }
        if node.kind.starts_with("ExprIdent") {
            return (1, &node.kind);
        }
        return (2, &node.kind);
    }
    (3, &node.kind)
}

pub fn should_prune(partial: &Node, path: &[usize], replacement: &Node) -> bool {
    if path.is_empty() {
        return false;
    }
    let last = path[path.len() - 1];
    let parent = partial.node_at_path(&path[..path.len() - 1]);

    // Dead code: StmtReturn as first stmt of BlockSeq makes the second unreachable.
    if last == 0 && parent.kind == "BlockSeq" && replacement.kind == "StmtReturn" {
        return true;
    }

    // Canonicalize commutative ops: enforce left ≤ right by node rank.
    if last == 1 {
        if COMMUTATIVE_OPS.contains(&parent.kind.as_str()) {
            if let Child::Node(left) = &parent.children[0] {
                return node_rank(left) > node_rank(replacement);
            }
        }
    }

    false
}
