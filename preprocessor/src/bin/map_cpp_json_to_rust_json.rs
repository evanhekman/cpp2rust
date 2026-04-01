//! Map benchmark-style C++ JSON to Rust-typed JSON.
//!
//! Input format (example): `preprocessor/test_outputs/dot_product_cpp.json`
//! Output format: same shape, with `params[].type` and `return_type` mapped to Rust types.
//!
//! Usage:
//!   cargo run -p cpp_preprocessor --bin map_cpp_json_to_rust_json -- <in.json> [--out out.json]

use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("Usage: map_cpp_json_to_rust_json <in.json> [--out out.json]");
        std::process::exit(1);
    }

    let in_path = PathBuf::from(&args[0]);
    let mut out_path: Option<PathBuf> = None;
    if args.len() >= 3 && args[1] == "--out" {
        out_path = Some(PathBuf::from(&args[2]));
    } else if args.len() > 1 {
        bail!("unexpected args: {:?}", &args[1..]);
    }

    let raw = fs::read_to_string(&in_path).with_context(|| format!("read {}", in_path.display()))?;
    let mut doc: Value = serde_json::from_str(&raw).context("parse input JSON")?;

    map_types_in_place(&mut doc);
    drop_len_param_for_slice_style(&mut doc);

    let rendered = serde_json::to_string_pretty(&doc)?;
    if let Some(p) = out_path {
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&p, rendered).with_context(|| format!("write {}", p.display()))?;
        eprintln!("Wrote {}", p.display());
    } else {
        println!("{rendered}");
    }
    Ok(())
}

fn map_types_in_place(doc: &mut Value) {
    if let Some(params) = doc.get_mut("params").and_then(Value::as_array_mut) {
        for p in params {
            let name = p.get("name").and_then(Value::as_str).unwrap_or("").to_string();
            let ty = p.get("type").and_then(Value::as_str).unwrap_or("").to_string();
            let ptr_nullifiable = p
                .get("ptr_nullifiable")
                .or_else(|| p.get("ptr_null_compared_or_assigned"))
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let ptr_used_in_arithmetic = p
                .get("ptr_used_in_arithmetic")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let ptr_associated_with_new_delete = p
                .get("ptr_associated_with_new_delete")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if ty.is_empty() {
                continue;
            }
            let is_ptr = ty.contains('*');
            let mapped = if is_ptr {
                map_cpp_pointer_type_to_rust(
                    &ty,
                    ptr_used_in_arithmetic,
                    ptr_associated_with_new_delete,
                )
            } else {
                map_cpp_type_to_rust(&ty)
            };
            let mapped = if is_ptr && ptr_nullifiable {
                format!("Option<{mapped}>")
            } else {
                mapped
            };
            if is_ptr {
                *p = json!({
                    "name": name,
                    "type": mapped,
                    "ptr_nullifiable": ptr_nullifiable,
                    "ptr_used_in_arithmetic": ptr_used_in_arithmetic,
                    "ptr_associated_with_new_delete": ptr_associated_with_new_delete
                });
            } else {
                *p = json!({ "name": name, "type": mapped });
            }
        }
    }

    if let Some(ret) = doc.get_mut("return_type") {
        match ret {
            Value::Null => { *ret = Value::String("()".into()); }
            Value::String(s) => {
                if s.trim() == "void" {
                    *ret = Value::String("()".into());
                } else {
                    let mapped = if s.contains('*') {
                        map_cpp_pointer_type_to_rust(s, false, false)
                    } else {
                        map_cpp_type_to_rust(s)
                    };
                    *ret = Value::String(mapped);
                }
            }
            _ => {}
        }
    }

    if let Some(ast) = doc.get_mut("ast") {
        map_types_in_value(ast);
    }
}

