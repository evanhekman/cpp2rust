use clap::Parser;
use serde_json::Value;
use std::io::Read;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "json2rust", about = "Wrap a synthesized Rust fn in a complete runnable .rs file")]
struct Cli {
    #[arg(long)]
    json: PathBuf,
    /// Synthesized function source (reads from stdin if omitted)
    #[arg(long, name = "fn")]
    fn_src: Option<String>,
    #[arg(long)]
    out: Option<PathBuf>,
}

fn rust_literal(val: &str, rust_type: &str) -> String {
    match rust_type {
        "bool" => val.to_string(),
        _ => format!("{val}_i32"),
    }
}

fn build_test_call(
    name: &str,
    params: &[(&str, &str)],
    ret_rust: &str,
    inputs: &[&str],
    expected: &str,
    idx: usize,
) -> Vec<String> {
    let needs_block = params.iter().any(|(_, t)| matches!(*t, "&i32" | "Option<&i32>"));
    let indent = "    ";
    let inner = if needs_block { "        " } else { "    " };
    let mut lines = Vec::new();

    if needs_block {
        lines.push(format!("{indent}// test case {}", idx + 1));
        lines.push(format!("{indent}{{"));
    }

    let mut arg_exprs = Vec::new();
    for ((pname, ptype), val) in params.iter().zip(inputs.iter()) {
        match *ptype {
            "&i32" => {
                lines.push(format!("{inner}let _{pname}_{idx} = {val}_i32;"));
                arg_exprs.push(format!("&_{pname}_{idx}"));
            }
            "Option<&i32>" => {
                if *val == "null" {
                    arg_exprs.push("None".into());
                } else {
                    lines.push(format!("{inner}let _{pname}_{idx} = {val}_i32;"));
                    arg_exprs.push(format!("Some(&_{pname}_{idx})"));
                }
            }
            "bool" => arg_exprs.push(val.to_string()),
            _ => arg_exprs.push(format!("{val}_i32")),
        }
    }

    let call = format!("{name}({})", arg_exprs.join(", "));
    let exp = rust_literal(expected, ret_rust);
    lines.push(format!("{inner}assert_eq!({call}, {exp}, \"test case {} failed\");", idx + 1));

    if needs_block {
        lines.push(format!("{indent}}}"));
    }
    lines
}

fn generate(target: &Value, fn_src: &str) -> String {
    let name = target["name"].as_str().unwrap();
    let ret_rust = target["return_type"].as_str().unwrap();
    let params: Vec<(&str, &str)> = target["params"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| (p["name"].as_str().unwrap(), p["type"].as_str().unwrap()))
        .collect();
    let test_cases = target["test_cases"].as_array().unwrap();

    let mut lines = vec![fn_src.trim().to_string(), String::new(), "fn main() {".into()];

    for (i, tc) in test_cases.iter().enumerate() {
        let inputs: Vec<&str> = tc["inputs"].as_array().unwrap()
            .iter().map(|v| v.as_str().unwrap()).collect();
        let expected = tc["expected_output"].as_str().unwrap();
        lines.extend(build_test_call(name, &params, ret_rust, &inputs, expected, i));
    }

    let n = test_cases.len();
    lines.push(format!("    println!(\"All {n} tests passed!\");"));
    lines.push("}".into());
    lines.join("\n") + "\n"
}

fn main() {
    let cli = Cli::parse();

    let target: Value = {
        let text = std::fs::read_to_string(&cli.json).unwrap_or_else(|e| {
            eprintln!("Error reading {:?}: {e}", cli.json);
            std::process::exit(1);
        });
        serde_json::from_str(&text).unwrap_or_else(|e| {
            eprintln!("JSON parse error: {e}");
            std::process::exit(1);
        })
    };

    let fn_src = match cli.fn_src {
        Some(s) => s,
        None => {
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf).unwrap();
            buf.trim().to_string()
        }
    };

    let result = generate(&target, &fn_src);

    match cli.out {
        Some(path) => {
            std::fs::write(&path, &result).unwrap_or_else(|e| {
                eprintln!("Error writing {path:?}: {e}");
                std::process::exit(1);
            });
            eprintln!("Written to {path:?}");
        }
        None => print!("{result}"),
    }
}
