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
mod neighbors;
mod oracle;
mod translator;
mod worklist;

use ast::{Child, Node};
use canonicalize::should_prune;
use codegen::render;
use eval::eval_fn;
use grammar::{build_grammar, build_reverse_map, count_programs, register_fn_def_known};
use heuristics::{score, HeuristicConfig};
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
    /// Disable a heuristic by name (repeatable). Valid names:
    /// ordering, absent, required, structural, block-sizes
    #[arg(long, value_name = "NAME")]
    disable_heuristic: Vec<String>,
    #[arg(long, default_value_t = 1_000_000)]
    worklist_cap: usize,
    /// Print the verbatim C++ translation seed and exit (no synthesis).
    #[arg(long)]
    show_seed: bool,
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
    let cases = &target.test_cases;
    if cases.is_empty() {
        return true;
    }

    let is_void = target.return_type == "()" || target.return_type.is_empty();
    let check = |tc: &loader::TestCase| -> bool {
        let env = parse_env(tc, &target.params);
        match eval_fn(node, &env, grammar) {
            Some((val, final_env)) => {
                if is_void {
                    if let Some(mut_param) = target.params.iter().find(|p| p.ty.starts_with("&mut")) {
                        final_env.get(&mut_param.name)
                            .map(|v| v.matches_str(&tc.expected_output))
                            .unwrap_or(false)
                    } else {
                        val.matches_str(&tc.expected_output)
                    }
                } else {
                    val.matches_str(&tc.expected_output)
                }
            }
            None => false,
        }
    };

    // Quick filter: test against case 0 only; run all if it passes.
    check(&cases[0]) && cases[1..].iter().all(check)
}

