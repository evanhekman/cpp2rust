from __future__ import annotations
from dataclasses import dataclass
from typing import List, Any, Optional

class _Hole:
    def __repr__(self): return "HOLE"
HOLE = _Hole()

Path = List[int]  # index into children at each level

@dataclass
class ASTNode:
    kind: str
    children: List[Any]  # ASTNode | str | HOLE
    depth: int = 0

    def is_complete(self) -> bool:
        for c in self.children:
            if isinstance(c, _Hole):
                return False
            if isinstance(c, ASTNode) and not c.is_complete():
                return False
        return True

    def first_hole_path(self) -> Optional[Path]:
        """Left-to-right DFS; returns path to first HOLE or None."""
        for i, c in enumerate(self.children):
            if isinstance(c, _Hole):
                return [i]
            if isinstance(c, ASTNode):
                sub = c.first_hole_path()
                if sub is not None:
                    return [i] + sub
        return None

    def node_at_path(self, path: Path) -> Any:
        node = self
        for idx in path:
            node = node.children[idx]
        return node

    def node_depth_at_path(self, path: Path) -> int:
        node = self
        for idx in path:
            node = node.children[idx]
            if isinstance(node, ASTNode):
                pass
        return len(path)  # depth relative to root

    def replace_at_path(self, path: Path, replacement: Any) -> 'ASTNode':
        """Return a new ASTNode with the node at path replaced."""
        if not path:
            return replacement
        i = path[0]
        new_children = list(self.children)
        old = new_children[i]
        if path[1:]:
            new_children[i] = old.replace_at_path(path[1:], replacement)
        else:
            new_children[i] = replacement
        return ASTNode(self.kind, new_children, self.depth)
