#!/usr/bin/env python3
"""
Rust Program Synthesizer — enumerative worklist-based synthesis.
"""

import argparse
import signal
import sys
import time

from synthesizer.ast_nodes import ASTNode, Hole
from synthesizer.codegen import render
from synthesizer.evaluator import test_candidate
from synthesizer.expander import nonterminal_at_path
from synthesizer.heuristics import score
from synthesizer.loader import (
    build_target_grammar,
    count_programs,
    load_symbols,
    load_target,
    make_fn_def_known,
    register_fn_def_known,
)
from synthesizer.worklist import Worklist

_interrupted = False


def _fmt_count(n: int) -> str:
    if n < 1_000_000:
        return str(n)
    exp = len(str(n)) - 1
    mantissa = n / (10 ** exp)
    return f"{mantissa:.2f}e{exp}"


def _handle_sigint(sig, frame):
    global _interrupted
    _interrupted = True
    print("\n  Interrupted — stopping search.")


def synthesize_target(target, literals, max_depth, timeout):
    global _interrupted
    fn_name = target["name"]
    test_cases = target["test_cases"]

    grammar = build_target_grammar(target, literals)
    register_fn_def_known(target, grammar)

    kind = make_fn_def_known(target)
    candidates_possible = count_programs(grammar, kind, max_depth)

    worklist = Worklist()
    root = ASTNode(kind=kind, children=[Hole("Block")], depth=0)
    worklist.push(root, score(root))

    deadline = time.time() + timeout
    candidates_tried = 0
    nodes_expanded = 0

    while worklist and not _interrupted:
        if time.time() > deadline:
            print(
                f"  TIMEOUT after {nodes_expanded} expansions, "
                f"{candidates_tried}/{_fmt_count(candidates_possible)} candidates tested"
            )
            return None

        partial = worklist.pop()

        if partial.is_complete():
            candidates_tried += 1
            try:
                src = render(partial, grammar)
            except Exception:
                continue
            if test_candidate(src, fn_name, test_cases):
                return src
            continue

        path = partial.first_hole_path()
        if path is None:
            continue

        hole_depth = len(path)

        try:
            nt = nonterminal_at_path(partial, path)
        except Exception:
            continue

        prods = grammar.get(nt, [])

        # At max_depth, only allow leaf productions (no children)
        if hole_depth >= max_depth:
            prods = [p for p in prods if not p.children_spec]
            if not prods:
                continue
        nodes_expanded += 1

        for prod in prods:
            new_children = [Hole(child_nt) for child_nt in prod.children_spec]
            replacement = ASTNode(
                kind=prod.name, children=new_children, depth=hole_depth + 1
            )
            new_partial = partial.replace_at_path(path, replacement)
            worklist.push(new_partial, score(new_partial))

    if _interrupted:
        print(
            f"  Stopped after {nodes_expanded} expansions, "
            f"{candidates_tried}/{_fmt_count(candidates_possible)} candidates tested"
        )
        return None

    print(
        f"  Search exhausted after {nodes_expanded} expansions, "
        f"{candidates_tried}/{_fmt_count(candidates_possible)} candidates tested"
    )
    return None


def main():
    parser = argparse.ArgumentParser(description="Rust Program Synthesizer")
    parser.add_argument("--symbols", default="symbols.txt", help="Path to symbols file")
    parser.add_argument("--dataset", default="synthesizer/dataset", help="Directory containing target JSON files")
    parser.add_argument("--target", required=True, help="Target name to synthesize (e.g. add_one)")
    parser.add_argument("--max-depth", type=int, default=8)
    parser.add_argument("--timeout", type=int, default=300, help="Timeout in seconds")
    args = parser.parse_args()

    signal.signal(signal.SIGINT, _handle_sigint)

    try:
        literals = load_symbols(args.symbols)
        target = load_target(args.dataset, args.target)
    except FileNotFoundError as e:
        print(f"Error: {e}")
        sys.exit(1)

    example = target.get("example_rust")
    print(f"Target:    {target['name']}")
    print(f"Signature: pub fn {target['name']}({', '.join(p['name'] + ': ' + p['type'] for p in target['params'])}) -> {target['return_type']}")
    if example:
        print(f"Example:   {example}")
    print(f"Tests:     {len(target['test_cases'])} cases")
    print(f"Literals:  {len(literals)}  Max depth: {args.max_depth}  Timeout: {args.timeout}s")
    print()

    t0 = time.time()
    result = synthesize_target(target, literals, args.max_depth, args.timeout)
    elapsed = time.time() - t0

    if result:
        print(f"  FOUND in {elapsed:.1f}s:\n  {result}")
    elif not _interrupted:
        print(f"  FAILED in {elapsed:.1f}s")


if __name__ == "__main__":
    main()
