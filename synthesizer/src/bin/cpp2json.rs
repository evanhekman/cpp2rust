use clap::Parser;
use regex::Regex;
use serde_json::{json, Value};
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

#[derive(Parser)]
#[command(name = "cpp2json", about = "Convert a C++ function file to synthesizer JSON")]
struct Cli {
    cpp_file: PathBuf,
    #[arg(long)]
    out: Option<PathBuf>,
    #[arg(long, default_value_t = 6)]
    n: usize,
    #[arg(long, default_value = "clang++")]
    compiler: String,
}

// ── Minimal PRNG (xorshift64) ─────────────────────────────────────────────────

struct Rng(u64);

impl Rng {
    fn new() -> Self {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        Self(seed ^ 0xdeadbeef_cafebabe)
    }
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn range(&mut self, lo: i32, hi: i32) -> i32 {
        let span = (hi - lo + 1) as u64;
        (self.next() % span) as i32 + lo
    }
    fn prob(&mut self, pct: u64) -> bool {
        self.next() % 100 < pct
    }
}

// ── C++ type → Rust type ──────────────────────────────────────────────────────

fn is_nullable(param: &str, body: &str) -> bool {
    // Heuristic: treat a pointer as Option<&i32> if the function body contains
    // an explicit null check for it.
    //
    // LIMITATIONS:
    //   - Only matches direct comparisons against nullptr/NULL and bare boolean
    //     checks (if (p), if (!p)). Does not detect:
    //       * assert(p != nullptr) or other precondition-style guards
    //       * null checks inside helper functions called from the body
    //       * pointer arithmetic used as a null sentinel
    //       * macros that expand to null checks
    //   - A pointer that is always non-null by documented contract will be
    //     treated as non-nullable if no null check appears in the source text,
    //     which is the correct result for this heuristic.
    let p = regex::escape(param);
    let patterns = [
        format!(r"{p}\s*==\s*(nullptr|NULL)"),
        format!(r"(nullptr|NULL)\s*==\s*{p}"),
        format!(r"{p}\s*!=\s*(nullptr|NULL)"),
        format!(r"(nullptr|NULL)\s*!=\s*{p}"),
        format!(r"if\s*\(\s*!\s*{p}\s*\)"),
        format!(r"if\s*\(\s*{p}\s*\)"),
    ];
    patterns.iter().any(|pat| Regex::new(pat).unwrap().is_match(body))
}

fn cpp_type_to_rust(cpp_type: &str, param: &str, body: &str) -> String {
    if cpp_type.contains('*') {
        if is_nullable(param, body) {
            "Option<&i32>".into()
        } else {
            "&i32".into()
        }
    } else {
        match cpp_type.trim() {
            "int" => "i32",
            "bool" => "bool",
            other => other,
        }
        .into()
    }
}

// ── Signature parsing ─────────────────────────────────────────────────────────

struct Param {
    name: String,
    rust_type: String,
}

struct Signature {
    name: String,
    params: Vec<Param>,
    ret_rust: String,
}

fn parse_signature(src: &str) -> Result<Signature, String> {
    let re = Regex::new(r"(?s)([\w\s*]+?)\s+(\w+)\s*\(([^)]*)\)\s*\{(.*)\}")
        .map_err(|e| e.to_string())?;
    let caps = re.captures(src).ok_or("Could not parse function signature")?;

    let ret_cpp = caps[1].trim();
    let name = caps[2].trim().to_string();
    let params_str = caps[3].trim();
    let body = &caps[4];

    let mut params = Vec::new();
    if !params_str.is_empty() {
        for part in params_str.split(',') {
            let part = part.trim();
            // rsplit on whitespace: last token is the name
            let (type_part, name_part) = part.rsplit_once(char::is_whitespace)
                .ok_or_else(|| format!("Cannot parse param: {part:?}"))?;
            let mut pname = name_part.trim_start_matches('*').to_string();
            let mut ptype = type_part.trim().to_string();
            // handle `const int *p` — star on the name side
            if name_part.starts_with('*') {
                ptype.push('*');
                pname = name_part.trim_start_matches('*').to_string();
            }
            let rust_type = cpp_type_to_rust(&ptype, &pname, body);
            params.push(Param { name: pname, rust_type });
        }
    }

    let ret_rust = match ret_cpp {
        "int" => "i32",
        "bool" => "bool",
        other => other,
    }
    .to_string();

    Ok(Signature { name, params, ret_rust })
}

// ── Test harness generation ───────────────────────────────────────────────────

