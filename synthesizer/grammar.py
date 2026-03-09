from __future__ import annotations
import copy
from dataclasses import dataclass
from typing import List, Dict


@dataclass
class Production:
    name: str           # unique production name
    nonterminal: str    # LHS
    children_spec: List[str]    # nonterminals for each child slot
    rust_template: str  # Python format string


# Base grammar — StmtReturn and leaves populated dynamically per target
BASE_GRAMMAR: Dict[str, List[Production]] = {
    "Block": [
        Production("BlockSingle", "Block", ["Stmt"], "{{ {0} }}"),
    ],
    "Stmt": [
        # StmtReturn added dynamically based on return type
        Production("StmtIf",     "Stmt", ["Expr_bool", "Block"],         "if {0} {1}"),
        Production("StmtIfElse", "Stmt", ["Expr_bool", "Block", "Block"], "if {0} {1} else {2}"),
    ],
    "Expr_i32": [
        Production("ExprAdd", "Expr_i32", ["Expr_i32", "Expr_i32"], "({0} + {1})"),
        Production("ExprSub", "Expr_i32", ["Expr_i32", "Expr_i32"], "({0} - {1})"),
        Production("ExprMul", "Expr_i32", ["Expr_i32", "Expr_i32"], "({0} * {1})"),
        Production("ExprDiv", "Expr_i32", ["Expr_i32", "Expr_i32"], "({0} / {1})"),
        Production("ExprMod", "Expr_i32", ["Expr_i32", "Expr_i32"], "({0} % {1})"),
        Production("ExprIfElse_i32", "Expr_i32", ["Expr_bool", "Expr_i32", "Expr_i32"],
                   "if {0} {{ {1} }} else {{ {2} }}"),
        # i32 literals and idents added dynamically
    ],
    "Expr_bool": [
        Production("ExprEq",  "Expr_bool", ["Expr_i32", "Expr_i32"], "({0} == {1})"),
        Production("ExprNe",  "Expr_bool", ["Expr_i32", "Expr_i32"], "({0} != {1})"),
        Production("ExprLt",  "Expr_bool", ["Expr_i32", "Expr_i32"], "({0} < {1})"),
        Production("ExprGt",  "Expr_bool", ["Expr_i32", "Expr_i32"], "({0} > {1})"),
        Production("ExprLe",  "Expr_bool", ["Expr_i32", "Expr_i32"], "({0} <= {1})"),
        Production("ExprGe",  "Expr_bool", ["Expr_i32", "Expr_i32"], "({0} >= {1})"),
        Production("ExprAnd", "Expr_bool", ["Expr_bool", "Expr_bool"], "({0} && {1})"),
        Production("ExprOr",  "Expr_bool", ["Expr_bool", "Expr_bool"], "({0} || {1})"),
        Production("ExprNot", "Expr_bool", ["Expr_bool"], "(!{0})"),
        Production("ExprIfElse_bool", "Expr_bool", ["Expr_bool", "Expr_bool", "Expr_bool"],
                   "if {0} {{ {1} }} else {{ {2} }}"),
        # bool literals and idents added dynamically
    ],
}

_BOOL_LITERALS = {"true", "false"}


def build_grammar(
    literals: List[str],
    params: List[Dict],  # [{"name": ..., "type": ...}]
    return_type: str,
) -> Dict[str, List[Production]]:
    g = copy.deepcopy(BASE_GRAMMAR)

    # StmtReturn typed to the function's return type
    expr_nt = f"Expr_{return_type}"
    g["Stmt"].append(
        Production("StmtReturn", "Stmt", [expr_nt], "return {0};")
    )

    # Literals → typed Expr leaves
    for i, lit in enumerate(literals):
        nt = "Expr_bool" if lit in _BOOL_LITERALS else "Expr_i32"
        g[nt].append(Production(f"ExprLit_{i}", nt, [], lit))

    # Idents → typed Expr leaves based on param type
    for i, param in enumerate(params):
        nt = f"Expr_{param['type']}"
        g.setdefault(nt, []).append(
            Production(f"ExprIdent_{i}", nt, [], param["name"])
        )

    return g
