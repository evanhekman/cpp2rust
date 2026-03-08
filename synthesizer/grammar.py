from __future__ import annotations
from dataclasses import dataclass
from typing import List, Dict

@dataclass
class Production:
    name: str          # unique production name
    nonterminal: str   # LHS
    children_spec: List[str]   # nonterminals for each child
    rust_template: str  # Python format string

# Base grammar — literals and idents populated dynamically
BASE_GRAMMAR: Dict[str, List[Production]] = {
    "Program": [
        Production("FnDef", "Program", ["FnDef"], "{0}"),
    ],
    "Block": [
        Production("BlockSingle", "Block", ["Stmt"], "{{ {0} }}"),
        Production("BlockMulti", "Block", ["Stmt", "Block"], "{{ {0} {1} }}"),
    ],
    "Stmt": [
        Production("StmtReturn", "Stmt", ["Expr"], "return {0};"),
        Production("StmtExpr", "Stmt", ["Expr"], "{0};"),
        Production("StmtLet", "Stmt", ["Ident", "Expr"], "let {0} = {1};"),
        Production("StmtLetMut", "Stmt", ["Ident", "Expr"], "let mut {0} = {1};"),
        Production("StmtIf", "Stmt", ["Expr", "Block"], "if {0} {1}"),
        Production("StmtIfElse", "Stmt", ["Expr", "Block", "Block"], "if {0} {1} else {2}"),
        Production("StmtWhile", "Stmt", ["Expr", "Block"], "while {0} {1}"),
    ],
    "Expr": [
        Production("ExprAdd", "Expr", ["Expr", "Expr"], "({0} + {1})"),
        Production("ExprSub", "Expr", ["Expr", "Expr"], "({0} - {1})"),
        Production("ExprMul", "Expr", ["Expr", "Expr"], "({0} * {1})"),
        Production("ExprDiv", "Expr", ["Expr", "Expr"], "({0} / {1})"),
        Production("ExprMod", "Expr", ["Expr", "Expr"], "({0} % {1})"),
        Production("ExprEq", "Expr", ["Expr", "Expr"], "({0} == {1})"),
        Production("ExprNe", "Expr", ["Expr", "Expr"], "({0} != {1})"),
        Production("ExprLt", "Expr", ["Expr", "Expr"], "({0} < {1})"),
        Production("ExprGt", "Expr", ["Expr", "Expr"], "({0} > {1})"),
        Production("ExprLe", "Expr", ["Expr", "Expr"], "({0} <= {1})"),
        Production("ExprGe", "Expr", ["Expr", "Expr"], "({0} >= {1})"),
        Production("ExprAnd", "Expr", ["Expr", "Expr"], "({0} && {1})"),
        Production("ExprOr", "Expr", ["Expr", "Expr"], "({0} || {1})"),
        Production("ExprNot", "Expr", ["Expr"], "(!{0})"),
        Production("ExprIfElse", "Expr", ["Expr", "Expr", "Expr"], "if {0} {{ {1} }} else {{ {2} }}"),
        Production("ExprIdent", "Expr", ["Ident"], "{0}"),
        Production("ExprLit", "Expr", ["Literal"], "{0}"),
    ],
    "Literal": [],  # populated from symbols.txt
    "Ident": [],    # populated per-target
}

def build_grammar(literals: List[str], idents: List[str]) -> Dict[str, List[Production]]:
    """Build a grammar with literals and idents filled in."""
    import copy
    g = copy.deepcopy(BASE_GRAMMAR)
    for i, lit in enumerate(literals):
        g["Literal"].append(Production(f"Lit_{i}", "Literal", [], lit))
        # Also add direct Expr → literal so leaves are reachable at any depth
        g["Expr"].append(Production(f"ExprLitDirect_{i}", "Expr", [], lit))
    for i, ident in enumerate(idents):
        g["Ident"].append(Production(f"Id_{i}", "Ident", [], ident))
        # Also add direct Expr → ident so leaves are reachable at any depth
        g["Expr"].append(Production(f"ExprIdentDirect_{i}", "Expr", [], ident))
    return g
