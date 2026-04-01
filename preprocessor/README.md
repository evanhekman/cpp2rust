# cpp2rust preprocessor

This preprocessor parses small C++ function files into a JSON AST and optionally maps C++ types to Rust-leaning types.

## Binaries

- `cpp2json_cpp`: Parse a single C++ file and emit JSON with C++ types.
- `map_cpp_json_to_rust_json`: Map C++ types in JSON to Rust types using pointer tags.

## Quick Start

```bash
# From cpp2rust/
cargo run -p cpp_preprocessor --bin cpp2json_cpp -- preprocessor/test_inputs/add_one.cpp --out /tmp/add_one_cpp.json
cargo run -p cpp_preprocessor --bin map_cpp_json_to_rust_json -- /tmp/add_one_cpp.json --out /tmp/add_one_rust.json
```

## Pointer Tag Mapping

Pointer tags are emitted by `cpp2json_cpp` and consumed by `map_cpp_json_to_rust_json`.

- `ptr_used_in_arithmetic` -> slice (array view)
  - Example: `int* p` used as `p + i` or `p[i]` maps to `&[i32]` (or `&[T]`).
- `ptr_associated_with_new_delete` -> `Box<T>`
  - Example: `int* p = new int(5)` maps to `Box<i32>`; `delete p` is dropped in Rust.
- `ptr_null_compared_or_assigned` / `ptr_nullifiable` -> `Option<...>`
  - Example: `int* p` compared to `nullptr` maps to `Option<&i32>` or `Option<Box<i32>>`.

The mapper applies these rules to function params, return types, and local `let` bindings when the JSON node has a `type` field.

## JSON Output Format

Example output from `cpp2json_cpp`:

```json
{
  "name": "dot",
  "params": [
    {
      "name": "a",
      "type": "uint8_t*",
      "ptr_nullifiable": false,
      "ptr_used_in_arithmetic": true,
      "ptr_associated_with_new_delete": false
    }
  ],
  "return_type": "int",
  "ast": [
    {
      "op": "let",
      "args": [
        { "var": "sum", "type": "int" },
        { "lit": "0" }
      ]
    }
  ]
}
```

Mapped output from `map_cpp_json_to_rust_json`:

```json
{
  "name": "dot",
  "params": [
    {
      "name": "a",
      "type": "&[u8]",
      "ptr_nullifiable": false,
      "ptr_used_in_arithmetic": true,
      "ptr_associated_with_new_delete": false
    }
  ],
  "return_type": "i32",
  "ast": [
    {
      "op": "let",
      "args": [
        { "var": "sum", "type": "i32" },
        { "lit": "0" }
      ]
    }
  ]
}
```

## Extending Tag Heuristics (Regex Rules)

Pointer tags are inferred in [preprocessor/src/bin/cpp2json_cpp.rs](preprocessor/src/bin/cpp2json_cpp.rs) by regex helpers:

- `has_nullptr_usage`: detects `nullptr`/`NULL` comparisons and assignments.
- `has_pointer_arithmetic_usage`: detects `p++`, `p += k`, `p + k`, and `p[i]`.
- `has_new_delete_usage`: detects `p = new T(...)` and `delete p`.

To extend the heuristics, update the regex pattern arrays in those functions and add new test inputs under `preprocessor/test_inputs/`.

## Test Inputs

The following files are useful for validating tag behavior:

- `preprocessor/test_inputs/new_delete_ptr.cpp`
- `preprocessor/test_inputs/nullable_ptr.cpp`
- `preprocessor/test_inputs/assigned_null.cpp`
- `preprocessor/test_inputs/pointer_init_tag_demo.cpp`

## Notes

- The JSON AST is intentionally minimal and geared toward later synthesis stages.
- Type mapping is best-effort and conservative; when no tags apply, pointers map to `&T`.
