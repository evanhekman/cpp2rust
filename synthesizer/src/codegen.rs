use crate::ast::{Node, Child};
use crate::grammar::{Grammar, find_production};

pub fn render(node: &Node, grammar: &Grammar) -> Result<String, String> {
    let prod = find_production(&node.kind, grammar)
        .ok_or_else(|| format!("No production for kind={}", node.kind))?;
    if prod.children_spec.is_empty() {
        return Ok(prod.rust_template.clone());
    }
    let parts: Result<Vec<String>, String> = node.children.iter().map(|c| match c {
        Child::Node(n) => render(n, grammar),
        Child::Hole(nt) => Ok(format!("???:{}", nt)),
    }).collect();
    Ok(format_template(&prod.rust_template, &parts?))
}

fn format_template(tmpl: &str, args: &[String]) -> String {
    let mut out = tmpl.to_string();
    // Replace in reverse index order to avoid {1} matching inside {10}
    for i in (0..args.len()).rev() {
        out = out.replace(&format!("{{{}}}", i), &args[i]);
    }
    out
}
