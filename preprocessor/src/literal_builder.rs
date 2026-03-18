//! Literal extraction (delegates to [`crate::extract`] so one walk covers everything).

use tree_sitter::Tree;

/// Collect literal spellings in pre-order (as they appear in source).
pub fn build_literal_values(source: &str, tree: &Tree) -> Vec<String> {
    crate::extract::extract_all(source, tree).literals
}

/// print the literal values
pub fn print_literal_values(literals: &[String]) {
    if literals.is_empty() {
        println!("  (no literal nodes in this tree)");
        return;
    }
    println!("  count: {}", literals.len());
    for (i, v) in literals.iter().enumerate() {
        println!("  [{}] {}", i, v);
    }
}
