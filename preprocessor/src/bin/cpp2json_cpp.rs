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
use regex::Regex;
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
    let out_json = function_to_json(func, &src, src.as_bytes())?;
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

fn function_to_json(func: Node<'_>, whole_src: &str, src: &[u8]) -> Result<Value> {
    let ret_type = func
        .child_by_field_name("type")
        .map(|n| text(n, src))
        .unwrap_or_else(|| "void".to_string());

    let declarator = func
        .child_by_field_name("declarator")
        .ok_or_else(|| anyhow::anyhow!("function missing declarator"))?;
    let name = function_name(declarator, src).unwrap_or_else(|| "unknown".to_string());
    let params = function_params(declarator, whole_src, src);

    let body = func
        .child_by_field_name("body")
        .ok_or_else(|| anyhow::anyhow!("function missing body"))?;
    let ast = compound_to_stmts(body, whole_src, src);

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

fn function_params(fd: Node<'_>, whole_src: &str, src: &[u8]) -> Vec<Value> {
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
        let is_ptr = full_ty.contains('*');
        if is_ptr {
            out.push(json!({
                "name": name,
                "type": full_ty,
                "ptr_nullifiable": has_nullptr_usage(whole_src, &name),
                "ptr_used_in_arithmetic": has_pointer_arithmetic_usage(whole_src, &name),
                "ptr_associated_with_new_delete": has_new_delete_usage(whole_src, &name),
                "ptr_data_mutated": has_pointer_write_through(whole_src, &name)
            }));
        } else {
            out.push(json!({ "name": name, "type": full_ty }));
        }
    }
    out
}

fn compound_to_stmts(compound: Node<'_>, whole_src: &str, src: &[u8]) -> Vec<Value> {
    let mut out = Vec::new();
    for i in 0u32..(compound.named_child_count() as u32) {
        let Some(st) = compound.named_child(i) else { continue };
        if let Some(v) = stmt(st, whole_src, src) {
            out.push(v);
        }
    }
    out
}

fn stmt(n: Node<'_>, whole_src: &str, src: &[u8]) -> Option<Value> {
    match n.kind() {
        "declaration" => decl_stmt(n, whole_src, src),
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
                .and_then(|x| stmt_or_expr(x, whole_src, src))
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
                .map(|b| block_to_vec(b, whole_src, src))
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
                .map(|b| block_to_vec(b, whole_src, src))
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
                .map(|b| block_to_vec(b, whole_src, src))
                .unwrap_or_default();
            let mut obj = serde_json::Map::new();
            obj.insert("condition".to_string(), condition);
            obj.insert("then".to_string(), Value::Array(then_body));
            if let Some(alt) = n.child_by_field_name("alternative") {
                obj.insert("else".to_string(), Value::Array(block_to_vec(alt, whole_src, src)));
            }
            Some(Value::Object(obj))
        }
        "throw_statement" => {
            let arg = n.named_child(0).map(|a| expr(a, src))
                .unwrap_or_else(|| json!({ "lit": "" }));
            Some(json!({ "op": "throw", "args": [arg] }))
        }
        "try_statement" => {
            let body = n.child_by_field_name("body")
                .map(|b| block_to_vec(b, whole_src, src))
                .unwrap_or_default();
            let catch = n.children(&mut n.walk())
                .find(|c| c.kind() == "catch_clause")
                .map(|c| {
                    let param = c.child_by_field_name("parameters")
                        .and_then(|p| p.named_child(0))
                        .and_then(|p| p.child_by_field_name("declarator")
                            .or_else(|| p.named_child(0)))
                        .map(|p| json!({ "var": text(p, src) }))
                        .unwrap_or(Value::Null);
                    let catch_body = c.child_by_field_name("body")
                        .map(|b| block_to_vec(b, whole_src, src))
                        .unwrap_or_default();
                    json!({ "param": param, "body": catch_body })
                })
                .unwrap_or(Value::Null);
            Some(json!({ "body": body, "catch": catch }))
        }
        "break_statement" => Some(json!({ "op": "break", "args": [] })),
        "continue_statement" => Some(json!({ "op": "continue", "args": [] })),
        _ => None,
    }
}

fn stmt_or_expr(n: Node<'_>, whole_src: &str, src: &[u8]) -> Option<Value> {
    stmt(n, whole_src, src).or_else(|| Some(expr(n, src)))
}

fn block_to_vec(n: Node<'_>, whole_src: &str, src: &[u8]) -> Vec<Value> {
    if n.kind() == "compound_statement" {
        compound_to_stmts(n, whole_src, src)
    } else {
        stmt(n, whole_src, src).into_iter().collect()
    }
}

