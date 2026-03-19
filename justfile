root := justfile_directory()
python := root / ".venv/bin/python"
autoverus := root / "verus-proof-synthesis/autoverus"

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
