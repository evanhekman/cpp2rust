from __future__ import annotations
from typing import Dict, List, Optional
from .ast_nodes import ASTNode, Path, _Hole
from .grammar import Production

def nonterminal_at_path(root: ASTNode, path: Path, grammar: Dict[str, List[Production]]) -> str:
    """
    Walk the path from root, consulting grammar children_spec to determine
    what nonterminal the HOLE at `path` expects.
    """
    node = root
    remaining = list(path)

    while remaining:
        idx = remaining[0]
        remaining = remaining[1:]

        # Find the production for this node
        prod = _find_production(node.kind, grammar)
        if prod is None:
            raise ValueError(f"No production found for kind={node.kind!r}")

        if not remaining:
            # This index points to the HOLE — return expected NT
            if idx < len(prod.children_spec):
                return prod.children_spec[idx]
            raise IndexError(f"idx={idx} out of range for {prod.name} children_spec")

        # Descend
        child = node.children[idx]
        if isinstance(child, _Hole):
            raise ValueError("Encountered HOLE before reaching end of path")
        node = child

    # path was empty — the root itself is the HOLE? shouldn't happen
    raise ValueError("Empty path in nonterminal_at_path")

def _find_production(kind: str, grammar: Dict[str, List[Production]]) -> Optional[Production]:
    for prods in grammar.values():
        for p in prods:
            if p.name == kind:
                return p
    return None
