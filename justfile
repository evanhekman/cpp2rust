root := justfile_directory()
python := root / ".venv/bin/python"
autoverus := root / "verus-proof-synthesis/autoverus"
synth  := root / "target/release/synth"
bench  := root / "target/release/bench"
data   := root / "data"
symbols := root / "synthesizer/symbols.txt"
cpp2json := root / "target/release/cpp2json_cpp"
mapjson  := root / "target/release/map_cpp_json_to_rust_json"

# build all binaries (incremental — fast if nothing changed)
build:
    cargo build --release

# preprocess C++ files into a benchmark's processed/ directory
# just preprocess benchmark0                  → all targets
# just preprocess benchmark0 dot_product      → one target
preprocess BENCH FUNC="": build
    #!/usr/bin/env bash
    set -euo pipefail
    cpp_dir={{data}}/{{BENCH}}/cpp
    _preprocess() {
        local cpp_file="$1" func="$2"
        local tmp; tmp=$(mktemp /tmp/${func}_cpp_XXXXXX)
        trap "rm -f $tmp" EXIT
        if ! {{cpp2json}} "$cpp_file" --out "$tmp" 2>&1; then
            echo "skipping $func (preprocessor could not parse)"
            return
        fi
        {{mapjson}} "$tmp" --out {{data}}/{{BENCH}}/processed/${func}.json
    }
    if [ -z "{{FUNC}}" ]; then
        for cpp_file in "$cpp_dir"/*.cpp; do
            _preprocess "$cpp_file" "$(basename "$cpp_file" .cpp)"
        done
    else
        _preprocess "$cpp_dir/{{FUNC}}.cpp" "{{FUNC}}"
    fi

# full pipeline: preprocess then synthesize
# just pipeline benchmark0                  → all targets
# just pipeline benchmark0 dot_product      → one target
pipeline BENCH FUNC="": (preprocess BENCH FUNC) (synthesize BENCH FUNC)

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
