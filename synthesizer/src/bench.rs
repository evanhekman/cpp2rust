use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;
use clap::Parser;

#[derive(Parser)]
#[command(name = "bench", about = "Benchmark the Rust synthesizer across targets")]
struct Cli {
    #[arg(long, default_value = "synthesizer/dataset")]
    dataset: PathBuf,
    #[arg(long, default_value = "symbols.txt")]
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
}

fn synth_binary() -> PathBuf {
    // Look for release binary next to bench, then debug
    let exe = std::env::current_exe().unwrap();
    let dir = exe.parent().unwrap();
    let release = dir.join("synth");
    if release.exists() { release } else { dir.join("synth") }
}

struct RunResult {
    elapsed: f64,
    found: bool,
}

fn run_once(binary: &PathBuf, target: &str, cli: &Cli) -> RunResult {
    let t0 = Instant::now();
    let output = Command::new(binary)
        .args([
            "--target", target,
            "--max-depth", &cli.max_depth.to_string(),
            "--timeout", &cli.timeout.to_string(),
            "--dataset", cli.dataset.to_str().unwrap(),
            "--symbols", cli.symbols.to_str().unwrap(),
        ])
        .output()
        .expect("failed to run synth binary");
    let elapsed = t0.elapsed().as_secs_f64();
    let stdout = String::from_utf8_lossy(&output.stdout);
    RunResult { elapsed, found: stdout.contains("FOUND") }
}

fn list_targets(dataset: &PathBuf) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dataset)
        .expect("cannot read dataset dir")
        .filter_map(|e| {
            let e = e.ok()?;
            let name = e.file_name().into_string().ok()?;
            name.ends_with(".json").then(|| name[..name.len() - 5].to_string())
        })
        .collect();
    names.sort();
    names
}

fn main() {
    let cli = Cli::parse();
    let binary = synth_binary();
    let targets = if cli.targets.is_empty() {
        list_targets(&cli.dataset)
    } else {
        cli.targets.clone()
    };

    println!("Benchmarking {} targets, {} run(s) each", targets.len(), cli.runs);
    println!("max-depth={}  timeout={}s  binary={}", cli.max_depth, cli.timeout, binary.display());
    println!();
    println!("{:<16} {:<8} {:>8} {:>8} {:>8}", "target", "status", "mean", "min", "max");
    println!("{}", "-".repeat(52));

    let mut solved = 0usize;
    let mut solved_times: Vec<f64> = Vec::new();

    for target in &targets {
        let mut times: Vec<f64> = Vec::new();
        let mut found = false;
        for _ in 0..cli.runs {
            let r = run_once(&binary, target, &cli);
            times.push(r.elapsed);
            found = r.found;
        }
        let mean = times.iter().sum::<f64>() / times.len() as f64;
        let min = times.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = times.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let status = if found { "FOUND" } else { "TIMEOUT" };
        println!("{:<16} {:<8} {:>7.2}s {:>7.2}s {:>7.2}s", target, status, mean, min, max);
        if found { solved += 1; solved_times.push(mean); }
    }

    println!();
    println!("Solved: {}/{}", solved, targets.len());
    if !solved_times.is_empty() {
        let avg = solved_times.iter().sum::<f64>() / solved_times.len() as f64;
        println!("Mean time (solved only): {:.2}s", avg);
    }
}
