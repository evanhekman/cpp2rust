from __future__ import annotations
from typing import Tuple
from .ast_nodes import ASTNode, Hole

# Commutative binary operators: (a op b) == (b op a)
COMMUTATIVE_OPS = {"ExprAdd", "ExprMul", "ExprEq", "ExprNe", "ExprAnd", "ExprOr"}


def node_rank(node: ASTNode) -> Tuple:
    """
    Comparable rank for canonical ordering of commutative operands.
    Rank order: literals < idents < compound expressions.
    Within each tier, sort lexicographically by kind.
    Holes are ranked highest so partial nodes are never incorrectly pruned.
    """
    if not node.children:
        if node.kind.startswith("ExprLit"):
            return (0, node.kind)
        if node.kind.startswith("ExprIdent"):
            return (1, node.kind)
        return (2, node.kind)
    return (3, node.kind)


def is_right_child_of_commutative(partial: ASTNode, path: list) -> bool:
    """True if the hole at `path` is the right child (index 1) of a commutative op."""
    if not path or path[-1] != 1:
        return False
    parent = _node_at(partial, path[:-1])
    return isinstance(parent, ASTNode) and parent.kind in COMMUTATIVE_OPS


def should_prune(partial: ASTNode, path: list, replacement: ASTNode) -> bool:
    """
    For the right child of a commutative op: prune if rank(left) > rank(replacement).
    This keeps only the canonical form where left ≤ right.
    Guaranteed complete: equal-rank pairs are always kept.
    """
    if not is_right_child_of_commutative(partial, path):
        return False
    parent = _node_at(partial, path[:-1])
    left = parent.children[0]
    if not isinstance(left, ASTNode):
        return False
    return node_rank(left) > node_rank(replacement)


def _node_at(root: ASTNode, path: list) -> ASTNode:
    node = root
    for idx in path:
        node = node.children[idx]
    return node
