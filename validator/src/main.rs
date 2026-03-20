use std::env;
use std::fs;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        eprintln!("Usage: {} <spec_file> <impl_file> <output_file>", args[0]);
        process::exit(1);
    }

    let spec_path = &args[1];
    let impl_path = &args[2];
    let output_path = &args[3];

    let spec_content = fs::read_to_string(spec_path)
        .unwrap_or_else(|e| { eprintln!("Error reading spec file '{}': {}", spec_path, e); process::exit(1); });
    let impl_content = fs::read_to_string(impl_path)
        .unwrap_or_else(|e| { eprintln!("Error reading impl file '{}': {}", impl_path, e); process::exit(1); });

    // Step 1: Strip everything before `use vstd::prelude::*;`
    let marker = "use vstd::prelude::*;";
    let trimmed_spec = match spec_content.find(marker) {
        Some(pos) => &spec_content[pos..],
        None => { eprintln!("Marker '{}' not found in spec file.", marker); process::exit(1); }
    };

    // Step 2: Extract the body of the last fn in the impl file
    let impl_body = extract_fn_body(&impl_content)
        .unwrap_or_else(|| { eprintln!("Could not find a function body in impl file."); process::exit(1); });

    // Step 3: Detect indentation of `assume(false);` in spec
    let placeholder = "assume(false);";
    let assume_line = trimmed_spec.lines()
        .find(|l| l.contains(placeholder))
        .unwrap_or_else(|| { eprintln!("Placeholder '{}' not found in spec file.", placeholder); process::exit(1); });
    let target_indent_len = assume_line.len() - assume_line.trim_start().len();
    let target_indent = " ".repeat(target_indent_len);

    // Step 4: Re-indent the extracted body to match the spec's indentation level
    let reindented = impl_body
        .lines()
        .map(|l| if l.trim().is_empty() { l.to_string() } else { format!("{}{}", target_indent, l) })
        .collect::<Vec<_>>()
        .join("\n");

    // Step 5: Splice — replace the whole `    assume(false);` line
    let result = trimmed_spec.replace(assume_line, &reindented);

    fs::write(output_path, &result)
        .unwrap_or_else(|e| { eprintln!("Error writing output file '{}': {}", output_path, e); process::exit(1); });

    println!("Done. Output written to '{}'.", output_path);
}

/// Extracts the body of the last `fn` in `source`, dedented to zero indentation.
fn extract_fn_body(source: &str) -> Option<String> {
    let fn_pos = source.rfind("fn ")?;
    let open_pos = source[fn_pos..].find('{')? + fn_pos;

    let mut depth = 0usize;
    let mut close_pos = None;
    for (i, ch) in source[open_pos..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 { close_pos = Some(open_pos + i); break; }
            }
            _ => {}
        }
    }
    let close_pos = close_pos?;

    let body = &source[open_pos + 1..close_pos];

    // Find minimum indentation of non-empty lines
    let base_indent = body
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.len() - l.trim_start().len())
        .min()
        .unwrap_or(0);

    // Dedent to zero
    let dedented = body
        .lines()
        .map(|l| {
            if l.len() >= base_indent && l[..base_indent].trim().is_empty() {
                &l[base_indent..]
            } else {
                l.trim_start()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    Some(dedented.trim_matches('\n').to_string())
}