fn decl_stmt(n: Node<'_>, whole_src: &str, src: &[u8]) -> Option<Value> {
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
    let base_ty = n
        .child_by_field_name("type")
        .map(|t| text(t, src))
        .unwrap_or_default();
    let lhs_decl = d.child_by_field_name("declarator")?;
    let var_name = base_name(lhs_decl, src);
    let full_ty = merge_type_and_declarator(&base_ty, lhs_decl, src);
    let is_ptr = full_ty.contains('*');
    let ptr_null_compared_or_assigned = if is_ptr {
        has_nullptr_usage(whole_src, &var_name)
    } else {
        false
    };
    let rhs = d
        .child_by_field_name("value")
        .map(|v| expr(v, src))
        .unwrap_or_else(|| json!({ "lit": "0" }));
    let lhs = if is_ptr {
        json!({
            "var": var_name,
            "type": full_ty,
            "ptr_null_compared_or_assigned": ptr_null_compared_or_assigned,
            "ptr_used_in_arithmetic": has_pointer_arithmetic_usage(whole_src, &var_name),
            "ptr_associated_with_new_delete": has_new_delete_usage(whole_src, &var_name)
        })
    } else {
        json!({ "var": var_name, "type": full_ty })
    };
    Some(json!({ "op": "let", "args": [lhs, rhs] }))
}

fn expr(n: Node<'_>, src: &[u8]) -> Value {
    match n.kind() {
        "condition_clause" => n
            .child_by_field_name("value")
            .map(|v| expr(v, src))
            .unwrap_or_else(|| json!({ "lit": text(n, src) })),
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

fn has_nullptr_usage(source: &str, var_name: &str) -> bool {
    let v = regex::escape(var_name);
    let vp = format!(r"\b{v}\b");
    let np = r"(nullptr|NULL)";
    let patterns = [
        // comparisons
        format!(r"{vp}\s*==\s*{np}"),
        format!(r"{vp}\s*!=\s*{np}"),
        format!(r"{np}\s*==\s*{vp}"),
        format!(r"{np}\s*!=\s*{vp}"),
        // assignments
        format!(r"{vp}\s*=\s*{np}"),
        format!(r"{vp}\s*=\s*\(?\s*{np}\s*\)?"),
        // guard conditions
        format!(r"(?:if|while)\s*\(\s*!?\s*{vp}\s*\)"),
    ];
    patterns
        .iter()
        .filter_map(|p| Regex::new(p).ok())
        .any(|re| re.is_match(source))
}

fn has_pointer_arithmetic_usage(source: &str, var_name: &str) -> bool {
    let v = regex::escape(var_name);
    let vp = format!(r"\b{v}\b");
    let patterns = [
        // ++p / p++ / --p / p--
        format!(r"(?:\+\+|--)\s*{vp}"),
        format!(r"{vp}\s*(?:\+\+|--)"),
        // p += k / p -= k
        format!(r"{vp}\s*(?:\+=|-=)\s*[^;,\)\]]+"),
        // p + k / p - k
        format!(r"{vp}\s*[+-]\s*[^;,\)\]]+"),
        // indexing p[i]
        format!(r"{vp}\s*\["),
    ];
    patterns
        .iter()
        .filter_map(|p| Regex::new(p).ok())
        .any(|re| re.is_match(source))
}

/// Returns true if the data pointed to by `var_name` is written through in `source`.
/// Covers:
///   - direct writes: `*var = ...` or `var[i] = ...`
///   - transitive writes: a local pointer initialised from `var` is later written through
fn has_pointer_write_through(source: &str, var_name: &str) -> bool {
    if has_direct_deref_write(source, var_name) {
        return true;
    }
    for derived in derived_pointer_vars(source, var_name) {
        if has_direct_deref_write(source, &derived) {
            return true;
        }
    }
    false
}

/// Check for `*var = ...` (not `==`) or `var[...] = ...` (not `==`).
/// Uses `=[^=]` instead of a lookahead since the regex crate doesn't support lookaheads.
fn has_direct_deref_write(source: &str, var_name: &str) -> bool {
    let v = regex::escape(var_name);
    let patterns = [
        format!(r"\*\s*\b{v}\b\s*=[^=]"),
        format!(r"\b{v}\b\s*\[[^\]]*\]\s*=[^=]"),
    ];
    patterns
        .iter()
        .filter_map(|p| Regex::new(p).ok())
        .any(|re| re.is_match(source))
}

/// Find names of local pointer variables initialised from an expression containing `var_name`.
/// Matches declarations of the form `T* local = ... var_name ...;`.
fn derived_pointer_vars(source: &str, var_name: &str) -> Vec<String> {
    let v = regex::escape(var_name);
    // `\bTYPE\b \* local = ... var_name ...;`
    let pat = format!(r"\b\w+\s*\*+\s*(\w+)\s*=\s*[^;]*\b{v}\b");
    let re = match Regex::new(&pat) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    re.captures_iter(source)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
        .collect()
}

fn has_new_delete_usage(source: &str, var_name: &str) -> bool {
    let v = regex::escape(var_name);
    let vp = format!(r"\b{v}\b");
    let patterns = [
        // p = new T(...) / p = new T[...] / p = (new T)
        format!(r"{vp}\s*=\s*\(?\s*new\b"),
        // delete p; / delete[] p;
        format!(r"delete\s*(?:\[\s*\])?\s*{vp}\b"),
    ];
    patterns
        .iter()
        .filter_map(|p| Regex::new(p).ok())
        .any(|re| re.is_match(source))
}

