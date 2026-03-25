//! Convert a C++ function file to benchmark-style JSON, preserving C++ types.
//!
//! Output schema is similar to `data/benchmark0/processed/*.json`:
//! - `name`
//! - `params`: [{name, type}]
//! - `return_type`
//! - `ast`: statement list in a small operator tree format
//!
//! Usage:
//!   cargo run --bin cpp2json_cpp -- <input.cpp> [--out out.json]

use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use tree_sitter::Node;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("Usage: cpp2json_cpp <input.cpp> [--out out.json]");
        std::process::exit(1);
    }

    let input = PathBuf::from(&args[0]);
    let mut out: Option<PathBuf> = None;
    if args.len() >= 3 && args[1] == "--out" {
        out = Some(PathBuf::from(&args[2]));
    } else if args.len() > 1 {
        bail!("unexpected args: {:?}", &args[1..]);
    }

    let src = fs::read_to_string(&input).with_context(|| format!("read {}", input.display()))?;
    let tree = cpp_preprocessor::parse_cpp_source(&src)?;
    let root = tree.root_node();

    let func = find_first_kind(root, "function_definition")
        .ok_or_else(|| anyhow::anyhow!("no function_definition found"))?;
    let out_json = function_to_json(func, src.as_bytes())?;
    let rendered = serde_json::to_string_pretty(&out_json)?;

    if let Some(path) = out {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, rendered)?;
        eprintln!("Wrote {}", path.display());
    } else {
        println!("{rendered}");
    }
    Ok(())
}

fn function_to_json(func: Node<'_>, src: &[u8]) -> Result<Value> {
    let ret_type = func
        .child_by_field_name("type")
        .map(|n| text(n, src))
        .unwrap_or_else(|| "void".to_string());

    let declarator = func
        .child_by_field_name("declarator")
        .ok_or_else(|| anyhow::anyhow!("function missing declarator"))?;
    let name = function_name(declarator, src).unwrap_or_else(|| "unknown".to_string());
    let params = function_params(declarator, src);

    let body = func
        .child_by_field_name("body")
        .ok_or_else(|| anyhow::anyhow!("function missing body"))?;
    let ast = compound_to_stmts(body, src);

    Ok(json!({
        "name": name,
        "params": params,
        "return_type": if ret_type == "void" { Value::Null } else { Value::String(ret_type) },
        "ast": ast,
    }))
}

fn function_name(fd: Node<'_>, src: &[u8]) -> Option<String> {
    let decl = fd.child_by_field_name("declarator")?;
    Some(base_name(decl, src))
}

fn function_params(fd: Node<'_>, src: &[u8]) -> Vec<Value> {
    let mut out = Vec::new();
    let Some(pl) = fd.child_by_field_name("parameters") else {
        return out;
    };
    for i in 0u32..(pl.named_child_count() as u32) {
        let Some(p) = pl.named_child(i) else { continue };
        if p.kind() != "parameter_declaration" {
            continue;
        }
        let ty = p
            .child_by_field_name("type")
            .map(|n| text(n, src))
            .unwrap_or_default();
        let Some(decl) = p.child_by_field_name("declarator") else {
            continue;
        };
        let name = base_name(decl, src);
        let full_ty = merge_type_and_declarator(&ty, decl, src);
        out.push(json!({ "name": name, "type": full_ty }));
    }
    out
}

fn compound_to_stmts(compound: Node<'_>, src: &[u8]) -> Vec<Value> {
    let mut out = Vec::new();
    for i in 0u32..(compound.named_child_count() as u32) {
        let Some(st) = compound.named_child(i) else { continue };
        if let Some(v) = stmt(st, src) {
            out.push(v);
        }
    }
    out
}

fn stmt(n: Node<'_>, src: &[u8]) -> Option<Value> {
    match n.kind() {
        "declaration" => decl_stmt(n, src),
        "expression_statement" => {
            let expr_node = n.named_child(0)?;
            Some(expr(expr_node, src))
        }
        "return_statement" => {
            let arg = n
                .child_by_field_name("argument")
                .or_else(|| n.named_child(0))
                .map(|a| expr(a, src))
                .unwrap_or_else(|| json!({ "lit": "" }));
            Some(json!({ "op": "return", "args": [arg] }))
        }
        "for_statement" => {
            let init = n
                .child_by_field_name("initializer")
                .and_then(|x| stmt_or_expr(x, src))
                .unwrap_or_else(|| json!({ "op": "nop", "args": [] }));
            let condition = n
                .child_by_field_name("condition")
                .map(|x| expr(x, src))
                .unwrap_or_else(|| json!({ "lit": "true" }));
            let update = n
                .child_by_field_name("update")
                .map(|x| expr(x, src))
                .unwrap_or_else(|| json!({ "op": "nop", "args": [] }));
            let body = n
                .child_by_field_name("body")
                .map(|b| block_to_vec(b, src))
                .unwrap_or_default();
            Some(json!({
                "init": init,
                "condition": condition,
                "update": update,
                "body": body
            }))
        }
        "while_statement" => {
            let condition = n
                .child_by_field_name("condition")
                .map(|x| expr(x, src))
                .unwrap_or_else(|| json!({ "lit": "true" }));
            let body = n
                .child_by_field_name("body")
                .map(|b| block_to_vec(b, src))
                .unwrap_or_default();
            Some(json!({ "condition": condition, "body": body }))
        }
        "if_statement" => {
            let condition = n
                .child_by_field_name("condition")
                .map(|x| expr(x, src))
                .unwrap_or_else(|| json!({ "lit": "false" }));
            let then_body = n
                .child_by_field_name("consequence")
                .map(|b| block_to_vec(b, src))
                .unwrap_or_default();
            let mut obj = serde_json::Map::new();
            obj.insert("condition".to_string(), condition);
            obj.insert("then".to_string(), Value::Array(then_body));
            if let Some(alt) = n.child_by_field_name("alternative") {
                obj.insert("else".to_string(), Value::Array(block_to_vec(alt, src)));
            }
            Some(Value::Object(obj))
        }
        "break_statement" => Some(json!({ "op": "break", "args": [] })),
        "continue_statement" => Some(json!({ "op": "continue", "args": [] })),
        _ => None,
    }
}

