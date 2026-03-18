//! Extract variables, functions, operators, and literals from a Tree-sitter C++ tree.
//!
//! This is **syntactic** only (no type checking); types are best-effort spellings from the parse tree.

use anyhow::Context;
use serde::Serialize;
use std::fs;
use std::path::Path;
use tree_sitter::{Node, Tree};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlFlowKind {
    If,
    Else,
    Switch,
    Case,
    Default,
    For,
    While,
    DoWhile,
    Break,
    Continue,
    Return,
    Goto,
    Throw,
    Try,
    Catch,
    CoReturn,
    CoYield,
    CoAwait,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ControlFlowInfo {
    pub kind: ControlFlowKind,
    /// Raw Tree-sitter node kind (e.g. `"if_statement"`).
    pub node_kind: String,
    /// Best-effort "header" / key expression for the construct (e.g. if-condition, loop-condition).
    /// Empty when not applicable.
    pub header: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VariableRole {
    /// File-scope or namespace-scope declaration.
    Global,
    /// Inside a function body (or similar block).
    Local,
    /// Function parameter.
    Parameter,
    /// Struct / class / union field.
    Field,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VariableInfo {
    pub name: String,
    pub type_spelling: String,
    pub role: VariableRole,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FunctionInfo {
    pub name: String,
    pub return_type: String,
    /// Each entry is `[type_spelling, parameter_name]` (name may be empty).
    pub parameters: Vec<(String, String)>,
    /// `true` for a body (`function_definition`), `false` for a declaration-only prototype.
    pub is_definition: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OperatorKind {
    Binary,
    Unary,
    Update,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OperatorInfo {
    pub spelling: String,
    pub kind: OperatorKind,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Extracted {
    pub variables: Vec<VariableInfo>,
    pub functions: Vec<FunctionInfo>,
    pub operators: Vec<OperatorInfo>,
    pub literals: Vec<String>,
    pub control_flow: Vec<ControlFlowInfo>,
}

/// Walk the tree once and fill all lists (order roughly follows source pre-order).
pub fn extract_all(source: &str, tree: &Tree) -> Extracted {
    let bytes = source.as_bytes();
    let mut out = Extracted::default();
    walk(tree.root_node(), None, bytes, &mut out);
    out
}

/// Pretty-printed JSON for one extraction.
pub fn extracted_to_json_pretty(ex: &Extracted) -> anyhow::Result<String> {
    serde_json::to_string_pretty(ex).context("serialize Extracted")
}

/// Write one `Extracted` to a JSON file (pretty).
pub fn write_extracted_json(path: impl AsRef<Path>, ex: &Extracted) -> anyhow::Result<()> {
    let s = extracted_to_json_pretty(ex)?;
    if let Some(parent) = path.as_ref().parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path.as_ref(), s).with_context(|| format!("write {}", path.as_ref().display()))
}

/// Batch export: multiple files in one JSON document.
#[derive(Debug, Serialize)]
pub struct ExtractedBatch {
    pub files: Vec<ExtractedFileRecord>,
}

#[derive(Debug, Serialize)]
pub struct ExtractedFileRecord {
    pub path: String,
    pub root_has_error: bool,
    pub extracted: Extracted,
}

pub fn batch_to_json_pretty(batch: &ExtractedBatch) -> anyhow::Result<String> {
    serde_json::to_string_pretty(batch).context("serialize batch")
}

pub fn write_batch_json(path: impl AsRef<Path>, batch: &ExtractedBatch) -> anyhow::Result<()> {
    let s = batch_to_json_pretty(batch)?;
    if let Some(parent) = path.as_ref().parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path.as_ref(), s).with_context(|| format!("write {}", path.as_ref().display()))
}

// ---------------------------------------------------------------------------
// Walk
// ---------------------------------------------------------------------------

fn walk(node: Node<'_>, parent: Option<Node<'_>>, source: &[u8], out: &mut Extracted) {
    match node.kind() {
        // ── Control flow / logic blocks ────────────────────────────────────
        "if_statement" => {
            out.control_flow.push(ControlFlowInfo {
                kind: ControlFlowKind::If,
                node_kind: node.kind().to_string(),
                header: header_from_field(node, source, "condition"),
            });
            // Tree-sitter-cpp represents `else` as an `else_clause` node.
        }
        "else_clause" => {
            out.control_flow.push(ControlFlowInfo {
                kind: ControlFlowKind::Else,
                node_kind: node.kind().to_string(),
                header: String::new(),
            });
        }
        "switch_statement" => {
            out.control_flow.push(ControlFlowInfo {
                kind: ControlFlowKind::Switch,
                node_kind: node.kind().to_string(),
                header: header_from_field(node, source, "condition"),
            });
        }
        "case_statement" => {
            out.control_flow.push(ControlFlowInfo {
                kind: ControlFlowKind::Case,
                node_kind: node.kind().to_string(),
                header: header_from_field(node, source, "value"),
            });
        }
        "default_statement" => {
            out.control_flow.push(ControlFlowInfo {
                kind: ControlFlowKind::Default,
                node_kind: node.kind().to_string(),
                header: String::new(),
            });
        }
        "for_statement" => {
            out.control_flow.push(ControlFlowInfo {
                kind: ControlFlowKind::For,
                node_kind: node.kind().to_string(),
                header: header_from_field(node, source, "condition"),
            });
        }
        "while_statement" => {
            out.control_flow.push(ControlFlowInfo {
                kind: ControlFlowKind::While,
                node_kind: node.kind().to_string(),
                header: header_from_field(node, source, "condition"),
            });
        }
        "do_statement" => {
            out.control_flow.push(ControlFlowInfo {
                kind: ControlFlowKind::DoWhile,
                node_kind: node.kind().to_string(),
                header: header_from_field(node, source, "condition"),
            });
        }
        "break_statement" => {
            out.control_flow.push(ControlFlowInfo {
                kind: ControlFlowKind::Break,
                node_kind: node.kind().to_string(),
                header: String::new(),
            });
        }
        "continue_statement" => {
            out.control_flow.push(ControlFlowInfo {
                kind: ControlFlowKind::Continue,
                node_kind: node.kind().to_string(),
                header: String::new(),
            });
        }
        "return_statement" => {
            out.control_flow.push(ControlFlowInfo {
                kind: ControlFlowKind::Return,
                node_kind: node.kind().to_string(),
                header: header_from_field(node, source, "argument"),
            });
        }
        "goto_statement" => {
            out.control_flow.push(ControlFlowInfo {
                kind: ControlFlowKind::Goto,
                node_kind: node.kind().to_string(),
                header: header_from_field(node, source, "label"),
            });
        }
        "throw_statement" => {
            out.control_flow.push(ControlFlowInfo {
                kind: ControlFlowKind::Throw,
                node_kind: node.kind().to_string(),
                header: header_from_field(node, source, "argument"),
            });
        }
        "try_statement" => {
            out.control_flow.push(ControlFlowInfo {
                kind: ControlFlowKind::Try,
                node_kind: node.kind().to_string(),
                header: String::new(),
            });
        }
        "catch_clause" => {
            // Some grammars have a `parameter`/`declarator` inside; keep a small header for inspection.
            out.control_flow.push(ControlFlowInfo {
                kind: ControlFlowKind::Catch,
                node_kind: node.kind().to_string(),
                header: first_named_child_text(node, source),
            });
        }
        "co_return_statement" => {
            out.control_flow.push(ControlFlowInfo {
                kind: ControlFlowKind::CoReturn,
                node_kind: node.kind().to_string(),
                header: header_from_field(node, source, "argument"),
            });
        }
        "co_yield_statement" => {
            out.control_flow.push(ControlFlowInfo {
                kind: ControlFlowKind::CoYield,
                node_kind: node.kind().to_string(),
                header: header_from_field(node, source, "argument"),
            });
        }
        "co_await_expression" => {
            out.control_flow.push(ControlFlowInfo {
                kind: ControlFlowKind::CoAwait,
                node_kind: node.kind().to_string(),
                header: header_from_field(node, source, "argument"),
            });
        }

        "function_definition" => {
            if let Some(fi) = extract_function_definition(node, source) {
                out.functions.push(fi);
            }
            // Parameters are collected when we visit `parameter_declaration` under the tree.
        }
        "declaration" => {
            let ty = type_from_declaration_like(node, source);
            // Variables: init_declarator(s)
            let mut any_init = false;
            for i in 0u32..(node.named_child_count() as u32) {
                let ch = match node.named_child(i) {
                    Some(c) => c,
                    None => continue,
                };
                if ch.kind() != "init_declarator" {
                    continue;
                }
                any_init = true;
                if let Some(d) = ch.child_by_field_name("declarator") {
                    if let Some(name) = declarator_to_var_name(d, source) {
                        let role = variable_role_for_declaration(parent);
                        out.variables.push(VariableInfo {
                            name,
                            type_spelling: ty.clone(),
                            role,
                        });
                    }
                }
            }
            // Function prototype (no body)
            if !any_init {
                if let Some(decl) = node.child_by_field_name("declarator") {
                    if decl.kind() == "function_declarator" {
                        if let Some(name) = function_declarator_name(decl, source) {
                            let params = parameters_from_function_declarator(decl, source);
                            out.functions.push(FunctionInfo {
                                name,
                                return_type: ty,
                                parameters: params,
                                is_definition: false,
                            });
                        }
                    }
                }
            }
        }
        "parameter_declaration" => {
            // Usually handled under function_definition; also catch standalone edge cases.
            let ty = node
                .child_by_field_name("type")
                .map(|t| node_text(t, source))
                .unwrap_or_default();
            if let Some(d) = node.child_by_field_name("declarator") {
                if let Some(name) = declarator_to_var_name(d, source) {
                    out.variables.push(VariableInfo {
                        name,
                        type_spelling: ty,
                        role: VariableRole::Parameter,
                    });
                }
            }
        }
        "field_declaration" => {
            let ty = node
                .child_by_field_name("type")
                .map(|t| node_text(t, source))
                .unwrap_or_default();
            if let Some(d) = node.child_by_field_name("declarator") {
                if let Some(name) = declarator_to_var_name(d, source) {
                    out.variables.push(VariableInfo {
                        name,
                        type_spelling: ty,
                        role: VariableRole::Field,
                    });
                }
            }
        }
        "binary_expression" => {
            if let Some(op_node) = node.child_by_field_name("operator") {
                let t = node_text(op_node, source);
                if !t.is_empty() {
                    out.operators.push(OperatorInfo {
                        spelling: t,
                        kind: OperatorKind::Binary,
                    });
                }
            }
        }
        "unary_expression" => {
            if let Some(op_node) = node.child_by_field_name("operator") {
                let t = node_text(op_node, source);
                if !t.is_empty() {
                    out.operators.push(OperatorInfo {
                        spelling: t,
                        kind: OperatorKind::Unary,
                    });
                }
            }
        }
        "update_expression" => {
            if let Some(op_node) = node.child_by_field_name("operator") {
                let t = node_text(op_node, source);
                if !t.is_empty() {
                    out.operators.push(OperatorInfo {
                        spelling: t,
                        kind: OperatorKind::Update,
                    });
                }
            }
        }
        "number_literal"
        | "string_literal"
        | "char_literal"
        | "raw_string_literal"
        | "user_defined_literal"
        | "true"
        | "false" => {
            if let Ok(t) = node.utf8_text(source) {
                out.literals.push(t.to_string());
            }
        }
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk(child, Some(node), source, out);
    }
}

fn variable_role_for_declaration(parent: Option<Node<'_>>) -> VariableRole {
    match parent.map(|p| p.kind()) {
        Some("translation_unit") | Some("namespace_definition") => VariableRole::Global,
        _ => VariableRole::Local,
    }
}

fn extract_function_definition(node: Node<'_>, source: &[u8]) -> Option<FunctionInfo> {
    let ret = node
        .child_by_field_name("type")
        .map(|t| node_text(t, source))
        .unwrap_or_default();
    let fd = node.child_by_field_name("declarator")?;
    if fd.kind() != "function_declarator" {
        return None;
    }
    let name = function_declarator_name(fd, source)?;
    let parameters = parameters_from_function_declarator(fd, source);
    Some(FunctionInfo {
        name,
        return_type: ret,
        parameters,
        is_definition: true,
    })
}

fn type_from_declaration_like(node: Node<'_>, source: &[u8]) -> String {
    node.child_by_field_name("type")
        .map(|t| node_text(t, source))
        .unwrap_or_default()
}

fn function_declarator_name(fd: Node<'_>, source: &[u8]) -> Option<String> {
    let d = fd.child_by_field_name("declarator")?;
    Some(decl_name_for_function_decl(d, source))
}

fn decl_name_for_function_decl(n: Node<'_>, source: &[u8]) -> String {
    node_text(n, source)
}

fn parameters_from_function_declarator(fd: Node<'_>, source: &[u8]) -> Vec<(String, String)> {
    let Some(pl) = fd.child_by_field_name("parameters") else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for i in 0u32..(pl.named_child_count() as u32) {
        let Some(ch) = pl.named_child(i) else {
            continue;
        };
        if ch.kind() != "parameter_declaration" {
            continue;
        }
        let ty = ch
            .child_by_field_name("type")
            .map(|t| node_text(t, source))
            .unwrap_or_default();
        let name = ch
            .child_by_field_name("declarator")
            .and_then(|d| declarator_to_var_name(d, source))
            .unwrap_or_default();
        out.push((ty, name));
    }
    out
}

fn declarator_to_var_name(node: Node<'_>, source: &[u8]) -> Option<String> {
    match node.kind() {
        "identifier" | "field_identifier" => {
            let s = node_text(node, source);
            if s.is_empty() {
                None
            } else {
                Some(s)
            }
        }
        "pointer_declarator" | "reference_declarator" => node
            .child_by_field_name("declarator")
            .and_then(|d| declarator_to_var_name(d, source)),
        "array_declarator" => node
            .child_by_field_name("declarator")
            .and_then(|d| declarator_to_var_name(d, source)),
        "function_declarator" => node
            .child_by_field_name("declarator")
            .and_then(|d| declarator_to_var_name(d, source)),
        "parenthesized_declarator" => node
            .named_child(0)
            .and_then(|d| declarator_to_var_name(d, source)),
        _ => None,
    }
}

fn node_text(n: Node<'_>, source: &[u8]) -> String {
    n.utf8_text(source).unwrap_or("").trim().to_string()
}

fn header_from_field(node: Node<'_>, source: &[u8], field: &str) -> String {
    node.child_by_field_name(field)
        .map(|n| node_text(n, source))
        .unwrap_or_default()
}

fn first_named_child_text(node: Node<'_>, source: &[u8]) -> String {
    node.named_child(0)
        .map(|n| node_text(n, source))
        .unwrap_or_default()
}
