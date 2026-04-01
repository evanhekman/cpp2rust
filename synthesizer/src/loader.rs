use crate::eval::Value;
use crate::grammar::is_slice_type;
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Deserialize, Clone, Debug)]
pub struct Param {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct TestCase {
    pub inputs: Vec<String>,
    pub expected_output: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct CppFeatures {
    pub operator_counts: std::collections::HashMap<String, usize>,
    pub operator_sequence: Vec<String>,
}

/// Deserialized from `data/<benchmark>/processed/<name>.json`.
///
/// Required fields: name, params, return_type, ast.
/// Optional: example_rust, cpp_features, test_cases (generated from C++ oracle if absent).
///
/// # AST format
/// Top-level array of statement objects. Each statement is one of:
/// ```json
/// {"op":"let",    "args":[{"var":"x"}, <expr>]}
/// {"op":"return", "args":[<expr>]}
/// {"op":"throw",  "args":[<expr>]}                      // becomes early return in Rust
/// {"op":"=",      "args":[{"var":"x"}, <expr>]}
/// {"op":"+=",     "args":[{"var":"x"}, <expr>]}
/// {"op":"-=",     "args":[{"var":"x"}, <expr>]}
/// {"op":"++",     "args":[{"var":"x"}]}
/// {"op":"--",     "args":[{"var":"x"}]}
/// {"condition":..., "then":[...], "else":[...]}          // if; else is optional
/// {"init":..., "condition":..., "update":..., "body":[...]}  // for loop
/// {"condition":..., "body":[...]}                        // while loop
/// {"body":[...], "catch":...}                            // try/catch; body visited, catch ignored
/// ```
/// Expressions are one of:
/// ```json
/// {"var": "x"}                          // variable reference
/// {"lit": "0"}                          // literal
/// {"op": "+", "args":[<expr>, <expr>]}  // binary/unary op
/// {"op": "[]", "args":[<expr>, <expr>]} // array index
/// {"op": "*",  "args":[<expr>]}         // pointer dereference
/// ```
#[derive(Deserialize, Clone, Debug)]
pub struct Target {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: String,
    pub example_rust: Option<String>,
    pub cpp_features: Option<CppFeatures>,
    #[serde(default)]
    pub test_cases: Vec<TestCase>,
    // Raw C++ AST from processed/ format
    #[serde(default)]
    pub ast: Option<serde_json::Value>,
    // Derived expected-Rust-node sequence for the AST heuristic (not in JSON)
    #[serde(skip)]
    pub ast_hints: Option<Vec<String>>,
    // ExprIdent production names that must appear in the synthesized program (not in JSON)
    #[serde(skip)]
    pub required_idents: Vec<String>,
    // Expected block sizes in DFS pre-order (one per Block node in the correct tree)
    #[serde(skip)]
    pub block_sizes: Vec<usize>,
    // Local variables extracted from the C++ AST: (name, rust_type)
    #[serde(skip)]
    pub local_vars: Vec<(String, String)>,
    // Path to the original C++ source file (derived from JSON path, not stored in JSON)
    #[serde(skip)]
    pub cpp_source: Option<PathBuf>,
}

pub fn load_target(dataset_dir: &Path, name: &str) -> Result<Target, String> {
    let path = dataset_dir.join(format!("{}.json", name));
    if !path.exists() {
        let available: Vec<String> = std::fs::read_dir(dataset_dir)
            .map(|rd| {
                rd.filter_map(|e| {
                    let e = e.ok()?;
                    let n = e.file_name().into_string().ok()?;
                    n.ends_with(".json").then(|| n[..n.len() - 5].to_string())
                })
                .collect()
            })
            .unwrap_or_default();
        let mut available = available;
        available.sort();
        return Err(format!(
            "Target '{}' not found in {:?}\nAvailable: {}",
            name,
            dataset_dir,
            available.join(", ")
        ));
    }
    let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    finish_load(serde_json::from_str(&text).map_err(|e| e.to_string())?, &path)
}

pub fn load_target_file(path: &Path) -> Result<Target, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    finish_load(serde_json::from_str(&text).map_err(|e| e.to_string())?, path)
}

/// Resolve the C++ source file for a processed JSON at `json_path`.
/// Convention: `processed/foo.json` → `../cpp/foo.cpp`.
fn find_cpp_source(json_path: &Path) -> Option<PathBuf> {
    let stem = json_path.file_stem()?.to_str()?;
    let candidate = json_path
        .parent()?
        .join("../cpp")
        .join(format!("{}.cpp", stem));
    candidate.canonicalize().ok()
}

/// Post-process a deserialized Target: derive ast_hints, local_vars, and cpp_source.
fn finish_load(mut t: Target, json_path: &Path) -> Result<Target, String> {
    t.cpp_source = find_cpp_source(json_path);
    if let Some(ast) = &t.ast.clone() {
        let slice_names: Vec<&str> = t.params.iter()
            .filter(|p| is_slice_type(&p.ty))
            .map(|p| p.name.as_str())
            .collect();
        t.ast_hints = Some(extract_ast_hints(ast, &slice_names));
        t.block_sizes = extract_block_sizes(ast);
        t.local_vars = extract_local_vars(ast, &t.params, &t.return_type);
        t.required_idents = extract_required_idents(ast, &t.params, &t.local_vars);
    }
    Ok(t)
}

// ── AST hint extraction ───────────────────────────────────────────────────────

/// Walk the C++ AST (processed/ format) and produce an ordered sequence of
/// expected Rust grammar node-kind prefixes, in DFS pre-order.
pub fn extract_ast_hints(ast: &serde_json::Value, slice_params: &[&str]) -> Vec<String> {
    let mut seq = Vec::new();
    if let Some(stmts) = ast.as_array() {
        for stmt in stmts {
            visit_stmt(stmt, slice_params, &mut seq);
        }
    }
    seq
}

/// Walk the C++ AST and collect expected block sizes in DFS pre-order.
/// Emits one entry per block-like construct (function body, loop body, if-then, if-else).
pub fn extract_block_sizes(ast: &serde_json::Value) -> Vec<usize> {
    let mut sizes = Vec::new();
    if let Some(stmts) = ast.as_array() {
        sizes.push(stmts.len());
        for stmt in stmts {
            collect_block_sizes_stmt(stmt, &mut sizes);
        }
    }
    sizes
}

fn collect_block_sizes_stmt(node: &serde_json::Value, sizes: &mut Vec<usize>) {
    // For loop: has "init", "condition", "update", "body"
    if node.get("init").is_some() {
        if let Some(body) = node["body"].as_array() {
            sizes.push(body.len());
            for s in body { collect_block_sizes_stmt(s, sizes); }
        }
        return;
    }
    // While loop: "condition" + "body", no "init"
    if node.get("condition").is_some() && node.get("body").is_some() {
        if let Some(body) = node["body"].as_array() {
            sizes.push(body.len());
            for s in body { collect_block_sizes_stmt(s, sizes); }
        }
        return;
    }
    // If statement: "condition" + "then" (optional "else")
    if node.get("condition").is_some() && node.get("then").is_some() {
        if let Some(then) = node["then"].as_array() {
            sizes.push(then.len());
            for s in then { collect_block_sizes_stmt(s, sizes); }
        }
        if let Some(els) = node.get("else").and_then(|v| v.as_array()) {
            sizes.push(els.len());
            for s in els { collect_block_sizes_stmt(s, sizes); }
        }
        return;
    }
    // Try/catch: visit body
    if node.get("catch").is_some() {
        if let Some(body) = node["body"].as_array() {
            sizes.push(body.len());
            for s in body { collect_block_sizes_stmt(s, sizes); }
        }
    }
}

fn visit_stmt(node: &serde_json::Value, slices: &[&str], seq: &mut Vec<String>) {
    // ── For loop: has "init", "condition", "update", "body" ──────────────────
    if node.get("init").is_some() {
        let ptr_loop = is_pointer_init(&node["init"], slices);
        let hint = if ptr_loop {
            "StmtWhile".to_string()
        } else {
            // Emit variable-specific StmtFor_<varname> hint
            let loop_var = node["init"]["args"].as_array()
                .and_then(|a| a.first())
                .and_then(|v| v.get("var"))
                .and_then(|v| v.as_str());
            match loop_var {
                Some(v) => format!("StmtFor_{}", v),
                None => "StmtFor".to_string(),
            }
        };
        seq.push(hint);
        if ptr_loop {
            visit_expr(&node["condition"], slices, seq);
        } else {
            // Standard C++ for loop: upper bound is `n` (slice length).
            // Emit ExprLen hint so the search prefers a.len() over literal ranges.
            let upper_is_var = node["condition"]["args"].as_array()
                .and_then(|a| a.get(1))
                .and_then(|v| v.get("var")).is_some();
            if upper_is_var {
                seq.push("ExprLen".to_string());
            }
        }
        visit_stmts(&node["body"], slices, seq);
        return;
    }

    // ── While loop: "condition" + "body", no "init" ──────────────────────────
    if node.get("condition").is_some() && node.get("body").is_some() {
        seq.push("StmtWhile".into());
        visit_expr(&node["condition"], slices, seq);
        visit_stmts(&node["body"], slices, seq);
        return;
    }

    // ── If statement: "condition" + "then" ───────────────────────────────────
    if node.get("condition").is_some() && node.get("then").is_some() {
        seq.push("StmtIf".into());
        visit_expr(&node["condition"], slices, seq);
        visit_stmts(&node["then"], slices, seq);
        return;
    }

    // ── Try/catch: transparent — visit body (throw becomes StmtReturn) ───────
    if node.get("catch").is_some() {
        visit_stmts(&node["body"], slices, seq);
        return;
    }

    // ── Statement with an "op" field ─────────────────────────────────────────
    let op = match node.get("op").and_then(|v| v.as_str()) {
        Some(o) => o,
        None => return,
    };
    let args = node["args"].as_array();

    match op {
        "let" => {
            // Emit variable-specific StmtLetMut_<varname>
            let var_name = args.and_then(|a| a.first())
                .and_then(|v| v.get("var")).and_then(|v| v.as_str());
            seq.push(match var_name {
                Some(v) => format!("StmtLetMut_{}", v),
                None => "StmtLetMut".to_string(),
            });
            if let Some(a) = args { if a.len() > 1 { visit_expr(&a[1], slices, seq); } }
        }
        "return" => {
            seq.push("StmtReturn".into());
            if let Some(a) = args { if let Some(e) = a.first() { visit_expr(e, slices, seq); } }
        }
        "throw" => {
            // throw → early return in Rust
            seq.push("StmtReturn".into());
            if let Some(a) = args { if let Some(e) = a.first() { visit_expr(e, slices, seq); } }
        }
        "+=" => {
            let var_name = args.and_then(|a| a.first())
                .and_then(|v| v.get("var")).and_then(|v| v.as_str());
            seq.push(match var_name {
                Some(v) => format!("StmtCompoundPlus_{}", v),
                None => "StmtCompoundPlus".to_string(),
            });
            if let Some(a) = args { if a.len() > 1 { visit_expr(&a[1], slices, seq); } }
        }
        "-=" => {
            let var_name = args.and_then(|a| a.first())
                .and_then(|v| v.get("var")).and_then(|v| v.as_str());
            seq.push(match var_name {
                Some(v) => format!("StmtCompoundMinus_{}", v),
                None => "StmtCompoundMinus".to_string(),
            });
            if let Some(a) = args { if a.len() > 1 { visit_expr(&a[1], slices, seq); } }
        }
        "++" => {
            let var_name = args.and_then(|a| a.first())
                .and_then(|v| v.get("var")).and_then(|v| v.as_str());
            seq.push(match var_name {
                Some(v) => format!("StmtCompoundPlus_{}", v),
                None => "StmtCompoundPlus".to_string(),
            });
        }
        "--" => {
            let var_name = args.and_then(|a| a.first())
                .and_then(|v| v.get("var")).and_then(|v| v.as_str());
            seq.push(match var_name {
                Some(v) => format!("StmtCompoundMinus_{}", v),
                None => "StmtCompoundMinus".to_string(),
            });
        }
        "=" => {
            if let Some(a) = args {
                if a.first().map(|l| is_slice_lvalue(l)).unwrap_or(false) {
                    seq.push("StmtSliceAssign".into());
                } else {
                    let var_name = a.first()
                        .and_then(|v| v.get("var")).and_then(|v| v.as_str());
                    seq.push(match var_name {
                        Some(v) => format!("StmtAssign_{}", v),
                        None => "StmtAssign".to_string(),
                    });
                }
                if a.len() > 1 { visit_expr(&a[1], slices, seq); }
            }
        }
        _ => { if let Some(a) = args { for e in a { visit_expr(e, slices, seq); } } }
    }
}

fn visit_stmts(node: &serde_json::Value, slices: &[&str], seq: &mut Vec<String>) {
    if let Some(stmts) = node.as_array() {
        for s in stmts { visit_stmt(s, slices, seq); }
    }
}

fn visit_expr(node: &serde_json::Value, slices: &[&str], seq: &mut Vec<String>) {
    // Leaf nodes (variable references and literals) are NOT emitted as hints.
    // `is_ast_scored_node` excludes leaves (nodes without children), so including
    // them in the hint sequence would cause the LCP to mis-align.
    if node.get("var").is_some() || node.get("lit").is_some() {
        return;
    }

    let op = match node.get("op").and_then(|v| v.as_str()) {
        Some(o) => o,
        None => return,
    };
    let args = node["args"].as_array();
    let arity = args.map(|a| a.len()).unwrap_or(0);

    match op {
        "+" => {
            // Pointer arithmetic (a + k, a + n) is absorbed into index addressing — skip node
            let is_ptr = args.map(|a| a.iter().any(|x| is_ptr_expr(x, slices))).unwrap_or(false);
            if is_ptr {
                if let Some(a) = args { for x in a { visit_expr(x, slices, seq); } }
            } else {
                seq.push("ExprAdd".into());
                if let Some(a) = args { for x in a { visit_expr(x, slices, seq); } }
            }
        }
        "-" if arity == 2 => {
            let is_ptr = args.map(|a| a.iter().any(|x| is_ptr_expr(x, slices))).unwrap_or(false);
            if is_ptr {
                if let Some(a) = args { for x in a { visit_expr(x, slices, seq); } }
            } else {
                seq.push("ExprSub".into());
                if let Some(a) = args { for x in a { visit_expr(x, slices, seq); } }
            }
        }
        "-" if arity == 1 => {
            // Unary negation: -x → (0 - x) in Rust → hint is ExprSub
            seq.push("ExprSub".into());
            if let Some(a) = args { for x in a { visit_expr(x, slices, seq); } }
        }
        "*" if arity == 1 => {
            // Pointer dereference → slice indexing in Rust
            seq.push("ExprIndex".into());
            // The index itself isn't a child in the AST, so nothing more to visit
        }
        "*" if arity == 2 => {
            seq.push("ExprMul".into());
            if let Some(a) = args { for x in a { visit_expr(x, slices, seq); } }
        }
        "[]" => {
            seq.push("ExprIndex".into());
            // Only visit the index expression (arg[1]), not the array name (arg[0] is baked into ExprIndex_name)
            if let Some(a) = args { if a.len() > 1 { visit_expr(&a[1], slices, seq); } }
        }
        "<"  => { seq.push("ExprLt".into()); if let Some(a) = args { for x in a { visit_expr(x, slices, seq); } } }
        ">"  => { seq.push("ExprGt".into()); if let Some(a) = args { for x in a { visit_expr(x, slices, seq); } } }
        "<=" => { seq.push("ExprLe".into()); if let Some(a) = args { for x in a { visit_expr(x, slices, seq); } } }
        ">=" => { seq.push("ExprGe".into()); if let Some(a) = args { for x in a { visit_expr(x, slices, seq); } } }
        "==" => { seq.push("ExprEq".into()); if let Some(a) = args { for x in a { visit_expr(x, slices, seq); } } }
        "!=" => { seq.push("ExprNe".into()); if let Some(a) = args { for x in a { visit_expr(x, slices, seq); } } }
        "%"  => { seq.push("ExprMod".into()); if let Some(a) = args { for x in a { visit_expr(x, slices, seq); } } }
        "&&" => { seq.push("ExprAnd".into()); if let Some(a) = args { for x in a { visit_expr(x, slices, seq); } } }
        "||" => { seq.push("ExprOr".into());  if let Some(a) = args { for x in a { visit_expr(x, slices, seq); } } }
        "!"  => { seq.push("ExprNot".into()); if let Some(a) = args { for x in a { visit_expr(x, slices, seq); } } }
        "?:" => {
            seq.push("ExprIfElse".into());
            if let Some(a) = args {
                // condition (arg 0), then-branch (arg 1), else-branch (arg 2)
                for x in a { visit_expr(x, slices, seq); }
            }
        }
        _ => { if let Some(a) = args { for x in a { visit_expr(x, slices, seq); } } }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Does this expression involve a slice parameter (i.e. is it pointer arithmetic)?
fn is_ptr_expr(node: &serde_json::Value, slices: &[&str]) -> bool {
    if let Some(name) = node.get("var").and_then(|v| v.as_str()) {
        return slices.contains(&name);
    }
    if let Some(op) = node.get("op").and_then(|v| v.as_str()) {
        if matches!(op, "+" | "-") {
            if let Some(args) = node["args"].as_array() {
                return args.iter().any(|a| is_ptr_expr(a, slices));
            }
        }
    }
    false
}

/// Is this for-loop init a pointer-based initialization (let p = slice + offset)?
fn is_pointer_init(init: &serde_json::Value, slices: &[&str]) -> bool {
    if init.get("op").and_then(|v| v.as_str()) == Some("let") {
        if let Some(args) = init["args"].as_array() {
            if args.len() > 1 { return is_ptr_expr(&args[1], slices); }
        }
    }
    false
}

/// Is this lvalue a slice element (array index or pointer dereference)?
fn is_slice_lvalue(node: &serde_json::Value) -> bool {
    match node.get("op").and_then(|v| v.as_str()) {
        Some("[]") => true,
        Some("*") => node["args"].as_array().map(|a| a.len() == 1).unwrap_or(false),
        _ => false,
    }
}

// ── Local variable extraction ─────────────────────────────────────────────────

/// Walk the C++ AST and collect local variable declarations with their inferred Rust types.
/// For-loop init vars → "usize"; slice-indexed vars → slice elem type;
/// pointer-arithmetic vars → "usize"; literal-init vars → function return type (accumulator);
/// pointer-deref vars → inferred from pointer's source slice.
pub fn extract_local_vars(ast: &serde_json::Value, params: &[Param], return_type: &str) -> Vec<(String, String)> {
    let slice_params: Vec<&str> = params.iter()
        .filter(|p| is_slice_type(&p.ty))
        .map(|p| p.name.as_str())
        .collect();
    // Map slice param name → elem type string
    let slice_elem: std::collections::HashMap<&str, String> = params.iter()
        .filter(|p| is_slice_type(&p.ty))
        .filter_map(|p| crate::grammar::elem_type_of(&p.ty).map(|e| (p.name.as_str(), e)))
        .collect();
    // Use return type for literal-initialised accumulator vars (e.g. sum: u32)
    let acc_ty = if return_type == "()" || return_type.is_empty() { "i32" } else { return_type };

    let mut vars: Vec<(String, String)> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    // ptr_vars tracks which local vars are pointer-type and what they point to (elem type)
    let mut ptr_vars: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    // Seed ptr_vars with slice params themselves
    for (name, elem) in &slice_elem {
        ptr_vars.insert(name.to_string(), elem.clone());
    }

    if let Some(stmts) = ast.as_array() {
        for stmt in stmts {
            collect_lvars(stmt, &slice_params, &slice_elem, &mut ptr_vars, acc_ty, &mut vars, &mut seen, false);
        }
    }
    vars
}

fn collect_lvars(
    node: &serde_json::Value,
    slice_params: &[&str],
    slice_elem: &std::collections::HashMap<&str, String>,
    ptr_vars: &mut std::collections::HashMap<String, String>,
    acc_ty: &str,
    vars: &mut Vec<(String, String)>,
    seen: &mut std::collections::HashSet<String>,
    in_for_init: bool,
) {
    // For loop: init → usize, then visit body
    if node.get("init").is_some() {
        collect_lvars(&node["init"], slice_params, slice_elem, ptr_vars, acc_ty, vars, seen, true);
        if let Some(body) = node["body"].as_array() {
            for s in body { collect_lvars(s, slice_params, slice_elem, ptr_vars, acc_ty, vars, seen, false); }
        }
        return;
    }
    // While / if: visit body and then-branch
    if node.get("condition").is_some() {
        if let Some(body) = node["body"].as_array() {
            for s in body { collect_lvars(s, slice_params, slice_elem, ptr_vars, acc_ty, vars, seen, false); }
        }
        if let Some(then) = node["then"].as_array() {
            for s in then { collect_lvars(s, slice_params, slice_elem, ptr_vars, acc_ty, vars, seen, false); }
        }
        return;
    }
    // Try/catch: visit body
    if node.get("catch").is_some() {
        if let Some(body) = node["body"].as_array() {
            for s in body { collect_lvars(s, slice_params, slice_elem, ptr_vars, acc_ty, vars, seen, false); }
        }
        return;
    }

    let op = match node.get("op").and_then(|v| v.as_str()) { Some(o) => o, None => return };
    let args = node["args"].as_array();

    if op == "let" {
        if let Some(a) = args {
            if let Some(name) = a.get(0).and_then(|v| v.get("var")).and_then(|v| v.as_str()) {
                if !seen.contains(name) {
                    seen.insert(name.to_string());
                    let ty = if in_for_init {
                        "usize".to_string()
                    } else if let Some(rhs) = a.get(1) {
                        infer_lvar_type(rhs, slice_params, slice_elem, ptr_vars, acc_ty)
                    } else {
                        "i32".to_string()
                    };
                    // Track pointer vars for deref type inference
                    if ty == "usize" {
                        // Record what elem type this pointer points to, if derivable
                        if let Some(rhs) = a.get(1) {
                            if let Some(src_elem) = ptr_source_elem(rhs, slice_params, slice_elem) {
                                ptr_vars.insert(name.to_string(), src_elem);
                            }
                        }
                    }
                    vars.push((name.to_string(), ty));
                }
            }
        }
    }
}

/// Infer the Rust type for a local variable based on its initializer.
fn infer_lvar_type(
    rhs: &serde_json::Value,
    slice_params: &[&str],
    slice_elem: &std::collections::HashMap<&str, String>,
    ptr_vars: &std::collections::HashMap<String, String>,
    acc_ty: &str,
) -> String {
    // Slice index a[i] → elem type of a
    if rhs.get("op").and_then(|v| v.as_str()) == Some("[]") {
        if let Some(a) = rhs["args"].as_array() {
            if let Some(arr) = a.get(0).and_then(|v| v.get("var")).and_then(|v| v.as_str()) {
                if let Some(elem) = slice_elem.get(arr) { return elem.clone(); }
            }
        }
    }
    // Pointer dereference *ptr → elem type of ptr
    if rhs.get("op").and_then(|v| v.as_str()) == Some("*") {
        if let Some(a) = rhs["args"].as_array() {
            if a.len() == 1 {
                if let Some(ptr) = a[0].get("var").and_then(|v| v.as_str()) {
                    if let Some(elem) = ptr_vars.get(ptr) { return elem.clone(); }
                    if let Some(elem) = slice_elem.get(ptr) { return elem.clone(); }
                }
            }
        }
    }
    // Variable reference that is a slice param → usize (pointer)
    if let Some(var_name) = rhs.get("var").and_then(|v| v.as_str()) {
        if slice_params.contains(&var_name) { return "usize".to_string(); }
        if ptr_vars.contains_key(var_name) { return "usize".to_string(); }
    }
    // Pointer arithmetic involving a slice → usize
    if is_ptr_expr(rhs, slice_params) { return "usize".to_string(); }
    // Recursive pointer arithmetic (e.g. (a + n) - 1)
    if let Some(op) = rhs.get("op").and_then(|v| v.as_str()) {
        if matches!(op, "+" | "-") {
            if let Some(a) = rhs["args"].as_array() {
                if a.iter().any(|x| is_ptr_expr(x, slice_params) || infer_lvar_type(x, slice_params, slice_elem, ptr_vars, acc_ty) == "usize") {
                    return "usize".to_string();
                }
            }
        }
    }
    // Literal initialiser → treat as accumulator (return type)
    if rhs.get("lit").is_some() { return acc_ty.to_string(); }
    // Default
    "i32".to_string()
}

/// If a pointer initialiser expr is derived from a slice, return that slice's elem type.
fn ptr_source_elem(
    rhs: &serde_json::Value,
    slice_params: &[&str],
    slice_elem: &std::collections::HashMap<&str, String>,
) -> Option<String> {
    if let Some(name) = rhs.get("var").and_then(|v| v.as_str()) {
        if slice_params.contains(&name) { return slice_elem.get(name).cloned(); }
    }
    if let Some(a) = rhs["args"].as_array() {
        for x in a { if let Some(e) = ptr_source_elem(x, slice_params, slice_elem) { return Some(e); } }
    }
    None
}

// ── Variable ident extraction ─────────────────────────────────────────────────

/// Walk the C++ AST and collect the names of variables that appear as
/// `{"var":"x"}` nodes, then map them to the ExprIdent production names used
/// in the Rust grammar.
///
/// Scalar params → `ExprIdent_{param_index}`  (same index as in build_grammar)
/// Local vars    → `ExprIdent_local_{local_index}`
///
/// Only scalar params and local vars are included; slice params are accessed
/// via ExprIndex/ExprLen and are not tracked here.
pub fn extract_required_idents(
    ast: &serde_json::Value,
    params: &[Param],
    local_vars: &[(String, String)],
) -> Vec<String> {
    let mut used_names = std::collections::HashSet::new();
    collect_var_names(ast, &mut used_names);

    let mut idents = Vec::new();
    for (i, p) in params.iter().enumerate() {
        if is_slice_type(&p.ty) {
            // Slice params are accessed via ExprIndex_{name}; always require them.
            idents.push(format!("ExprIndex_{}", p.name));
        } else if used_names.contains(p.name.as_str()) {
            idents.push(format!("ExprIdent_{}", i));
        }
    }
    for (idx, (name, _)) in local_vars.iter().enumerate() {
        if used_names.contains(name.as_str()) {
            idents.push(format!("ExprIdent_local_{}", idx));
        }
    }
    idents
}

fn collect_var_names(val: &serde_json::Value, names: &mut std::collections::HashSet<String>) {
    if let Some(v) = val.get("var").and_then(|v| v.as_str()) {
        names.insert(v.to_string());
        return;
    }
    if let Some(obj) = val.as_object() {
        for v in obj.values() { collect_var_names(v, names); }
    } else if let Some(arr) = val.as_array() {
        for v in arr { collect_var_names(v, names); }
    }
}

// ── Other loaders ─────────────────────────────────────────────────────────────

pub fn load_symbols(path: &Path) -> Result<Vec<String>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    Ok(text
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| l.to_string())
        .collect())
}

pub fn parse_env(test_case: &TestCase, params: &[Param]) -> crate::eval::Env {
    params
        .iter()
        .zip(test_case.inputs.iter())
        .map(|(param, input)| {
            let val = match param.ty.as_str() {
                "bool" => Value::Bool(input == "true"),
                "&i32" => Value::Int(input.parse::<i32>().expect("invalid &i32 input")),
                "Option<&i32>" => {
                    if input == "null" || input == "None" {
                        Value::Opt(None)
                    } else {
                        Value::Opt(Some(input.parse::<i32>().expect("invalid Option<&i32> input")))
                    }
                }
                "&[i32]" => {
                    let v: Vec<i32> = serde_json::from_str(input).expect("invalid &[i32] input");
                    Value::SliceI32(v)
                }
                "&mut [i32]" => {
                    let v: Vec<i32> = serde_json::from_str(input).expect("invalid &mut [i32] input");
                    Value::SliceMutI32(v)
                }
                "&[u8]" => {
                    let v: Vec<u8> = serde_json::from_str(input).expect("invalid &[u8] input");
                    Value::SliceU8(v)
                }
                _ => Value::Int(input.parse::<i32>().expect("invalid i32 input")),
            };
            (param.name.clone(), val)
        })
        .collect()
}
