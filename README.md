### overview
- synthesizer contains all synthesizer code and tests
  - dataset/ contains 10 simple examples with depth < 8
  - code is broken down inside synthesizer/
- using rust
  - to get started, [install rust](https://rust-lang.org/tools/install/) and run `cd synthesizer && cargo build --release && ./target/release/bench` to make sure you're up to speed
  - `cargo build` to build, add `--release` flag for optimizations (run from synthesizer/)
  - `cargo check` to see if you have viable code without waiting for compile
  - `./target/debug/executable` or `./target/release/executable` to run

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