fn stmt_or_expr(n: Node<'_>, src: &[u8]) -> Option<Value> {
    stmt(n, src).or_else(|| Some(expr(n, src)))
}

fn block_to_vec(n: Node<'_>, src: &[u8]) -> Vec<Value> {
    if n.kind() == "compound_statement" {
        compound_to_stmts(n, src)
    } else {
        stmt(n, src).into_iter().collect()
    }
}

fn decl_stmt(n: Node<'_>, src: &[u8]) -> Option<Value> {
    // Handle "int x = 0;" and "int x;"
    let mut target: Option<Node<'_>> = None;
    for i in 0u32..(n.named_child_count() as u32) {
        let Some(ch) = n.named_child(i) else { continue };
        if ch.kind() == "init_declarator" {
            target = Some(ch);
            break;
        }
    }
    let d = target?;
    let lhs_decl = d.child_by_field_name("declarator")?;
    let var_name = base_name(lhs_decl, src);
    let rhs = d
        .child_by_field_name("value")
        .map(|v| expr(v, src))
        .unwrap_or_else(|| json!({ "lit": "0" }));
    Some(json!({
        "op": "let",
        "args": [{ "var": var_name }, rhs]
    }))
}

fn expr(n: Node<'_>, src: &[u8]) -> Value {
    match n.kind() {
        "identifier" | "field_identifier" => json!({ "var": text(n, src) }),
        "number_literal" | "char_literal" | "string_literal" | "true" | "false" | "nullptr" => {
            json!({ "lit": text(n, src) })
        }
        "binary_expression" | "assignment_expression" => {
            let op = n
                .child_by_field_name("operator")
                .map(|x| text(x, src))
                .unwrap_or_else(|| "?".to_string());
            let left = n
                .child_by_field_name("left")
                .map(|x| expr(x, src))
                .unwrap_or_else(|| json!({ "lit": "" }));
            let right = n
                .child_by_field_name("right")
                .map(|x| expr(x, src))
                .unwrap_or_else(|| json!({ "lit": "" }));
            json!({ "op": op, "args": [left, right] })
        }
        "update_expression" | "unary_expression" | "pointer_expression" => {
            let op = n
                .child_by_field_name("operator")
                .map(|x| text(x, src))
                .unwrap_or_else(|| "?".to_string());
            let arg = n
                .child_by_field_name("argument")
                .map(|x| expr(x, src))
                .unwrap_or_else(|| json!({ "lit": "" }));
            json!({ "op": op, "args": [arg] })
        }
        "subscript_expression" => {
            let arg = n
                .child_by_field_name("argument")
                .map(|x| expr(x, src))
                .unwrap_or_else(|| json!({ "lit": "" }));
            let idx = n
                .child_by_field_name("index")
                .or_else(|| n.child_by_field_name("indices").and_then(|inds| inds.named_child(0)))
                .map(|x| expr(x, src))
                .unwrap_or_else(|| json!({ "lit": "" }));
            json!({ "op": "[]", "args": [arg, idx] })
        }
        "parenthesized_expression" => n.named_child(0).map(|x| expr(x, src)).unwrap_or(json!({ "lit": "" })),
        _ => json!({ "lit": text(n, src) }),
    }
}

fn merge_type_and_declarator(base_type: &str, declarator: Node<'_>, src: &[u8]) -> String {
    let base = base_type.trim();
    let base_stars = base.matches('*').count();
    let decl_stars = text(declarator, src).matches('*').count();
    let base_clean = base.replace('*', "").trim().to_string();
    let total = base_stars + decl_stars;
    if total == 0 {
        base_clean
    } else {
        format!("{}{}", base_clean, "*".repeat(total))
    }
}

fn base_name(node: Node<'_>, src: &[u8]) -> String {
    match node.kind() {
        "identifier" | "field_identifier" => text(node, src),
        _ => node
            .child_by_field_name("declarator")
            .map(|d| base_name(d, src))
            .or_else(|| node.named_child(0).map(|d| base_name(d, src)))
            .unwrap_or_else(|| text(node, src)),
    }
}

fn find_first_kind<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    if node.kind() == kind {
        return Some(node);
    }
    let mut cursor = node.walk();
    for ch in node.children(&mut cursor) {
        if let Some(x) = find_first_kind(ch, kind) {
            return Some(x);
        }
    }
    None
}

fn text(n: Node<'_>, src: &[u8]) -> String {
    n.utf8_text(src).unwrap_or("").trim().to_string()
}

