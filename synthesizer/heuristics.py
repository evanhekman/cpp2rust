from __future__ import annotations

from collections import Counter
from typing import List

from .ast_nodes import ASTNode, Hole


def score(node: ASTNode) -> int:
    """
    Heuristic score for a partial AST. Intended for a min-priority queue
    (lower score = expanded sooner). Not yet wired into the main loop.

    Comment/uncomment heuristic lines below to test different combinations.
    """
    cost = 0
    # cost += h_operator_reuse(node)  # +1 unique op, +2 duplicate op
    # cost += h_duplicate_arg(node)  # +1 for binary op with identical children
    return cost


# ---------------------------------------------------------------------------
# Heuristics — each returns an int cost contribution
# ---------------------------------------------------------------------------


def h_operator_reuse(node: ASTNode) -> int:
    """
    +1 per unique operator kind in the tree.
    +2 per occurrence of any operator kind used more than once.
    """
    op_counts = Counter(_collect_operators(node))
    cost = 0
    for op, count in op_counts.items():
        if count == 1:
            cost += 1
        else:
            cost += 2 * count
    return cost


def h_duplicate_arg(node: ASTNode) -> int:
    """
    +1 for each binary operator whose two Expr children are structurally
    identical (e.g. ExprAdd(x, x)). Only applied to complete subtrees.
    """
    return _duplicate_arg_penalty(node)


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _collect_operators(node: ASTNode) -> List[str]:
    """Return a flat list of all operator kinds in the tree (DFS)."""
    result = []
    if _is_operator(node):
        result.append(node.kind)
    for child in node.children:
        if isinstance(child, ASTNode):
            result.extend(_collect_operators(child))
    return result


def _is_operator(node: ASTNode) -> bool:
    """Operators are non-leaf Expr nodes."""
    return node.kind.startswith("Expr") and bool(node.children)


def _duplicate_arg_penalty(node: ASTNode) -> int:
    """
    +1 for each binary operator whose two Expr children are structurally
    identical (e.g. ExprAdd(x, x)). Only counted when both children are
    complete (no Holes), to avoid false positives on partial trees.
    """
    penalty = 0
    if (
        _is_operator(node)
        and len(node.children) == 2
        and all(isinstance(c, ASTNode) and c.is_complete() for c in node.children)
        and _structurally_equal(node.children[0], node.children[1])
    ):
        penalty += 1
    for child in node.children:
        if isinstance(child, ASTNode):
            penalty += _duplicate_arg_penalty(child)
    return penalty


def _structurally_equal(a: ASTNode, b: ASTNode) -> bool:
    if a.kind != b.kind or len(a.children) != len(b.children):
        return False
    for ca, cb in zip(a.children, b.children):
        if not isinstance(ca, type(cb)):
            return False
        if isinstance(ca, ASTNode) and not _structurally_equal(ca, cb):
            return False
        if isinstance(ca, Hole) and ca.nt != cb.nt:
            return False
    return True
