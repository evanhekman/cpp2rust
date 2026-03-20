# system overview
The system contains three components: a preprocessor, synthesizer, and validator. The preprocessor runs static analysis on the C++ source code to find key structural components, which are passed to the synthesizer. The synthesizer produces candidate rust program(s) using heuristic search, which are passed to the validator. The validator uses the pre/post conditions to try and prove the rust is correct, returning to the synthesizer if unable to do so. Given raw C++ and a set of pre/post conditions, the system should either return fully validated rust code or an error.
```
           ┌──────────────┐    
raw C++ -> │ preprocessor │ -> intermediate AST + info
           └──────────────┘    

                           ┌─────────────┐
intermediate AST + info -> │ synthesizer │ -> rust -> rust candidate
                           └─────────────┘

                  ┌───────────┐ -> (1) validated rust (success) 
rust candidate -> │ validator │
                  └───────────┘ -> (2) fail to validate (re-synthesize)
```

## Preprocessor
Runs static analysis on the C++, finding things like unnecessary parameters or internal try/catch blocks. These are structural aspects of the C++ that must be changed to produce acceptable Rust code. There are *three things* the preprocessor needs to scan for that will impact the new rust function signature: 
  - Pointer-length param
    - requires a T* param
    - requires an int param
    - requires the int param is readonly
    - requires the int param is only ever used as a bound check against the T* param
  - Mutable pointer
    - pointer is written to
  - Nullable pointer
    - pointer is null-checked at some point
  - (if our dataset included functions that could exit with error, we would need to scan for that as well)

Once scanned, the preprocessor should be able to generate the new Rust function signature using the C++ -> Rust keyword mappings. The preprocessor then provides two things to the synthesizer:
  - the new rust function signature
  - the AST for the C++ code (whitespace and punctuation filtered out)

Preprocessor should be able to take `cpp/` inputs and produce `processed/` outputs.

## Synthesizer
Synthesizer is very similar to EECS498 A3, consisting of:
  - A Rust grammar and production rules
  - Evaluation queue and search algorithm
  - Heuristics to speed up the search
Note that the synthesizer assumes the Rust function signature is correct and that no imports are required. The synthesizer also owns the test case generation process so it is easy to generate more test cases if needed.

Synthesizer should be able to take `processed/` inputs and produce `rust/` outputs.

## Validator
The validator takes a rust candidate program and a set of pre/post conditions, then tries to create a verus proof to prove correctness, yielding one of three results:
1. Counterexample (not a solution, synthesize new program)
2. Indeterminate (unknown, synthesize new program)
3. Success (fully validated rust solution)

Validator should be able to take `rust/` and `prepost/` inputs and produce `validated/` outputs.


# Setup
## Using rust
  - to get started, [install rust](https://rust-lang.org/tools/install/) and run `cd synthesizer && cargo build --release && ./target/release/bench` to make sure you're up to speed
  - `cargo build` to build, add `--release` flag for optimizations (run from synthesizer/)
  - `cargo check` to see if you have viable code without waiting for compile
  - `./target/debug/executable` or `./target/release/executable` to run
## Using Verus
  - executable only (suitable for manual verification)
  - from `scripts`, use `setup_verus_unix.sh` or `setup_verus_windows.ps1`
  - executable will be at `verus/verus`
  - check code by running `./verus/verus --crate-type lib benchmark0/validated/testcase.rs`
## Using AutoVerus (verus-proof-synthesis)
  - clone autoverus `git clone https://github.com/microsoft/verus-proof-synthesis`
  - `cd verus-proof-synthesis`
  - `git clone https://github.com/verus-lang/verus.git`
  - set up pinned verus build
    - `cd verus`
    - `git checkout 33269ac6a0ea33a08109eefe5016c1fdd0ce9fbd` if you want Verus-Bench (default)
    - `git checkout ddc66116aa7a844a9e19cc50922fe85c84b8b4a5` if you want VeruSAGE-Bench
    - `./tools/get-z3.sh && source tools/activate`
    - `vargo build --release`
    - executable will be at `verus/source/target-verus/release/verus`
    - note: this will require that you have the correct rust toolchain (1.76.0) set in PATH
  - configure `autoverus/config.json`, particularly
    - the `verus` executable path
    - the OPENAI_KEY
    - `aoai_generation_model` and `aoai_debug_model`
  - *then* run with `python main.py --input input_filepath --output output_filepath`
## Using Justfile
  - `just verus FILE` for verifying files
  - `just old-verus FILE` to use pinned build inside autoverus
  - `just autoverus INPUT OUTPUT` to use autoverus

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

### Style
  - Recommended to use `clippy` as linter for rust (`cargo clippy --fix` to apply formatting), comes preinstalled
  - Work on branches, PRs when applicable
  - No AI-generated markdown files
  - Use the .gitignore
