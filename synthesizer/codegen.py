from __future__ import annotations
from .ast_nodes import ASTNode, _Hole
from .grammar import Production
from typing import Dict, List

def render(node: ASTNode, grammar: Dict[str, List[Production]]) -> str:
    """Render an AST node to a Rust source string."""
    if isinstance(node, _Hole):
        return "???"
    if isinstance(node, str):
        return node

    prod = _find_production(node.kind, grammar)
    if prod is None:
        raise ValueError(f"No production for kind={node.kind!r}")

    if not prod.children_spec:
        # Leaf production — template is the literal value
        return prod.rust_template

    rendered_children = [render(c, grammar) for c in node.children]
    return prod.rust_template.format(*rendered_children)

def wrap_in_harness(fn_src: str, fn_name: str, inputs: List[str]) -> str:
    """Wrap function source in a main() that calls it with given inputs and prints result."""
    args = ", ".join(inputs)
    return f"""{fn_src}

fn main() {{
    let result = {fn_name}({args});
    println!("{{:?}}", result);
}}
"""

def _find_production(kind: str, grammar):
    for prods in grammar.values():
        for p in prods:
            if p.name == kind:
                return p
    return None
