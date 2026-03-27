# cpp2rust

Translates C++ functions and pre/post conditions to formally verified Rust. Three components in sequence:

```
cpp/ → [preprocessor] → processed/ 
processed/ + cpp/ + prepost/ → [synthesizer] → stitched/ 
stitched/ → [validator] → validated/
```

## Components

**Preprocessor** — static analysis on C++; detects pointer-length params, mutable/nullable pointers; produces simplified AST and Rust function signature. (no preprocessor/CLAUDE.md yet)

**Synthesizer** — heuristic search over a Rust grammar to produce candidate programs from preprocessor output. See @synthesizer/CLAUDE.md for full context.

**Validator** — uses Verus to prove Rust candidates correct against pre/post conditions; returns counterexample, indeterminate, or success. (no validator/CLAUDE.md yet)

## Data layout

```
data/benchmark0/      full pipeline benchmark (cpp/, prepost/, processed/, stitched/, validated/)
data/synthesizer/     synthesizer-only benchmarks (b0/, b1/)
```

## Details
See README.md for more information (Rust, Verus, AutoVerus, Justfile).
