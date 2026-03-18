//! C++ preprocessor: Tree-sitter parse trees for downstream synthesis.

pub mod cpp_ast;
pub mod extract;
pub mod literal_builder;

pub use cpp_ast::{parse_cpp_file, parse_cpp_source, tree_as_sexp};
pub use extract::{extract_all, Extracted, FunctionInfo, OperatorInfo, OperatorKind, VariableInfo, VariableRole};
