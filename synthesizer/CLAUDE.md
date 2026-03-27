# Synthesizer

Heuristic search over a Rust grammar. Takes `processed/` inputs, produces `stitched/` outputs.

- `processed/` — simplified C++ AST + Rust function signature (from preprocessor)
- `prepost/` — pre/post conditions used only as a wrapper around synthesized code
- `cpp/` — original C++ used only to generate test cases

For system-level context, see @../CLAUDE.md.

## Rules

- keep files below 800 lines
- keep functions simple and single-purpose
- never sweep errors under the rug
- use `clippy` to lint rust files (`cargo clippy --fix --allow-dirty`)
- if a command is run more than once, consider adding it to the justfile
- readonly git operations *only*, never commit or push.

## Testing

- prefer quick, targeted tests over long black-box runs
- timeout limit: 30 seconds MAX
- `just synthesize <benchmark> [target]` — runs synthesis (builds automatically); omit target to run all
- if a long test fails:
  - exhausts options before time limit -> correct solutions are being pruned
  - hits time limit -> heuristics are too slow; isolate the relevant subcase, shrink it, fix that first
  - worklist cap exceeded -> might be evicting valid solutions 

## Interaction

Always alert the user when you:
- create a new test case
- find a key bug
- find convincing results for heuristic effectiveness
- make changes to the justfile

Always ask the user to do these things manually:
- run a Python script not via an established shebang
- install a new dependency
