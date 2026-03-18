//! Extract variables, functions, operators, and literals from a Tree-sitter C++ tree.
//!
//! This is **syntactic** only (no type checking); types are best-effort spellings from the parse tree.

use tree_sitter::{Node, Tree};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableInfo {
    pub name: String,
    pub type_spelling: String,
    pub role: VariableRole,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionInfo {
    pub name: String,
    pub return_type: String,
    pub parameters: Vec<(String, String)>,
    /// `true` for a body (`function_definition`), `false` for a declaration-only prototype.
    pub is_definition: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperatorKind {
    Binary,
    Unary,
    Update,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorInfo {
    pub spelling: String,
    pub kind: OperatorKind,
}

#[derive(Debug, Clone, Default)]
pub struct Extracted {
    pub variables: Vec<VariableInfo>,
    pub functions: Vec<FunctionInfo>,
    pub operators: Vec<OperatorInfo>,
    pub literals: Vec<String>,
}

/// Walk the tree once and fill all lists (order roughly follows source pre-order).
pub fn extract_all(source: &str, tree: &Tree) -> Extracted {
    let bytes = source.as_bytes();
    let mut out = Extracted::default();
    walk(tree.root_node(), None, bytes, &mut out);
    out
}

// ---------------------------------------------------------------------------
// Walk
// ---------------------------------------------------------------------------

fn walk(node: Node<'_>, parent: Option<Node<'_>>, source: &[u8], out: &mut Extracted) {
    match node.kind() {
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
