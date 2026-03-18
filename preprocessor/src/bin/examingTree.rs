//! Print extracted features and optionally write them to JSON.
//!
//! From `preprocessor/`:
//!   cargo run --bin examing_tree
//!   cargo run --bin examing_tree -- --json test_outputs/extracted_batch.json
//!   cargo run --bin examing_tree -- --json-dir test_outputs/json

use anyhow::{Context, Result};
use cpp_preprocessor::{
    extract_all, parse_cpp_source, write_batch_json, write_extracted_json, ExtractedBatch,
    ExtractedFileRecord,
};
use std::env;
use std::path::PathBuf;

fn main() -> Result<()> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let test_inputs = manifest.join("test_inputs");

    let examples = [
        "add_one.cpp",
        "literals.cpp",
        "hello_main.cpp",
        "minimal_class.cpp",
        "control_flow.cpp",
        "nullable_ptr.cpp",
    ];

    let args: Vec<String> = env::args().skip(1).collect();
    let mut json_batch_path: Option<PathBuf> = None;
    let mut json_dir: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--json" => {
                i += 1;
                let p = args.get(i).context("--json requires a file path")?;
                json_batch_path = Some(PathBuf::from(p));
            }
            "--json-dir" => {
                i += 1;
                let p = args.get(i).context("--json-dir requires a directory")?;
                json_dir = Some(PathBuf::from(p));
            }
            "--help" | "-h" => {
                eprintln!(
                    "Usage: examing_tree [--json BATCH.json] [--json-dir DIR/]\n\
                     \n\
                     Default: print extraction for test_inputs/*.cpp\n\
                     --json PATH   Write one JSON file with all files under {{ \"files\": [...] }}\n\
                     --json-dir D  Write one JSON per input (add_one.json, ...)"
                );
                return Ok(());
            }
            other => anyhow::bail!("unknown arg: {} (try --help)", other),
        }
        i += 1;
    }

    let mut batch_records = Vec::new();

    for name in examples {
        let path = test_inputs.join(name);
        println!("{}", "=".repeat(60));
        println!("file: {}", path.display());

        let source =
            std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
        let tree = parse_cpp_source(&source).with_context(|| format!("parse {}", path.display()))?;

        let root_has_error = tree.root_node().has_error();
        println!("root_has_error: {}", root_has_error);

        let ex = extract_all(&source, &tree);

        if let Some(ref dir) = json_dir {
            std::fs::create_dir_all(dir)?;
            let stem = path.file_stem().unwrap_or_default().to_string_lossy();
            let out = dir.join(format!("{}.json", stem));
            write_extracted_json(&out, &ex)?;
            eprintln!("  wrote {}", out.display());
        }

        batch_records.push(ExtractedFileRecord {
            path: path.to_string_lossy().to_string(),
            root_has_error,
            extracted: ex.clone(),
        });

        println!("\n-- functions ({}) --", ex.functions.len());
        for f in &ex.functions {
            let params: Vec<String> = f
                .parameters
                .iter()
                .map(|(t, n)| {
                    if n.is_empty() {
                        t.clone()
                    } else {
                        format!("{} {}", t, n)
                    }
                })
                .collect();
            println!(
                "  {} {}({})  [{}]",
                f.return_type,
                f.name,
                params.join(", "),
                if f.is_definition { "def" } else { "decl" }
            );
        }

        println!("\n-- variables ({}) --", ex.variables.len());
        for v in &ex.variables {
            println!(
                "  {:12} {:20} type: {}",
                format!("{:?}", v.role),
                v.name,
                v.type_spelling
            );
            if let Some(note) = &v.nullability_note {
                println!("    note: {}", note);
            }
        }

        println!("\n-- operators ({}) --", ex.operators.len());
        for o in &ex.operators {
            println!("  {:?}  `{}`", o.kind, o.spelling);
        }

        println!("\n-- literals ({}) --", ex.literals.len());
        for (idx, lit) in ex.literals.iter().enumerate() {
            println!("  [{}] {}", idx, lit);
        }

        println!("\n-- control_flow ({}) --", ex.control_flow.len());
        for c in &ex.control_flow {
            if c.header.is_empty() {
                println!("  {:?}  ({})", c.kind, c.node_kind);
            } else {
                println!("  {:?}  ({})  header: {}", c.kind, c.node_kind, c.header);
            }
        }
        println!();
    }

    if let Some(path) = json_batch_path {
        let batch = ExtractedBatch {
            files: batch_records,
        };
        write_batch_json(&path, &batch)?;
        eprintln!("Wrote batch JSON: {}", path.display());
    }

    Ok(())
}
