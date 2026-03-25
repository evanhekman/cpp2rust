//! Neighborhood generation for program search.
//!
//! Given a complete program (no holes), `neighbors` returns a set of partial
//! programs — one per subtree — where each subtree has been replaced by a Hole
//! of the appropriate nonterminal.  These partial programs seed the worklist
//! with high-quality starting points; top-down expansion fills in the holes.

use crate::ast::{Child, Node};
use crate::grammar::ReverseMap;

/// Generates all neighbors of `root` up to `max_depth` levels of nesting.
/// A neighbor is `root` with exactly one non-root subtree replaced by a Hole.
pub fn neighbors(root: &Node, reverse_map: &ReverseMap, max_depth: usize) -> Vec<Node> {
    let mut result = Vec::new();
    collect(root, root, &mut Vec::new(), &mut result, reverse_map, max_depth);
    result
}

fn collect(
    root:        &Node,
    node:        &Node,
    path:        &mut Vec<usize>,
    out:         &mut Vec<Node>,
    reverse_map: &ReverseMap,
    max_depth:   usize,
) {
    for (i, child) in node.children.iter().enumerate() {
        if let Child::Node(child_node) = child {
            path.push(i);
            if path.len() <= max_depth {
                if let Some(nt) = reverse_map.get(&child_node.kind) {
                    out.push(root.punch_hole_at_path(path, nt.clone()));
                }
            }
            collect(root, child_node, path, out, reverse_map, max_depth);
            path.pop();
        }
    }
}
