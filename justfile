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
# just synthesize synthesizer/b0                                       → all b0 targets
# just synthesize synthesizer/b0 sum_array                             → one b0 target
# just synthesize synthesizer/b1                                       → all b1 targets
# just synthesize benchmark0                                           → all benchmark0 targets
# just synthesize synthesizer/b0 "" "absent structural"               → disable heuristics
synthesize BENCH TARGET="" DISABLE="": build
    #!/usr/bin/env bash
    set -euo pipefail
    dataset={{data}}/{{BENCH}}/processed
    disable_flags=""
    for h in {{DISABLE}}; do
        disable_flags="$disable_flags --disable-heuristic $h"
    done
    if [ -z "{{TARGET}}" ]; then
        {{bench}} --dataset "$dataset" --symbols {{symbols}} $disable_flags
    else
        {{synth}} --file "$dataset/{{TARGET}}.json" --symbols {{symbols}} $disable_flags
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
