root := justfile_directory()
python := root / ".venv/bin/python"
autoverus := root / "verus-proof-synthesis/autoverus"
synth  := root / "target/release/synth"
bench  := root / "target/release/bench"
data   := root / "data"
symbols := root / "synthesizer/symbols.txt"

# build synthesizer binaries (incremental — fast if nothing changed)
build:
    cargo build --release

# synthesize targets in a benchmark dataset
# just synthesize synth_benchmark              → all targets
# just synthesize synth_benchmark sum_array    → one target
# just synthesize benchmark0                   → all targets
# just synthesize benchmark0 dot_product       → one target
synthesize BENCH TARGET="": build
    #!/usr/bin/env bash
    set -euo pipefail
    dataset={{data}}/{{BENCH}}/processed
    if [ -z "{{TARGET}}" ]; then
        {{bench}} --dataset "$dataset" --symbols {{symbols}}
    else
        {{synth}} --file "$dataset/{{TARGET}}.json" --symbols {{symbols}}
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
