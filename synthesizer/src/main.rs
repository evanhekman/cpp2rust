use clap::Parser;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

mod ast;
mod canonicalize;
mod codegen;
mod eval;
mod grammar;
mod heuristics;
mod loader;
mod worklist;

use ast::{Child, Node};
use canonicalize::should_prune;
use codegen::render;
use eval::eval_fn;
use grammar::{build_grammar, count_programs, register_fn_def_known};
use heuristics::score;
use loader::{load_symbols, load_target, load_target_file, parse_env};
use worklist::Worklist;

#[derive(Parser)]
#[command(name = "synth", about = "Rust Program Synthesizer")]
struct Cli {
    #[arg(long, default_value = "synthesizer/symbols.txt")]
    symbols: PathBuf,
    #[arg(long, default_value = "synthesizer/dataset0")]
    dataset: PathBuf,
    #[arg(long)]
    target: Option<String>,
    /// Direct path to a JSON target file (bypasses --dataset and --target)
    #[arg(long)]
    file: Option<PathBuf>,
    #[arg(long, default_value_t = 8)]
    max_depth: usize,
    #[arg(long, default_value_t = 300)]
    timeout: u64,
}

fn fmt_count(n: u128) -> String {
    if n < 1_000_000 {
        return n.to_string();
    }
    let s = n.to_string();
    let exp = s.len() - 1;
    let mantissa = n as f64 / 10f64.powi(exp as i32);
    format!("{:.2}e{}", mantissa, exp)
}

fn test_candidate(node: &Node, target: &loader::Target, grammar: &grammar::Grammar) -> bool {
    target.test_cases.iter().all(|tc| {
        let env = parse_env(tc, &target.params);
        match eval_fn(node, &env, grammar) {
            Some(val) => val.matches_str(&tc.expected_output),
            None => false,
        }
    })
}

fn synthesize(
    target: &loader::Target,
    literals: &[String],
    max_depth: usize,
    timeout: u64,
    interrupted: &Arc<AtomicBool>,
) -> Option<String> {
    let mut grammar = build_grammar(literals, &target.params, &target.return_type);
    let kind = register_fn_def_known(target, &mut grammar);
    let candidates_possible = count_programs(&grammar, &kind, max_depth);

    let root = Node::new(&kind, vec![Child::Hole("Block".into())], 0);
    let mut worklist = Worklist::new();
    worklist.push(root, 0);

    let deadline = Instant::now() + Duration::from_secs(timeout);
    let mut candidates_tried: u64 = 0;
    let mut nodes_expanded: u64 = 0;

    while !worklist.is_empty() && !interrupted.load(Ordering::Relaxed) {
        if Instant::now() > deadline {
            println!(
                "  TIMEOUT after {} expansions, {}/{} candidates tested",
                nodes_expanded,
                candidates_tried,
                fmt_count(candidates_possible)
            );
            return None;
        }

        let partial = worklist.pop().unwrap();

        if partial.is_complete() {
            candidates_tried += 1;
            if test_candidate(&partial, target, &grammar) {
                return render(&partial, &grammar).ok();
            }
            continue;
        }

        let path = match partial.first_hole_path() {
            Some(p) => p,
            None => continue,
        };

        let hole_depth = path.len();
        let nt = partial.hole_nt_at_path(&path).to_string();
        let prods = match grammar.get(&nt) {
            Some(p) => p.clone(),
            None => continue,
        };

        let prods: Vec<_> = if hole_depth >= max_depth {
            prods
                .into_iter()
                .filter(|p| p.children_spec.is_empty())
                .collect()
        } else {
            prods
        };
        if prods.is_empty() {
            continue;
        }

        nodes_expanded += 1;

        for prod in &prods {
            let new_children: Vec<Child> = prod
                .children_spec
                .iter()
                .map(|nt| Child::Hole(nt.clone()))
                .collect();
            let replacement = Node::new(&prod.name, new_children, hole_depth + 1);
            if should_prune(&partial, &path, &replacement) {
                continue;
            }
            let new_partial = partial.replace_at_path(&path, replacement);
            let s = score(&new_partial);
            worklist.push(new_partial, s);
        }
    }

    if interrupted.load(Ordering::Relaxed) {
        println!("  Interrupted");
        return None;
    }

    println!(
        "  Search exhausted after {} expansions, {}/{} candidates tested",
        nodes_expanded,
        candidates_tried,
        fmt_count(candidates_possible)
    );
    None
}

fn project_root() -> std::path::PathBuf {
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

fn main() {
    let mut cli = Cli::parse();
    let root = project_root();

    if cli.symbols == PathBuf::from("synthesizer/symbols.txt") {
        cli.symbols = root.join("synthesizer/symbols.txt");
    }
    if cli.dataset == PathBuf::from("synthesizer/dataset0") {
        cli.dataset = root.join("synthesizer/dataset0");
    }

    let interrupted = Arc::new(AtomicBool::new(false));
    let interrupted_clone = interrupted.clone();
    ctrlc::set_handler(move || {
        interrupted_clone.store(true, Ordering::Relaxed);
        eprintln!("\n  Interrupted — stopping search.");
    })
    .expect("Error setting Ctrl-C handler");

    let literals = match load_symbols(&cli.symbols) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Error loading symbols: {}", e);
            std::process::exit(1);
        }
    };
    let target = if let Some(ref file) = cli.file {
        match load_target_file(file) {
            Ok(t) => t,
            Err(e) => { eprintln!("Error: {}", e); std::process::exit(1); }
        }
    } else {
        let name = cli.target.as_deref().unwrap_or_else(|| {
            eprintln!("Error: provide --target <name> or --file <path>");
            std::process::exit(1);
        });
        match load_target(&cli.dataset, name) {
            Ok(t) => t,
            Err(e) => { eprintln!("Error: {}", e); std::process::exit(1); }
        }
    };

    let param_sig = target
        .params
        .iter()
        .map(|p| format!("{}: {}", p.name, p.ty))
        .collect::<Vec<_>>()
        .join(", ");

    println!("Target:    {}", target.name);
    println!(
        "Signature: pub fn {}({}) -> {}",
        target.name, param_sig, target.return_type
    );
    if let Some(ex) = &target.example_rust {
        println!("Example:   {}", ex);
    }
    println!("Tests:     {} cases", target.test_cases.len());
    println!(
        "Literals:  {}  Max depth: {}  Timeout: {}s\n",
        literals.len(),
        cli.max_depth,
        cli.timeout
    );

    let t0 = Instant::now();
    let result = synthesize(&target, &literals, cli.max_depth, cli.timeout, &interrupted);
    let elapsed = t0.elapsed().as_secs_f64();

    if let Some(src) = result {
        println!("  FOUND in {:.1}s:\n  {}", elapsed, src);
    } else if !interrupted.load(Ordering::Relaxed) {
        println!("  FAILED in {:.1}s", elapsed);
    }
}
