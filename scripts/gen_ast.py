#!/Users/evanhekman/cpp2rust/.venv/bin/python3

import tree_sitter_cpp as tscpp
from tree_sitter import Language, Parser
import json
import os
import re


def dumps(obj):
    s = json.dumps(obj, indent=2)
    s = re.sub(r'\{\s*\n\s*"(var|lit)": (".*?")\s*\n\s*\}', r'{ "\1": \2 }', s)
    return s

CPP_LANGUAGE = Language(tscpp.language())
parser = Parser(CPP_LANGUAGE)


def fld(node, name):
    return node.child_by_field_name(name)


def base_name(node):
    """Unwrap nested declarator nodes to get the base identifier name."""
    if node.type == 'identifier':
        return node.text.decode()
    inner = fld(node, 'declarator')
    if inner:
        return base_name(inner)
    return node.text.decode()


def expr(node):
    t = node.type

    if t == 'identifier':
        return {'var': node.text.decode()}

    if t in ('number_literal', 'char_literal', 'string_literal', 'true', 'false', 'nullptr'):
        return {'lit': node.text.decode()}

    if t in ('binary_expression', 'assignment_expression'):
        return {
            'op': fld(node, 'operator').text.decode(),
            'args': [expr(fld(node, 'left')), expr(fld(node, 'right'))]
        }

    if t == 'update_expression':
        return {
            'op': fld(node, 'operator').text.decode(),
            'args': [expr(fld(node, 'argument'))]
        }

    if t in ('unary_expression', 'pointer_expression'):
        return {
            'op': fld(node, 'operator').text.decode(),
            'args': [expr(fld(node, 'argument'))]
        }

    if t == 'subscript_expression':
        indices = fld(node, 'indices')
        index = next(c for c in indices.named_children)
        return {'op': '[]', 'args': [expr(fld(node, 'argument')), expr(index)]}

    if t == 'parenthesized_expression':
        return expr(node.named_children[0])

    if t == 'condition_clause':
        return expr(fld(node, 'value'))

    # Fallback
    return {'lit': node.text.decode()}


def decl_stmt(node):
    declarator = fld(node, 'declarator')
    if declarator is None:
        return None
    if declarator.type == 'init_declarator':
        name = base_name(fld(declarator, 'declarator'))
        value = fld(declarator, 'value')
        return {'op': 'let', 'args': [{'var': name}, expr(value)]}
    return {'op': 'let', 'args': [{'var': base_name(declarator)}]}


def body_stmts(node):
    if node.type == 'compound_statement':
        result = []
        for child in node.named_children:
            s = stmt(child)
            if s is None:
                continue
            if isinstance(s, list):
                result.extend(s)
            else:
                result.append(s)
        return result
    else:
        s = stmt(node)
        if s is None:
            return []
        return s if isinstance(s, list) else [s]


def stmt(node):
    t = node.type

    if t == 'compound_statement':
        return body_stmts(node)

    if t == 'expression_statement':
        for child in node.named_children:
            return expr(child)
        return None

    if t == 'declaration':
        return decl_stmt(node)

    if t == 'return_statement':
        children = node.named_children
        if children:
            return {'op': 'return', 'args': [expr(children[0])]}
        return {'op': 'return', 'args': []}

    if t == 'throw_statement':
        children = node.named_children
        if children:
            return {'op': 'throw', 'args': [expr(children[0])]}
        return {'op': 'throw', 'args': []}

    if t == 'for_statement':
        result = {}
        init = fld(node, 'initializer')
        cond = fld(node, 'condition')
        update = fld(node, 'update')
        body = fld(node, 'body')
        if init:
            result['init'] = stmt(init)
        if cond:
            result['condition'] = expr(cond)
        if update:
            result['update'] = expr(update)
        if body:
            result['body'] = body_stmts(body)
        return result

    if t == 'while_statement':
        cond = fld(node, 'condition')
        body = fld(node, 'body')
        result = {}
        if cond:
            result['condition'] = expr(cond)
        if body:
            result['body'] = body_stmts(body)
        return result

    if t == 'if_statement':
        cond = fld(node, 'condition')
        cons = fld(node, 'consequence')
        alt = fld(node, 'alternative')
        result = {'condition': expr(cond)}
        if cons:
            result['then'] = body_stmts(cons)
        if alt:
            result['else'] = body_stmts(alt)
        return result

    if t == 'try_statement':
        body = fld(node, 'body')
        catch_nodes = [c for c in node.named_children if c.type == 'catch_clause']
        result = {'body': body_stmts(body)}
        if catch_nodes:
            catch = catch_nodes[0]
            params = fld(catch, 'parameters')
            catch_body = fld(catch, 'body')
            param_name = None
            if params and params.named_children:
                decl_node = fld(params.named_children[0], 'declarator')
                if decl_node:
                    param_name = base_name(decl_node)
            result['catch'] = {
                'param': {'var': param_name} if param_name else None,
                'body': body_stmts(catch_body) if catch_body else []
            }
        return result

    return None


def find_function(node):
    if node.type == 'function_definition':
        return node
    for child in node.named_children:
        result = find_function(child)
        if result:
            return result
    return None


files = [
    'data/benchmark0/cpp/dot_product.cpp',
    'data/benchmark0/cpp/reverse.cpp',
    'data/benchmark0/cpp/max_even_indexed.cpp',
    'data/benchmark0/cpp/exception.cpp',
]

for filepath in files:
    name = os.path.basename(filepath).replace('.cpp', '')

    sig_path = f'data/benchmark0/processed/{name}.json'
    with open(sig_path) as f:
        sig = json.load(f)
    sig.pop('ast', None)

    with open(filepath, 'rb') as f:
        source = f.read()
    tree = parser.parse(source)
    func_node = find_function(tree.root_node)
    ast = body_stmts(func_node.child_by_field_name('body'))

    unified = {**sig, 'ast': ast}
    with open(sig_path, 'w') as f:
        f.write(dumps(unified))
    print(f'wrote {sig_path}')
