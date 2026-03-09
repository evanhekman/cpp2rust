from __future__ import annotations
import json
import os
from typing import Dict, Any, List

from .grammar import build_grammar, Production


def load_symbols(symbols_path: str) -> List[str]:
    """Load literal symbols from a text file (one per line, ignoring comments/blanks)."""
    syms = []
    with open(symbols_path) as f:
        for line in f:
            line = line.strip()
            if line and not line.startswith("#"):
                syms.append(line)
    return syms


def load_target(dataset_dir: str, name: str) -> Dict[str, Any]:
    """Load a single synthesis target from dataset_dir/<name>.json."""
    path = os.path.join(dataset_dir, f"{name}.json")
    if not os.path.exists(path):
        available = [
            f[:-5] for f in os.listdir(dataset_dir) if f.endswith(".json")
        ]
        raise FileNotFoundError(
            f"Target '{name}' not found in {dataset_dir}/\n"
            f"Available: {', '.join(sorted(available))}"
        )
    with open(path) as f:
        return json.load(f)


def build_target_grammar(target: Dict[str, Any], literals: List[str]):
    """Build a typed grammar for a specific target."""
    return build_grammar(literals, target.get("params", []), target.get("return_type", "i32"))


def make_fn_def_known(target: Dict[str, Any]) -> str:
    return f"FnDefKnown_{target['name']}"


def register_fn_def_known(target: Dict[str, Any], grammar: Dict[str, List[Production]]):
    params = target.get("params", [])
    ret_type = target.get("return_type", "i32")
    fn_name = target["name"]
    param_str = ", ".join(f"{p['name']}: {p['type']}" for p in params)
    kind = make_fn_def_known(target)

    prod = Production(
        name=kind,
        nonterminal="FnDefKnown",
        children_spec=["Block"],
        rust_template=f"pub fn {fn_name}({param_str}) -> {ret_type} " + "{0}",
    )

    grammar.setdefault("FnDefKnown", []).append(prod)
    grammar.setdefault(kind, [prod])


def count_programs(grammar: Dict[str, List[Production]], nt: str, max_depth: int) -> int:
    """
    Count total complete programs rooted at nonterminal `nt` reachable
    within `max_depth` hole-path depth. Uses memoization.
    """
    memo: Dict[tuple, int] = {}

    def _count(nt: str, remaining: int) -> int:
        key = (nt, remaining)
        if key in memo:
            return memo[key]
        total = 0
        for prod in grammar.get(nt, []):
            if not prod.children_spec:
                total += 1
            elif remaining > 0:
                subtotal = 1
                for child_nt in prod.children_spec:
                    subtotal *= _count(child_nt, remaining - 1)
                total += subtotal
        memo[key] = total
        return total

    return _count(nt, max_depth)
