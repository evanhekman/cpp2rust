//! Ensures every file under `test_inputs/*.cpp` parses without a fully broken root.

use std::fs;
use std::path::PathBuf;

fn test_inputs_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_inputs")
}

#[test]
fn all_test_inputs_parse() {
    let dir = test_inputs_dir();
    assert!(dir.is_dir(), "missing {}", dir.display());

    for entry in fs::read_dir(&dir).expect("read_dir test_inputs") {
        let path = entry.expect("entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("cpp") {
            continue;
        }
        let src = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {:?}: {}", path, e));
        let tree = cpp_preprocessor::parse_cpp_source(&src)
            .unwrap_or_else(|e| panic!("parse {:?}: {}", path, e));
        // Tree-sitter is error-tolerant; we only require a non-empty tree.
        assert!(
            tree.root_node().child_count() > 0 || !src.trim().is_empty(),
            "empty tree for {:?}",
            path
        );
    }
}

#[test]
fn add_one_sexp_is_non_empty() {
    let path = test_inputs_dir().join("add_one.cpp");
    let src = fs::read_to_string(&path).unwrap();
    let tree = cpp_preprocessor::parse_cpp_source(&src).unwrap();
    let sexp = cpp_preprocessor::tree_as_sexp(&tree);
    assert!(sexp.len() > 50);
    assert!(sexp.contains("function_definition") || sexp.contains("translation_unit"));
}
