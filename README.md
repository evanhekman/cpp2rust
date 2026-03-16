# system overview
The system contains three components: a preprocessor, synthesizer, and validator. The preprocessor runs static analysis on the C++ source code to find key structural components, which are passed to the synthesizer. The synthesizer produces candidate rust program(s) using heuristic search, which are passed to the validator. The validator uses the pre/post conditions to try and prove the rust is correct, returning to the synthesizer if unable to do so. Given raw C++ and a set of pre/post conditions, the system should either return fully validated rust code or an error.
```
           ┌───────────────┐    ┌─────────────┐
raw C++ -> │ preprocessing │ -> │ synthesizer │ -> rust candidate(s)
           └───────────────┘    └─────────────┘
                
rust candidate ------> ┌───────────┐ -> (1) validated rust (success) 
                       │ validator │ -> (2) counterexample (try again)
pre/post conditions -> └───────────┘ -> (3) fail to validate (try again)
```

## Preprocessor
### Purpose
Runs static analysis on the C++, finding things like unnecessary parameters or internal try/catch blocks. These are structural aspects of the C++ that must be changed to produce acceptable Rust code. Specific things the preprocessor needs to look for:
  - self-contained throw/catch blocks that are turned into early returns
  - pointer-length params (encoded in rust fat pointers directly)
  - read-only or write-through pointers (whether rust needs `mut`)
  - pointer arithmetic (converted to index-based loop)

The preprocessor is responsible for providing three things to the synthesizer:
  - a rust function signature (might not be an exact match for the original C++ function)
  - an AST-like structure containing the relevant information about the C++ (used to guide heuristic search during synthesis)
  - a set of input/output examples on the C++, translated to equivalent rust

### Testing
Preprocessor should be able to take `cpp/` inputs and produce `processed/` outputs.

## Synthesizer
### Purpose
Synthesizer is very similar to EECS498 A3, consisting of:
  - A Rust grammar and production rules
  - Evaluation queue and search algorithm
  - Heuristics to speed up the search

### Testing
Synthesizer should be able to take `processed/` inputs and produce `rust/` outputs.

## Validator
### Purpose
The validator takes a rust candidate program and a set of pre/post conditions, then tries to create a verus proof to prove correctness, yielding one of three results:
1. Counterexample (not a solution, synthesize new program)
2. Indeterminate (unknown, synthesize new program)
3. Success (fully validated rust solution)

### Testing
Validator should be able to take `rust/` and `prepost/` inputs and produce `validated/` outputs.

The orchestrator file should live in root and call `python validator/validate.py --input [INPUT FILE] --output [OUTPUT FILE]`
Alternatively, you could import and call the run_validator function.

validate.py returns a tuple containing the return code (not really important) and the verification success (true if verified, false if not) 


# Setup
## Using rust
  - to get started, [install rust](https://rust-lang.org/tools/install/) and run `cd synthesizer && cargo build --release && ./target/release/bench` to make sure you're up to speed
  - `cargo build` to build, add `--release` flag for optimizations (run from synthesizer/)
  - `cargo check` to see if you have viable code without waiting for compile
  - `./target/debug/executable` or `./target/release/executable` to run

# Benchmarks
## `benchmark0`
All 4 test cases provide an opportunity for the preprocessor to flag unnecessary parameters and condense the function signature. Each test case is intended to test a different aspect of at least one component:
- `dot_product`
  - overflow safety -> error on overflow
- `exception`
  - internal throw/catch -> early return
- `max_even_indexed`
  - pointer arithmetic -> part of loop
- `reverse`
  - in-place element mutation -> mutable slice

# Verus
- Setup scripts can be found in scripts/.
- setup scripts in scripts/
- check verus/ code with `./verus/verus --crate-type lib benchmark0/validated/testcase.rs`

### Rules
- Recommended to use `clippy` as linter for rust (`cargo clippy --fix` to apply formatting), comes preinstalled
- Work on branches, PRs when applicable
- No AI-generated markdown files
- Use the .gitignore

### TODO
- [ ] Refactor things into an actual preprocessor (cpp2json.rs, etc.)
- [ ] Build out validator
- [ ] Build out full pipeline
- [ ] Expand synthesizer to handle new dataset
- [ ] Determine preprocessing artifacts to define inputs for synthesizer
