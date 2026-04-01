use clap::Parser;
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

#[derive(Parser)]
#[command(
    name = "bench",
    about = "Benchmark the Rust synthesizer across targets"
)]
struct Cli {
    #[arg(long, default_value = "synthesizer/dataset0")]
    dataset: PathBuf,
    /// Use a C++ dataset directory instead (runs cpp2json first, times only synth)
    #[arg(long)]
    cpp_dataset: Option<PathBuf>,
    #[arg(long, default_value = "synthesizer/symbols.txt")]
    symbols: PathBuf,
    #[arg(long, default_value_t = 8)]
    max_depth: usize,
    #[arg(long, default_value_t = 30)]
    timeout: u64,
    #[arg(long, default_value_t = 1)]
    runs: usize,
    /// Specific targets to benchmark (default: all)
    #[arg(long, num_args = 1..)]
    targets: Vec<String>,
    /// Disable a heuristic by name (repeatable). Valid names:
    /// ordering, absent, required, structural, block-sizes
    #[arg(long, value_name = "NAME")]
    disable_heuristic: Vec<String>,
    #[arg(long, default_value_t = 1_000_000)]
    worklist_cap: usize,
}

fn fmt_candidates(n: u64) -> String {
    if n < 1_000 { return n.to_string(); }
    if n < 1_000_000 { return format!("{:.1}k", n as f64 / 1_000.0); }
    format!("{:.1}M", n as f64 / 1_000_000.0)
}

fn synth_binary() -> PathBuf {
    let exe = std::env::current_exe().unwrap();
    exe.parent().unwrap().join("synth")
}

fn cpp2json_binary() -> PathBuf {
    let exe = std::env::current_exe().unwrap();
    exe.parent().unwrap().join("cpp2json")
}

fn list_cpp_targets(dir: &PathBuf) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .expect("cannot read cpp dataset dir")
        .filter_map(|e| {
            let e = e.ok()?;
            let name = e.file_name().into_string().ok()?;
            name.ends_with(".cpp")
                .then(|| name[..name.len() - 4].to_string())
        })
        .collect();
    names.sort();
    names
}

/// Runs cpp2json on a .cpp file and returns path to the generated JSON.
/// Returns Err if cpp2json fails.
fn generate_json(cpp2json: &PathBuf, cpp_file: &PathBuf) -> Result<PathBuf, String> {
    let out = std::env::temp_dir().join(format!(
        "bench_{}_{}.json",
        cpp_file.file_stem().unwrap().to_str().unwrap(),
        std::process::id(),
    ));
    let result = Command::new(cpp2json)
        .args([cpp_file.to_str().unwrap(), "--out", out.to_str().unwrap()])
        .output()
        .map_err(|e| e.to_string())?;
    if result.status.success() {
        Ok(out)
    } else {
        Err(String::from_utf8_lossy(&result.stderr).to_string())
    }
}

/// Find the project root by looking for symbols.txt, searching from cwd
/// then from the binary's location upward. Works whether invoked from
/// the project root or from inside synthesizer/.
fn project_root() -> PathBuf {
    let from_binary = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap() // target/release
        .parent()
        .unwrap() // target
        .parent()
        .unwrap() // synthesizer
        .parent()
        .unwrap() // project root
        .to_path_buf();

    let from_cwd = std::env::current_dir().unwrap();

    for dir in [&from_cwd, &from_binary] {
        if dir.join("synthesizer/symbols.txt").exists() {
            return dir.clone();
        }
    }
    from_cwd
}

struct RunResult {
    elapsed: f64,
    found: bool,
    cap_hit: bool,
    candidates: Option<u64>,
}

fn run_once(binary: &PathBuf, target: &str, cli: &Cli, json_file: Option<&PathBuf>) -> RunResult {
    let t0 = Instant::now();
    let mut cmd = Command::new(binary);
    if let Some(f) = json_file {
        cmd.args(["--file", f.to_str().unwrap()]);
    } else {
        cmd.args([
            "--target", target,
            "--dataset", cli.dataset.to_str().unwrap(),
            "--symbols", cli.symbols.to_str().unwrap(),
        ]);
    }
    cmd.args([
        "--max-depth",    &cli.max_depth.to_string(),
        "--timeout",      &cli.timeout.to_string(),
        "--worklist-cap", &cli.worklist_cap.to_string(),
    ]);
    for name in &cli.disable_heuristic {
        cmd.args(["--disable-heuristic", name]);
    }
    let output = cmd.output().expect("failed to run synth binary");
    let elapsed = t0.elapsed().as_secs_f64();
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Parse candidate count from "FOUND in Xs  (N candidates, ...)" or
    // "TIMEOUT after N expansions, M/Z candidates tested"
    let candidates = stdout.lines().find_map(|line| {
        if line.contains("candidates") {
            line.split('(').nth(1)
                .and_then(|s| s.split_whitespace().next())
                .and_then(|s| s.parse::<u64>().ok())
                .or_else(|| {
                    // timeout format: "N expansions, M/Z candidates tested"
                    line.split(',').nth(1)
                        .and_then(|s| s.trim().split_whitespace().next())
                        .and_then(|s| s.split('/').next())
                        .and_then(|s| s.parse::<u64>().ok())
                })
        } else {
            None
        }
    });
    RunResult {
        elapsed,
        found: stdout.contains("FOUND"),
        cap_hit: stdout.contains("worklist cap hit"),
        candidates,
    }
}

