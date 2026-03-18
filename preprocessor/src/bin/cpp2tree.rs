//! CLI: dump Tree-sitter S-expression for a C++ file (or batch over a directory).
//!
//! Usage:
//!   cargo run --bin cpp2tree -- path/to/file.cpp              # stdout
//!   cargo run --bin cpp2tree -- path/to/file.cpp -o out.txt   # file
//!   cargo run --bin cpp2tree -- --batch test_inputs test_outputs

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let args = &args[1..];

    if args.first().map(|s| s.as_str()) == Some("--batch") {
        let in_dir = args.get(1).context("usage: cpp2tree --batch <input_dir> <output_dir>")?;
        let out_dir = args.get(2).context("usage: cpp2tree --batch <input_dir> <output_dir>")?;
        batch(in_dir, out_dir)?;
        return Ok(());
    }

    let (input, out_path) = match args {
        [] => {
            eprintln!("Usage: cpp2tree <file.cpp> [-o out.txt]");
            eprintln!("       cpp2tree --batch <input_dir> <output_dir>");
            std::process::exit(1);
        }
        [path] => (path.as_str(), None),
        [path, flag, out] if flag == "-o" => (path.as_str(), Some(PathBuf::from(out))),
        _ => anyhow::bail!("unexpected args: {:?}", args),
    };

    let tree = cpp_preprocessor::parse_cpp_file(input)?;
    let sexp = cpp_preprocessor::tree_as_sexp(&tree);
    let text = format!(
        "// parsed: {}\n// root_has_error: {}\n\n{}\n",
        input,
        tree.root_node().has_error(),
        sexp
    );
    if let Some(p) = out_path {
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&p, &text)?;
        eprintln!("wrote {}", p.display());
    } else {
        print!("{}", text);
    }
    Ok(())
}

fn batch(in_dir: &str, out_dir: &str) -> Result<()> {
    let in_path = Path::new(in_dir);
    let out_path = Path::new(out_dir);
    fs::create_dir_all(out_path)?;
    for entry in fs::read_dir(in_path).with_context(|| format!("read_dir {}", in_dir))? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("cpp") {
            continue;
        }
        let tree = cpp_preprocessor::parse_cpp_file(&path)?;
        let sexp = cpp_preprocessor::tree_as_sexp(&tree);
        let stem = path.file_stem().unwrap().to_string_lossy();
        let dest = out_path.join(format!("{}.tree.txt", stem));
        let text = format!(
            "// parsed: {}\n// root_has_error: {}\n\n{}\n",
            path.display(),
            tree.root_node().has_error(),
            sexp
        );
        fs::write(&dest, &text)?;
        eprintln!("wrote {}", dest.display());
    }
    Ok(())
}
