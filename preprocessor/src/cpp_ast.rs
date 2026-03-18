//! Parse C++ source with Tree-sitter into a syntax tree.

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use tree_sitter::Parser;
use tree_sitter::Tree;

/// Parse C++ from a string.
pub fn parse_cpp_source(source: &str) -> Result<Tree> {
    let mut parser = Parser::new();
    let language = tree_sitter_cpp::LANGUAGE;
    parser
        .set_language(&language.into())
        .context("failed to load C++ grammar")?;
    parser
        .parse(source, None)
        .context("parse returned None (usually OOM)")
}

/// Read a file and parse it as C++.
pub fn parse_cpp_file(path: impl AsRef<Path>) -> Result<Tree> {
    let path = path.as_ref();
    let source = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    parse_cpp_source(&source)
}

/// Full syntax tree as an S-expression (good for debugging / diffing).
pub fn tree_as_sexp(tree: &Tree) -> String {
    tree.root_node().to_sexp()
}