fn list_targets(dataset: &PathBuf) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dataset)
        .expect("cannot read dataset dir")
        .filter_map(|e| {
            let e = e.ok()?;
            let name = e.file_name().into_string().ok()?;
            name.ends_with(".json")
                .then(|| name[..name.len() - 5].to_string())
        })
        .collect();
    names.sort();
    names
}

fn main() {
    let mut cli = Cli::parse();
    let root = project_root();

    if cli.dataset == PathBuf::from("synthesizer/dataset0") {
        cli.dataset = root.join("synthesizer/dataset0");
    }
    if cli.symbols == PathBuf::from("synthesizer/symbols.txt") {
        cli.symbols = root.join("synthesizer/symbols.txt");
    }

    let binary = synth_binary();
    let cpp2json = cpp2json_binary();

    let (targets, cpp_dir) = if let Some(ref dir) = cli.cpp_dataset {
        (if cli.targets.is_empty() { list_cpp_targets(dir) } else { cli.targets.clone() }, Some(dir.clone()))
    } else {
        (if cli.targets.is_empty() { list_targets(&cli.dataset) } else { cli.targets.clone() }, None)
    };

    let mode = if cpp_dir.is_some() { "C++ dataset" } else { "JSON dataset" };
    let disabled_str = if cli.disable_heuristic.is_empty() {
        "none".to_string()
    } else {
        cli.disable_heuristic.join(", ")
    };
    println!("Benchmarking {} targets, {} run(s) each  [{}]", targets.len(), cli.runs, mode);
    println!(
        "max-depth={}  timeout={}s  cap={}  disabled={}  binary={}",
        cli.max_depth,
        cli.timeout,
        cli.worklist_cap,
        disabled_str,
        binary.display()
    );
    println!();
    println!(
        "{:<20} {:<12} {:>8} {:>8} {:>8} {:>10}  {}",
        "target", "status", "mean", "min", "max", "cands", "warnings"
    );
    println!("{}", "-".repeat(80));

    let mut solved = 0usize;
    let mut solved_times: Vec<f64> = Vec::new();

    for target in &targets {
        // For C++ mode, generate JSON once (outside run timing)
        let json_file: Option<PathBuf> = if let Some(ref dir) = cpp_dir {
            let cpp_file = dir.join(format!("{target}.cpp"));
            match generate_json(&cpp2json, &cpp_file) {
                Ok(p) => Some(p),
                Err(e) => {
                    println!("{:<16} {:<12}", target, "CPP2JSON_ERR");
                    eprintln!("  cpp2json error for {target}: {e}");
                    continue;
                }
            }
        } else {
            None
        };

        let mut times: Vec<f64> = Vec::new();
        let mut found = false;
        let mut cap_hit = false;
        let mut last_candidates: Option<u64> = None;
        for _ in 0..cli.runs {
            let r = run_once(&binary, target, &cli, json_file.as_ref());
            times.push(r.elapsed);
            found = r.found;
            cap_hit |= r.cap_hit;
            last_candidates = r.candidates;
        }
        let mean = times.iter().sum::<f64>() / times.len() as f64;
        let min = times.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = times.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let status = if found { "FOUND" } else { "TIMEOUT" };
        let cands_str = last_candidates.map(|c| fmt_candidates(c)).unwrap_or_else(|| "-".into());
        let warnings = if cap_hit { "⚠ worklist cap hit" } else { "" };
        println!(
            "{:<20} {:<12} {:>7.2}s {:>7.2}s {:>7.2}s {:>10}  {}",
            target, status, mean, min, max, cands_str, warnings
        );
        if found {
            solved += 1;
            solved_times.push(mean);
        }
    }

    println!();
    println!("Solved: {}/{}", solved, targets.len());
    if !solved_times.is_empty() {
        let avg = solved_times.iter().sum::<f64>() / solved_times.len() as f64;
        println!("Mean time (solved only): {:.2}s", avg);
    }
}