fn map_types_in_value(v: &mut Value) {
    match v {
        Value::Array(items) => {
            for item in items {
                map_types_in_value(item);
            }
        }
        Value::Object(obj) => {
            if let Some(ty) = obj.get("type").and_then(Value::as_str) {
                if !is_probably_rust_type(ty) {
                    let ptr_nullifiable = obj
                        .get("ptr_nullifiable")
                        .or_else(|| obj.get("ptr_null_compared_or_assigned"))
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                    let ptr_used_in_arithmetic = obj
                        .get("ptr_used_in_arithmetic")
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                    let ptr_associated_with_new_delete = obj
                        .get("ptr_associated_with_new_delete")
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                    let is_ptr = ty.contains('*');
                    let mapped = if is_ptr {
                        map_cpp_pointer_type_to_rust(
                            ty,
                            ptr_used_in_arithmetic,
                            ptr_associated_with_new_delete,
                        )
                    } else {
                        map_cpp_type_to_rust(ty)
                    };
                    let mapped = if is_ptr && ptr_nullifiable {
                        format!("Option<{mapped}>")
                    } else {
                        mapped
                    };
                    obj.insert("type".to_string(), Value::String(mapped));
                }
            }
            for value in obj.values_mut() {
                map_types_in_value(value);
            }
        }
        _ => {}
    }
}

fn is_probably_rust_type(ty: &str) -> bool {
    if ty.contains('&') || ty.starts_with("Box<") || ty.starts_with("Option<") {
        return true;
    }
    matches!(
        ty,
        "bool"
            | "u8"
            | "i8"
            | "u16"
            | "i16"
            | "u32"
            | "i32"
            | "u64"
            | "i64"
            | "usize"
            | "f32"
            | "f64"
    )
}

fn drop_len_param_for_slice_style(doc: &mut Value) {
    let Some(params) = doc.get_mut("params").and_then(Value::as_array_mut) else {
        return;
    };
    let has_slice_like = params.iter().any(|p| {
        p.get("type")
            .and_then(Value::as_str)
            .map(|t| t.starts_with("&[") || t.starts_with("&mut ["))
            .unwrap_or(false)
    });
    if !has_slice_like {
        return;
    }
    params.retain(|p| {
        let name = p.get("name").and_then(Value::as_str).unwrap_or("");
        let ty = p.get("type").and_then(Value::as_str).unwrap_or("");
        let is_len = name == "n" || name == "len" || name == "size";
        let is_int_like = matches!(ty, "i32" | "u32" | "usize");
        !(is_len && is_int_like)
    });
}

fn map_cpp_type_to_rust(cpp_type: &str) -> String {
    let t = normalize_cpp_type(cpp_type);
    map_cpp_base_type_to_rust(&t)
}

fn map_cpp_pointer_type_to_rust(
    cpp_type: &str,
    ptr_used_in_arithmetic: bool,
    ptr_associated_with_new_delete: bool,
) -> String {
    let t = normalize_cpp_type(cpp_type);
    let base_cpp = strip_pointer_suffix(&t);
    let base = map_cpp_base_type_to_rust(base_cpp);
    if ptr_associated_with_new_delete {
        format!("Box<{base}>")
    } else if ptr_used_in_arithmetic {
        format!("&[{base}]")
    } else {
        format!("&{base}")
    }
}

fn normalize_cpp_type(cpp_type: &str) -> String {
    let mut t = cpp_type.replace("const", "");
    t = t.split_whitespace().collect::<Vec<_>>().join(" ");
    t.trim().to_string()
}

fn strip_pointer_suffix(ty: &str) -> &str {
    let mut out = ty.trim_end();
    while out.ends_with('*') {
        out = out.trim_end_matches('*').trim_end();
    }
    out.trim()
}

fn map_cpp_base_type_to_rust(t: &str) -> String {
    match t {
        "bool" => "bool".to_string(),
        "uint8_t" | "unsigned char" => "u8".to_string(),
        "int8_t" | "char" | "signed char" => "i8".to_string(),
        "uint16_t" | "unsigned short" => "u16".to_string(),
        "int16_t" | "short" => "i16".to_string(),
        "uint32_t" | "unsigned int" => "u32".to_string(),
        "int32_t" | "int" => "i32".to_string(),
        "uint64_t" | "unsigned long long" => "u64".to_string(),
        "int64_t" | "long long" => "i64".to_string(),
        "size_t" => "usize".to_string(),
        "float" => "f32".to_string(),
        "double" => "f64".to_string(),
        other => other.to_string(),
    }
}

