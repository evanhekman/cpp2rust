### overview
- synthesizer contains all synthesizer code and tests
  - dataset/ contains 10 simple examples with depth < 8
  - code is broken down inside synthesizer/
- using rust
  - to get started, [install rust](https://rust-lang.org/tools/install/) and run `cd synthesizer && cargo check` to make sure you're running
  - generally run commands from inside synthesizer/
  - `cargo build` to build, add `--release` flag for optimizations
  - `./target/debug/executable` or `./target/release/executable` to run

### rules
- `cargo clippy` recommended linter (`cargo clippy --fix` to apply formatting)
- no AI-generated markdown files
-
