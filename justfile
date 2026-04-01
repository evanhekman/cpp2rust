root := justfile_directory()
python := root / ".venv/bin/python"
# TEMP: targets excluded from preprocess and synthesize until preprocessor supports them
skip_targets := "graphs doubly_linekedlsit shared_mutable_aliasing"
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
    _skip() { for s in {{skip_targets}}; do [ "$1" = "$s" ] && return 0; done; return 1; }
    if [ -z "{{FUNC}}" ]; then
        for cpp_file in "$cpp_dir"/*.cpp; do
            func="$(basename "$cpp_file" .cpp)"
            if _skip "$func"; then echo "skipping $func (excluded)"; continue; fi
            _preprocess "$cpp_file" "$func"
        done
    else
        _preprocess "$cpp_dir/{{FUNC}}.cpp" "{{FUNC}}"
    fi

# full pipeline: preprocess, synthesize, then validate
# just pipeline benchmark0                  → all targets
# just pipeline benchmark0 dot_product      → one target
pipeline BENCH FUNC="": (preprocess BENCH FUNC) (synthesize BENCH FUNC) (validate BENCH FUNC)

# validate stitched rust files using autoverus
# just validate benchmark0                  → all targets
# just validate benchmark0 dot_product      → one target
validate BENCH FUNC="":
    #!/usr/bin/env bash
    set -euo pipefail
    stitched_dir={{data}}/{{BENCH}}/stitched
    validated_dir={{data}}/{{BENCH}}/validated
    mkdir -p "$validated_dir"
    _validate() {
        local rs_file="$1" func="$2"
        echo "validating $func..."
        {{python}} {{root}}/validator/validate.py \
            --input "$rs_file" \
            --output "$validated_dir/${func}.rs"
    }
    if [ -z "{{FUNC}}" ]; then
        for rs_file in "$stitched_dir"/*.rs; do
            func="$(basename "$rs_file" .rs)"
            _validate "$rs_file" "$func"
        done
    else
        _validate "$stitched_dir/{{FUNC}}.rs" "{{FUNC}}"
    fi

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
    _skip() { for s in {{skip_targets}}; do [ "$1" = "$s" ] && return 0; done; return 1; }
    if [ -z "{{TARGET}}" ]; then
        targets=""
        for json in "$dataset"/*.json; do
            t="$(basename "$json" .json)"
            _skip "$t" || targets="$targets $t"
        done
        {{bench}} --dataset "$dataset" --symbols {{symbols}} --targets $targets $disable_flags
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
