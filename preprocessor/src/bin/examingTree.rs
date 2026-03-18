//! Print extracted variables, functions, operators, and literals for example inputs.
//!
//! From `preprocessor/`:
//!   cargo run --bin examing_tree

use anyhow::{Context, Result};
use cpp_preprocessor::extract_all;
use cpp_preprocessor::parse_cpp_source;
use std::path::PathBuf;

fn main() -> Result<()> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let test_inputs = manifest.join("test_inputs");

    let examples = [
        "add_one.cpp",
        "literals.cpp",
        "hello_main.cpp",
        "minimal_class.cpp",
    ];

    for name in examples {
        let path = test_inputs.join(name);
        println!("{}", "=".repeat(60));
        println!("file: {}", path.display());

        let source =
            std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
        let tree = parse_cpp_source(&source).with_context(|| format!("parse {}", path.display()))?;

        println!("root_has_error: {}", tree.root_node().has_error());

        let ex = extract_all(&source, &tree);

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
        }

        println!("\n-- operators ({}) --", ex.operators.len());
        for o in &ex.operators {
            println!("  {:?}  `{}`", o.kind, o.spelling);
        }

        println!("\n-- literals ({}) --", ex.literals.len());
        for (i, lit) in ex.literals.iter().enumerate() {
            println!("  [{}] {}", i, lit);
        }
        println!();
    }

    Ok(())
}
