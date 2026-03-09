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
