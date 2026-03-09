use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use clap::Parser;

mod ast;
mod grammar;
mod eval;
mod worklist;
mod heuristics;
mod canonicalize;
mod loader;
mod codegen;

use ast::{Node, Child};
use grammar::{build_grammar, register_fn_def_known, count_programs};
use eval::eval_fn;
use worklist::Worklist;
use heuristics::score;
use canonicalize::should_prune;
use loader::{load_symbols, load_target, parse_env};
use codegen::render;

#[derive(Parser)]
#[command(name = "synth", about = "Rust Program Synthesizer")]
struct Cli {
    #[arg(long, default_value = "symbols.txt")]
    symbols: PathBuf,
    #[arg(long, default_value = "synthesizer/dataset")]
    dataset: PathBuf,
    #[arg(long)]
    target: String,
    #[arg(long, default_value_t = 8)]
    max_depth: usize,
    #[arg(long, default_value_t = 300)]
    timeout: u64,
}

fn fmt_count(n: u128) -> String {
    if n < 1_000_000 { return n.to_string(); }
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
            println!("  TIMEOUT after {} expansions, {}/{} candidates tested",
                nodes_expanded, candidates_tried, fmt_count(candidates_possible));
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
            prods.into_iter().filter(|p| p.children_spec.is_empty()).collect()
        } else {
            prods
        };
        if prods.is_empty() { continue; }

        nodes_expanded += 1;

        for prod in &prods {
            let new_children: Vec<Child> = prod.children_spec.iter()
                .map(|nt| Child::Hole(nt.clone()))
                .collect();
            let replacement = Node::new(&prod.name, new_children, hole_depth + 1);
            if should_prune(&partial, &path, &replacement) { continue; }
            let new_partial = partial.replace_at_path(&path, replacement);
            let s = score(&new_partial);
            worklist.push(new_partial, s);
        }
    }

    if interrupted.load(Ordering::Relaxed) {
        println!("  Interrupted");
        return None;
    }

    println!("  Search exhausted after {} expansions, {}/{} candidates tested",
        nodes_expanded, candidates_tried, fmt_count(candidates_possible));
    None
}

fn main() {
    let cli = Cli::parse();

    let interrupted = Arc::new(AtomicBool::new(false));
    let interrupted_clone = interrupted.clone();
    ctrlc::set_handler(move || {
        interrupted_clone.store(true, Ordering::Relaxed);
        eprintln!("\n  Interrupted — stopping search.");
    }).expect("Error setting Ctrl-C handler");

    let literals = match load_symbols(&cli.symbols) {
        Ok(l) => l,
        Err(e) => { eprintln!("Error loading symbols: {}", e); std::process::exit(1); }
    };
    let target = match load_target(&cli.dataset, &cli.target) {
        Ok(t) => t,
        Err(e) => { eprintln!("Error: {}", e); std::process::exit(1); }
    };

    let param_sig = target.params.iter()
        .map(|p| format!("{}: {}", p.name, p.ty))
        .collect::<Vec<_>>().join(", ");

    println!("Target:    {}", target.name);
    println!("Signature: pub fn {}({}) -> {}", target.name, param_sig, target.return_type);
    if let Some(ex) = &target.example_rust { println!("Example:   {}", ex); }
    println!("Tests:     {} cases", target.test_cases.len());
    println!("Literals:  {}  Max depth: {}  Timeout: {}s\n", literals.len(), cli.max_depth, cli.timeout);

    let t0 = Instant::now();
    let result = synthesize(&target, &literals, cli.max_depth, cli.timeout, &interrupted);
    let elapsed = t0.elapsed().as_secs_f64();

    if let Some(src) = result {
        println!("  FOUND in {:.1}s:\n  {}", elapsed, src);
    } else if !interrupted.load(Ordering::Relaxed) {
        println!("  FAILED in {:.1}s", elapsed);
    }
}
