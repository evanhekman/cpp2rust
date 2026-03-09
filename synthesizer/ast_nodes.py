from __future__ import annotations
from dataclasses import dataclass
from typing import List, Any, Optional

@dataclass
class Hole:
    """A typed hole — a blank slot expecting a specific nonterminal."""
    nt: str

    def __repr__(self):
        return f"HOLE({self.nt})"

Path = List[int]  # index into children at each level

@dataclass
class ASTNode:
    kind: str
    children: List[Any]  # ASTNode | Hole
    depth: int = 0

    def is_complete(self) -> bool:
        for c in self.children:
            if isinstance(c, Hole):
                return False
            if isinstance(c, ASTNode) and not c.is_complete():
                return False
        return True

    def first_hole_path(self) -> Optional[Path]:
        """Left-to-right DFS; returns path to first Hole or None."""
        for i, c in enumerate(self.children):
            if isinstance(c, Hole):
                return [i]
            if isinstance(c, ASTNode):
                sub = c.first_hole_path()
                if sub is not None:
                    return [i] + sub
        return None

    def hole_at_path(self, path: Path) -> Hole:
        """Return the Hole at the given path."""
        node = self
        for idx in path[:-1]:
            node = node.children[idx]
        return node.children[path[-1]]

    def replace_at_path(self, path: Path, replacement: Any) -> 'ASTNode':
        """Return a new ASTNode with the node at path replaced."""
        if not path:
            return replacement
        i = path[0]
        new_children = list(self.children)
        if path[1:]:
            new_children[i] = new_children[i].replace_at_path(path[1:], replacement)
        else:
            new_children[i] = replacement
        return ASTNode(self.kind, new_children, self.depth)