fn build_harness(func_src: &str, sig: &Signature) -> String {
    let mut lines = vec![
        "#include <iostream>".into(),
        "#include <cstdlib>".into(),
        "#include <cstring>".into(),
        String::new(),
        func_src.trim().to_string(),
        String::new(),
        "int main(int argc, char* argv[]) {".into(),
    ];

    let mut arg_exprs = Vec::new();
    for (i, p) in sig.params.iter().enumerate() {
        let argv = format!("argv[{}]", i + 1);
        match p.rust_type.as_str() {
            "&i32" | "Option<&i32>" => {
                lines.push(format!("    const int* _{} = nullptr;", p.name));
                lines.push(format!("    int _{}_val;", p.name));
                lines.push(format!("    if (strcmp({argv}, \"null\") != 0) {{"));
                lines.push(format!("        _{}_val = atoi({argv});", p.name));
                lines.push(format!("        _{} = &_{}_val;", p.name, p.name));
                lines.push("    }".into());
                arg_exprs.push(format!("_{}", p.name));
            }
            "bool" => {
                lines.push(format!("    bool _{} = strcmp({argv}, \"true\") == 0;", p.name));
                arg_exprs.push(format!("_{}", p.name));
            }
            _ => {
                lines.push(format!("    int _{} = atoi({argv});", p.name));
                arg_exprs.push(format!("_{}", p.name));
            }
        }
    }

    let call = format!("{}({})", sig.name, arg_exprs.join(", "));
    if sig.ret_rust == "bool" {
        lines.push(format!("    std::cout << ({call} ? \"true\" : \"false\") << std::endl;"));
    } else {
        lines.push(format!("    std::cout << {call} << std::endl;"));
    }
    lines.push("    return 0;".into());
    lines.push("}".into());
    lines.join("\n")
}

