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
    if path.is_empty() || path[path.len() - 1] != 1 {
        return false;
    }
    let parent = partial.node_at_path(&path[..path.len() - 1]);
    if !COMMUTATIVE_OPS.contains(&parent.kind.as_str()) {
        return false;
    }
    match &parent.children[0] {
        Child::Node(left) => node_rank(left) > node_rank(replacement),
        _ => false,
    }
}
