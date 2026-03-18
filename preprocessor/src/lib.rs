//! C++ preprocessor: Tree-sitter parse trees for downstream synthesis.

pub mod cpp_ast;
pub mod extract;

pub use cpp_ast::{parse_cpp_file, parse_cpp_source, tree_as_sexp};
pub use extract::{
    batch_to_json_pretty, build_literal_values, extract_all, extracted_to_json_pretty, print_literal_values,
    write_batch_json, write_extracted_json, ControlFlowInfo, ControlFlowKind, Extracted, ExtractedBatch,
    ExtractedFileRecord, FunctionInfo, OperatorInfo, OperatorKind, VariableInfo, VariableRole,
};