fn compile_harness(src: &str, compiler: &str) -> Result<PathBuf, String> {
    let dir = std::env::temp_dir().join(format!(
        "cpp2json_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let src_path = dir.join("harness.cpp");
    let bin_path = dir.join("harness");
    std::fs::write(&src_path, src).map_err(|e| e.to_string())?;
    let out = Command::new(compiler)
        .args(["-std=c++17", "-o", bin_path.to_str().unwrap(), src_path.to_str().unwrap()])
        .output()
        .map_err(|e| format!("Failed to run compiler: {e}"))?;
    if !out.status.success() {
        return Err(format!("Compile error:\n{}", String::from_utf8_lossy(&out.stderr)));
    }
    Ok(bin_path)
}

fn run_harness(bin: &PathBuf, inputs: &[String]) -> Result<String, String> {
    let out = Command::new(bin)
        .args(inputs)
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(format!("Runtime error:\n{}", String::from_utf8_lossy(&out.stderr)));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

// ── Random input generation ───────────────────────────────────────────────────

fn random_inputs(sig: &Signature, rng: &mut Rng) -> Vec<String> {
    sig.params.iter().map(|p| match p.rust_type.as_str() {
        "Option<&i32>" => {
            if rng.prob(33) { "null".into() } else { rng.range(-20, 20).to_string() }
        }
        "bool" => if rng.prob(50) { "true".into() } else { "false".into() },
        _ => rng.range(-20, 20).to_string(),
    }).collect()
}

fn generate_test_cases(
    func_src: &str,
    sig: &Signature,
    n: usize,
    compiler: &str,
) -> Result<Vec<Value>, String> {
    let harness_src = build_harness(func_src, sig);
    let bin = compile_harness(&harness_src, compiler)?;

    let mut rng = Rng::new();
    let mut cases = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut attempts = 0;

    while cases.len() < n && attempts < n * 20 {
        attempts += 1;
        let inputs = random_inputs(sig, &mut rng);
        if !seen.insert(inputs.clone()) {
            continue;
        }
        match run_harness(&bin, &inputs) {
            Ok(output) => cases.push(json!({ "inputs": inputs, "expected_output": output })),
            Err(e) => eprintln!("  Warning: skipped case: {e}"),
        }
    }

    if cases.len() < n {
        return Err(format!("Only generated {}/{n} test cases", cases.len()));
    }
    Ok(cases)
}

// ── C++ feature extraction ────────────────────────────────────────────────────

/// Scan the C++ body left-to-right and return (operator_counts, operator_sequence).
///
/// Operator names map to Rust grammar node prefixes, with one special case:
///   "IfElse" — matches both ExprIfElse_* and StmtIfElse in the Rust AST.
///
/// Overlap resolution: at any position, the longest match wins (multi-char
/// operators like ">=" take priority over ">"). Matches that start inside an
/// already-covered range are skipped.
///
/// Known false positives:
///   - "*" is detected as ExprMul even when it is a pointer dereference.
///   - "-" in return types like "->", function names, etc. may produce spurious
///     ExprSub entries, though our simple dataset functions don't use these.
fn scan_operators(body: &str, params: &[Param]) -> (std::collections::HashMap<String, usize>, Vec<String>) {
    // Build dynamic param-specific patterns first (higher priority via longer match).
    // These are processed before the generic scan to correctly classify pointer ops.
    let mut dynamic: Vec<(String, &str)> = Vec::new();
    for p in params {
        let pe = regex::escape(&p.name);
        match p.rust_type.as_str() {
            "Option<&i32>" => {
                // null checks → Option predicates
                dynamic.push((format!(r"\b{pe}\s*!=\s*(nullptr|NULL)\b"), "ExprOptIsSome"));
                dynamic.push((format!(r"(nullptr|NULL)\s*!=\s*\b{pe}\b"), "ExprOptIsSome"));
                dynamic.push((format!(r"\b{pe}\s*==\s*(nullptr|NULL)\b"), "ExprOptIsNone"));
                dynamic.push((format!(r"(nullptr|NULL)\s*==\s*\b{pe}\b"), "ExprOptIsNone"));
                // bare boolean checks: if (p) / if (!p)
                dynamic.push((format!(r"if\s*\(\s*{pe}\s*\)"), "ExprOptIsSome"));
                dynamic.push((format!(r"if\s*\(\s*!\s*{pe}\s*\)"), "ExprOptIsNone"));
                // dereference → unwrap
                dynamic.push((format!(r"\*\s*{pe}\b"), "ExprOptUnwrapOr"));
            }
            "&i32" => {
                // dereference of non-nullable pointer is auto-deref in Rust; suppress ExprMul
                dynamic.push((format!(r"\*\s*{pe}\b"), "__skip__"));
            }
            _ => {}
        }
    }

    // (regex, feature_name) — longer/multi-char patterns listed before single-char
    // so that when two patterns match the same position, the longer one has a later
    // end index and wins the sort.
    let generic: &[(&str, &str)] = &[
        (r"==",    "ExprEq"),
        (r"!=",    "ExprNe"),
        (r">=",    "ExprGe"),
        (r"<=",    "ExprLe"),
        (r"&&",    "ExprAnd"),
        (r"\|\|",  "ExprOr"),
        (r"\bif\b","IfElse"),   // each `if` = one if/else construct
        (r">",     "ExprGt"),
        (r"<",     "ExprLt"),
        (r"\+",    "ExprAdd"),
        (r"-",     "ExprSub"),
        (r"\*",    "ExprMul"),  // false positive: also matches dereference
        (r"/",     "ExprDiv"),
        (r"%",     "ExprMod"),
        (r"!",     "ExprNot"),
    ];

    // Collect all matches as (start, end, name)
    // Dynamic patterns use owned Strings; generic use &str.
    let mut all: Vec<(usize, usize, String)> = Vec::new();
    for (pat, name) in &dynamic {
        let re = Regex::new(pat).unwrap();
        for m in re.find_iter(body) {
            all.push((m.start(), m.end(), name.to_string()));
        }
    }
    for (pat, name) in generic {
        let re = Regex::new(pat).unwrap();
        for m in re.find_iter(body) {
            all.push((m.start(), m.end(), name.to_string()));
        }
    }

    // Sort by start position; ties broken by longest match first
    all.sort_by(|a, b| a.0.cmp(&b.0).then(b.1.cmp(&a.1)));

    // Greedily collect non-overlapping matches
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut sequence: Vec<String> = Vec::new();
    let mut covered = 0usize;
    for (start, end, name) in all {
        if start >= covered {
            covered = end;
            if name != "__skip__" {
                *counts.entry(name.clone()).or_default() += 1;
                sequence.push(name);
            }
        }
    }

    (counts, sequence)
}

fn extract_features(body: &str, params: &[Param]) -> Value {
    let (counts, sequence) = scan_operators(body, params);
    json!({
        "operator_counts": counts,
        "operator_sequence": sequence,
    })
}

// ── Main ──────────────────────────────────────────────────────────────────────

fn main() {
    let cli = Cli::parse();

    let src = std::fs::read_to_string(&cli.cpp_file).unwrap_or_else(|e| {
        eprintln!("Error reading {:?}: {e}", cli.cpp_file);
        std::process::exit(1);
    });

    let sig = parse_signature(&src).unwrap_or_else(|e| {
        eprintln!("Parse error: {e}");
        std::process::exit(1);
    });

    eprintln!("Function: {}", sig.name);
    eprintln!("Params:   {:?}", sig.params.iter().map(|p| (&p.name, &p.rust_type)).collect::<Vec<_>>());
    eprintln!("Returns:  {}", sig.ret_rust);

    let test_cases = generate_test_cases(&src, &sig, cli.n, &cli.compiler).unwrap_or_else(|e| {
        eprintln!("Error generating test cases: {e}");
        std::process::exit(1);
    });

    let features = {
        let re = Regex::new(r"(?s)\{(.*)\}").unwrap();
        let body = re.captures(&src).and_then(|c| c.get(1)).map(|m| m.as_str()).unwrap_or(&src);
        extract_features(body, &sig.params)
    };

    let out = json!({
        "name": sig.name,
        "params": sig.params.iter().map(|p| json!({"name": p.name, "type": p.rust_type})).collect::<Vec<_>>(),
        "return_type": sig.ret_rust,
        "example_cpp": src.trim(),
        "cpp_features": features,
        "test_cases": test_cases,
    });

    let result = serde_json::to_string_pretty(&out).unwrap();
    match cli.out {
        Some(path) => {
            std::fs::write(&path, &result).unwrap_or_else(|e| {
                eprintln!("Error writing {path:?}: {e}");
                std::process::exit(1);
            });
            eprintln!("Written to {path:?}");
        }
        None => println!("{result}"),
    }
}
