from __future__ import annotations
from .ast_nodes import ASTNode, Hole
from .grammar import Production
from typing import Dict, List


def render(node: ASTNode, grammar: Dict[str, List[Production]]) -> str:
    """Render an AST node to a Rust source string."""
    if isinstance(node, Hole):
        return f"???:{node.nt}"
    if isinstance(node, str):
        return node

    prod = _find_production(node.kind, grammar)
    if prod is None:
        raise ValueError(f"No production for kind={node.kind!r}")

    if not prod.children_spec:
        return prod.rust_template

    rendered_children = [render(c, grammar) for c in node.children]
    return prod.rust_template.format(*rendered_children)


def wrap_in_harness(fn_src: str, fn_name: str, test_cases: List[Dict]) -> str:
    """
    Wrap function source in a main() that runs all test cases in one binary.
    Each call prints its result on a separate line.
    """
    calls = "\n".join(
        f'    println!("{{:?}}", {fn_name}({", ".join(tc["inputs"])}));'
        for tc in test_cases
    )
    return f"""{fn_src}

fn main() {{
{calls}
}}
"""


def _find_production(kind: str, grammar):
    for prods in grammar.values():
        for p in prods:
            if p.name == kind:
                return p
    return None
