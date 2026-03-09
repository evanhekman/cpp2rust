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
    #[arg(long, default_value = "synthesizer/dataset")]
    dataset: PathBuf,
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
}

fn synth_binary() -> PathBuf {
    let exe = std::env::current_exe().unwrap();
    exe.parent().unwrap().join("synth")
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
}

fn run_once(binary: &PathBuf, target: &str, cli: &Cli) -> RunResult {
    let t0 = Instant::now();
    let output = Command::new(binary)
        .args([
            "--target",
            target,
            "--max-depth",
            &cli.max_depth.to_string(),
            "--timeout",
            &cli.timeout.to_string(),
            "--dataset",
            cli.dataset.to_str().unwrap(),
            "--symbols",
            cli.symbols.to_str().unwrap(),
        ])
        .output()
        .expect("failed to run synth binary");
    let elapsed = t0.elapsed().as_secs_f64();
    let stdout = String::from_utf8_lossy(&output.stdout);
    RunResult {
        elapsed,
        found: stdout.contains("FOUND"),
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

    if cli.dataset == PathBuf::from("synthesizer/dataset") {
        cli.dataset = root.join("synthesizer/dataset");
    }
    if cli.symbols == PathBuf::from("synthesizer/symbols.txt") {
        cli.symbols = root.join("synthesizer/symbols.txt");
    }

    let binary = synth_binary();
    let targets = if cli.targets.is_empty() {
        list_targets(&cli.dataset)
    } else {
        cli.targets.clone()
    };

    println!(
        "Benchmarking {} targets, {} run(s) each",
        targets.len(),
        cli.runs
    );
    println!(
        "max-depth={}  timeout={}s  binary={}",
        cli.max_depth,
        cli.timeout,
        binary.display()
    );
    println!();
    println!(
        "{:<16} {:<8} {:>8} {:>8} {:>8}",
        "target", "status", "mean", "min", "max"
    );
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
        println!(
            "{:<16} {:<8} {:>7.2}s {:>7.2}s {:>7.2}s",
            target, status, mean, min, max
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
