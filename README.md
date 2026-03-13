### system overview
The system contains three components:
- the preprocessor
  - this does static analysis on the c++, finding things like unnecessary parameters or internal try/catch blocks
  - this provides three key things to the synthesizer:
    - a rust function signature (might not be an exact match for the original c++ function)
    - an AST-like structure containing the relevant information about the c++ (used to guide heuristic search during synthesis)
    - a set of input/output examples on the c++, translated to equivalent rust
- the synthesizer
  - using the specified rust grammar, generates candidates until it finds one that passes all tests
  - heuristic search to find good candidates, evaluated by using the input/output examples
- the validator
  - takes a rust candidate program and a set of pre/post conditions
  - creates a verus proof to see if the program satisfies the conditions, yielding one of three results:
    - counterexample (not a solution, synthesize new program)
    - indeterminate (unknown, synthesize new program)
    - success (fully validated rust solution)

                ┌---------------┐    ┌-------------┐
raw c++ code -> | preprocessing | -> | synthesizer | -> rust candidate program(s)
                └---------------┘    └-------------┘
                
rust candidate programs -> ┌-----------┐ -> validated rust (success) 
                           | validator |
pre/post conditions -----> └-----------┘ -> counterexample or undetermined (back to synthesizer)



- synthesizer contains all synthesizer code and tests
  - dataset/ contains 10 simple examples with depth < 8
  - code is broken down inside synthesizer/
- using rust
  - to get started, [install rust](https://rust-lang.org/tools/install/) and run `cd synthesizer && cargo build --release && ./target/release/bench` to make sure you're up to speed
  - `cargo build` to build, add `--release` flag for optimizations (run from synthesizer/)
  - `cargo check` to see if you have viable code without waiting for compile
  - `./target/debug/executable` or `./target/release/executable` to run
- datasets
  - synthesizer/dataset0 was used to test core synthesizer functionality (no c++, no heuristics, pure synthesis)
    - very basic snippets, less than 8 nodes in the AST
    - synthesizer does not succeed on all of them which is okay (hard problem when there is no heuristic to guide the search)
  - synthesizer/dataset1 is basic c++ files to test using similarity heuristics to generate the rust
  - benchmark0 is the first benchmark, and tests all stages (preprocessing, synthesis, validation)
    - cpp folder with the raw cpp files
      - dot_product
        - unnecessary pointer length param, overflow safety, index-based loop
      - exception
        - internal throw/catch as early return, unnecessary pointer length param
      - max_even_indexed
        - index-based loop, pointer arithmetic
      - reverse
        - unnecessary pointer length param, in-place mutation
    - rust folder with reasonable rust implementations
    - prepost folder with verus function stubs showing the pre and post conditions
    - verus folder containing reasonable verus proofs for each function
- preprocessing
  - happens on raw cpp before synthesizer runs
  - catches a few key things
    - whether pointers are read-only or part of output
    - whether parameters are readonly length descriptors for pointer sizes
    - throw that is caught inside of function (-> early return)
    - pointer aliasing (-> mut reference or shared reference)
- verus
  - setup scripts in scripts/
  - check verus/ code with `./verus/verus --crate-type lib benchmark0/verus/testcase.rs

### rules
- `clippy` recommended linter (`cargo clippy --fix` to apply formatting), comes preinstalled with rust
- work on branches, PRs when applicable
- no AI-generated markdown files

### todo
- write c++ static analysis portion that handles three types of pointers
  - non-nullable pointer -> &T
  - nullable pointer -> Option<&T>
  - output pointer (changing the pointer is part of the point of the function) -> ...difficult
- fix dataset0 performance - currently generating spurious solutions for many examples
