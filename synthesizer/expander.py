from __future__ import annotations
from .ast_nodes import ASTNode, Hole, Path


def nonterminal_at_path(root: ASTNode, path: Path) -> str:
    """Return the expected nonterminal for the Hole at the given path."""
    hole = root.hole_at_path(path)
    if not isinstance(hole, Hole):
        raise ValueError(f"Expected Hole at path {path}, got {hole!r}")
    return hole.nt
