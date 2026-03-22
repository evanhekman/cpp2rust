root := justfile_directory()
python := root / ".venv/bin/python"
autoverus := root / "verus-proof-synthesis/autoverus"
synth  := root / "target/release/synth"
bench  := root / "target/release/bench"
processed := root / "data/benchmark0/processed"
synth_bench := root / "data/synth_benchmark/processed"
symbols := root / "synthesizer/symbols.txt"

# build all synthesizer binaries
build:
    cargo build --release

# synthesize benchmark0 target(s)
# usage: just synthesize              → all four targets
#        just synthesize dot_product  → one target
synthesize target="benchmark0":
    #!/usr/bin/env bash
    set -euo pipefail
    if [ "{{target}}" = "benchmark0" ]; then
        {{bench}} \
            --dataset {{processed}} \
            --symbols {{symbols}}
    else
        {{synth}} \
            --file {{processed}}/{{target}}.json \
            --symbols {{symbols}}
    fi

# run controlled synthesis benchmark (depth 4-8)
# usage: just synth-bench            → all five targets
#        just synth-bench sum_array  → one target
synth-bench target="all":
    #!/usr/bin/env bash
    set -euo pipefail
    if [ "{{target}}" = "all" ]; then
        {{bench}} \
            --dataset {{synth_bench}} \
            --symbols {{symbols}}
    else
        {{synth}} \
            --file {{synth_bench}}/{{target}}.json \
            --symbols {{symbols}}
    fi

# run verus on a file
verus FILE:
    {{root}}/verus/verus --crate-type lib {{FILE}}

# run autoverus on a file
autoverus INPUT OUTPUT:
    {{python}} {{autoverus}}/main.py \
        --input {{INPUT}} \
        --output {{OUTPUT}} \
        --config {{autoverus}}/config.json

# run the pinned verus build used by autoverus
old-verus FILE:
    {{root}}/verus-proof-synthesis/verus/source/target-verus/release/verus --crate-type lib {{FILE}}