fn synthesize(
    target: &loader::Target,
    literals: &[String],
    max_depth: usize,
    timeout: u64,
    worklist_cap: usize,
    interrupted: &Arc<AtomicBool>,
    hcfg: &HeuristicConfig,
) -> Option<(String, u64, u64)> {
    let mut grammar = build_grammar(literals, &target.params, &target.local_vars, &target.return_type);
    let kind = register_fn_def_known(target, &mut grammar);
    let candidates_possible = count_programs(&grammar, &kind, max_depth);
    let reverse_map = build_reverse_map(&grammar);

    let root = Node::new(&kind, vec![Child::Hole("Block".into())], 0);
    let mut worklist = Worklist::with_capacity(worklist_cap);
    let mut visited: std::collections::HashSet<u64> = std::collections::HashSet::new();

    // Seed with the verbatim C++ translation if available, and its neighbors.
    if let Some(ast) = &target.ast {
        if let Some(block) = translator::translate(ast, &target.params, &target.local_vars, &grammar, &target.return_type) {
            let translated = root.replace_at_path(&[0], block);
            let h = translated.structural_hash();
            if visited.insert(h) {
                let s = score(&translated, target.cpp_features.as_ref(), target.ast_hints.as_deref(), Some(&target.block_sizes), Some(&target.required_idents), hcfg);
                worklist.push(translated.clone(), s.min(-10));
                // Seed neighbors of the translation (one subtree punched out each)
                for nbr in neighbors::neighbors(&translated, &reverse_map, 4) {
                    let nh = nbr.structural_hash();
                    if visited.insert(nh) {
                        let ns = score(&nbr, target.cpp_features.as_ref(), target.ast_hints.as_deref(), Some(&target.block_sizes), Some(&target.required_idents), hcfg);
                        worklist.push(nbr, ns);
                    }
                }
            }
        }
    }

    // Always also seed the generic root-hole so the search is complete.
    let root_hash = root.structural_hash();
    if visited.insert(root_hash) {
        worklist.push(root, 0);
    }

    let deadline = Instant::now() + Duration::from_secs(timeout);
    let mut candidates_tried: u64 = 0;
    let mut nodes_expanded: u64 = 0;

    while !worklist.is_empty() && !interrupted.load(Ordering::Relaxed) {
        if Instant::now() > deadline {
            let evictions = worklist.evictions();
            print!(
                "  TIMEOUT after {} expansions, {}/{} candidates tested",
                nodes_expanded,
                candidates_tried,
                fmt_count(candidates_possible)
            );
            if evictions > 0 {
                print!("\n  WARNING: worklist cap hit — {} candidates evicted, valid solutions may have been missed", evictions);
            }
            println!();
            return None;
        }

        let partial = worklist.pop().unwrap();

        if partial.is_complete() {
            candidates_tried += 1;
            if test_candidate(&partial, target, &grammar) {
                return render(&partial, &grammar).ok().map(|src| (src, candidates_tried, nodes_expanded));
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
            let h = new_partial.structural_hash();
            if !visited.insert(h) { continue; }
            let s = score(&new_partial, target.cpp_features.as_ref(), target.ast_hints.as_deref(), Some(&target.block_sizes), Some(&target.required_idents), hcfg);
            worklist.push(new_partial, s);
        }
    }

    if interrupted.load(Ordering::Relaxed) {
        println!("  Interrupted");
        return None;
    }

    let evictions = worklist.evictions();
    print!(
        "  Search exhausted after {} expansions, {}/{} candidates tested",
        nodes_expanded,
        candidates_tried,
        fmt_count(candidates_possible)
    );
    if evictions > 0 {
        print!("\n  WARNING: worklist cap hit — {} candidates evicted, valid solutions may have been missed", evictions);
    }
    println!();
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
    let mut target = if let Some(ref file) = cli.file {
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

    // If no hand-written test cases, compile the C++ oracle and generate them.
    if target.test_cases.is_empty() {
        let cpp_source = target.cpp_source.clone().unwrap_or_else(|| {
            eprintln!("Error: no test_cases and no C++ source found — cannot synthesize.");
            std::process::exit(1);
        });
        print!("Compiling C++ oracle... ");
        let _ = std::io::Write::flush(&mut std::io::stdout());
        let binary = oracle::compile_oracle(
            &target.name, &target.params, &target.return_type, &cpp_source,
        ).unwrap_or_else(|e| { eprintln!("Error: {}", e); std::process::exit(1); });
        println!("done.");

        print!("Generating test cases... ");
        let _ = std::io::Write::flush(&mut std::io::stdout());
        let cases = oracle::generate_test_cases(
            &binary, &target.params, &target.return_type, /*seed=*/42, /*count=*/8,
        ).unwrap_or_else(|e| { eprintln!("Error: {}", e); std::process::exit(1); });
        println!("{} generated.", cases.len());
        target.test_cases = cases;
    }

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
    if !target.local_vars.is_empty() {
        println!("Locals:    {}", target.local_vars.iter().map(|(n,t)| format!("{}: {}", n, t)).collect::<Vec<_>>().join(", "));
    }
    if let Some(hints) = &target.ast_hints {
        println!("AST hints: {}", hints.join(" → "));
    }
    if !target.required_idents.is_empty() {
        println!("Req idents: {}", target.required_idents.join(", "));
    }
    if !target.block_sizes.is_empty() {
        println!("Block sizes: {}", target.block_sizes.iter().map(|s| s.to_string()).collect::<Vec<_>>().join(" → "));
    }
    if cli.show_seed {
        let grammar = {
            let mut g = build_grammar(&literals, &target.params, &target.local_vars, &target.return_type);
            register_fn_def_known(&target, &mut g);
            g
        };
        match target.ast.as_ref().and_then(|ast| translator::translate(ast, &target.params, &target.local_vars, &grammar, &target.return_type)) {
            Some(block) => {
                let root = Node::new(&format!("FnDefKnown_{}", target.name), vec![Child::Node(Box::new(block))], 0);
                match codegen::render(&root, &grammar) {
                    Ok(src) => println!("{}", src),
                    Err(e)  => println!("(render error: {:?})", e),
                }
            }
            None => println!("(no translation available)"),
        }
        return;
    }

    let hcfg = HeuristicConfig::from_disabled(&cli.disable_heuristic);
    let disabled_str = if cli.disable_heuristic.is_empty() {
        "none".to_string()
    } else {
        cli.disable_heuristic.join(", ")
    };
    println!(
        "Literals:  {}  Max depth: {}  Timeout: {}s  Disabled: {}\n",
        literals.len(),
        cli.max_depth,
        cli.timeout,
        disabled_str,
    );

    let t0 = Instant::now();
    let result = synthesize(&target, &literals, cli.max_depth, cli.timeout, cli.worklist_cap, &interrupted, &hcfg);
    let elapsed = t0.elapsed().as_secs_f64();

    if let Some((src, candidates, expansions)) = result {
        println!(
            "  FOUND in {:.1}s  ({} candidates, {} expansions):\n  {}",
            elapsed, candidates, expansions, src
        );
    } else if !interrupted.load(Ordering::Relaxed) {
        println!("  FAILED in {:.1}s", elapsed);
    }
}
