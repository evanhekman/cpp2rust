use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::eval::Value;
use crate::grammar::{elem_type_of, is_slice_type};
use crate::loader::{Param, TestCase};

// ── Seeded LCG (no external deps) ────────────────────────────────────────────

pub struct Lcg {
    state: u64,
}

impl Lcg {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.state
    }

    fn next_i32_range(&mut self, lo: i32, hi: i32) -> i32 {
        let range = (hi - lo + 1) as u64;
        lo + (self.next_u64() % range) as i32
    }

    fn next_usize_range(&mut self, lo: usize, hi: usize) -> usize {
        let range = (hi - lo + 1) as u64;
        lo + (self.next_u64() % range) as usize
    }

    fn next_bool(&mut self) -> bool {
        self.next_u64() % 2 == 0
    }
}

// ── Random input generation ───────────────────────────────────────────────────

/// Generate one input Value for a given Param.
/// `shared_len` is used for all slice params so they have a consistent length.
fn gen_one(param: &Param, shared_len: usize, rng: &mut Lcg) -> Value {
    match param.ty.as_str() {
        "i32" => Value::Int(rng.next_i32_range(-10, 10)),
        "u32" => Value::U32(rng.next_i32_range(0, 20) as u32),
        "bool" => Value::Bool(rng.next_bool()),
        "&i32" => Value::Int(rng.next_i32_range(-10, 10)),
        "Option<&i32>" => {
            if rng.next_bool() {
                Value::Opt(None)
            } else {
                Value::Opt(Some(rng.next_i32_range(-10, 10)))
            }
        }
        "&[i32]" => {
            Value::SliceI32((0..shared_len).map(|_| rng.next_i32_range(-10, 10)).collect())
        }
        "&mut [i32]" => {
            Value::SliceMutI32((0..shared_len).map(|_| rng.next_i32_range(-10, 10)).collect())
        }
        "&[u8]" => {
            Value::SliceU8((0..shared_len).map(|_| rng.next_i32_range(0, 20) as u8).collect())
        }
        _ => Value::Int(rng.next_i32_range(-10, 10)),
    }
}

/// Generate random inputs for all params.
/// All slice params get the same length so the shared C++ `n` is consistent.
pub fn gen_inputs(params: &[Param], rng: &mut Lcg) -> Vec<Value> {
    let shared_len = if params.iter().any(|p| is_slice_type(&p.ty)) {
        rng.next_usize_range(1, 6)
    } else {
        0
    };
    params.iter().map(|p| gen_one(p, shared_len, rng)).collect()
}

// ── C++ harness generation ────────────────────────────────────────────────────

fn cpp_elem_type(rust_ty: &str) -> &'static str {
    match rust_ty {
        "i32" => "int",
        "u8" => "uint8_t",
        "u32" => "uint32_t",
        "bool" => "bool",
        _ => "int",
    }
}

/// Generate a self-contained C++ harness that reads inputs from stdin,
/// calls the target function (included from `cpp_source_path`), and
/// prints the result to stdout.
///
/// Protocol (stdin):
///   - For each param in order:
///       slice: first line is N, then N lines each with one element value
///       scalar: one line with the value
///
/// Protocol (stdout):
///   - non-void return: one line with the return value
///   - void (unit): N on first line, then N lines with the modified &mut slice elements
pub fn generate_harness(
    fn_name: &str,
    params: &[Param],
    return_type: &str,
    cpp_source_path: &Path,
) -> String {
    let mut s = String::new();

    s.push_str("#include <iostream>\n");
    s.push_str("#include <vector>\n");
    s.push_str("#include <cstdint>\n\n");
    // Include the target function source directly (all four .cpp files are
    // small, header-free or use only standard headers with include guards).
    s.push_str(&format!("#include \"{}\"\n\n", cpp_source_path.display()));
    s.push_str("int main() {\n");

    // Collect slice params to derive the shared `n` later.
    let slice_params: Vec<&Param> = params.iter().filter(|p| is_slice_type(&p.ty)).collect();

    // Read each param from stdin.
    for param in params {
        if is_slice_type(&param.ty) {
            let elem_rust = elem_type_of(&param.ty).unwrap_or_else(|| "i32".into());
            let elem_cpp = cpp_elem_type(&elem_rust);
            let nvar = format!("{}_n", param.name);
            s.push_str(&format!("    int {nvar}; std::cin >> {nvar};\n"));
            s.push_str(&format!(
                "    std::vector<{elem}> {name}({nvar});\n",
                elem = elem_cpp,
                name = param.name,
                nvar = nvar,
            ));
            if elem_cpp == "uint8_t" {
                // cin >> doesn't work cleanly for uint8_t; read as int and cast.
                s.push_str(&format!(
                    "    for (int _i = 0; _i < {nvar}; _i++) {{ int _tmp; std::cin >> _tmp; {name}[_i] = ({elem})_tmp; }}\n",
                    nvar = nvar, name = param.name, elem = elem_cpp,
                ));
            } else {
                s.push_str(&format!(
                    "    for (int _i = 0; _i < {nvar}; _i++) std::cin >> {name}[_i];\n",
                    nvar = nvar, name = param.name,
                ));
            }
        } else {
            let cpp_ty = cpp_elem_type(&param.ty);
            s.push_str(&format!(
                "    {ty} {name}; std::cin >> {name};\n",
                ty = cpp_ty, name = param.name,
            ));
        }
    }

    // Build the argument list: slice pointers first (in param order),
    // followed by `n` once if any slices exist (shared length convention).
    let mut args: Vec<String> = Vec::new();
    for param in params {
        if is_slice_type(&param.ty) {
            args.push(format!("{}.data()", param.name));
        } else {
            args.push(param.name.clone());
        }
    }
    if !slice_params.is_empty() {
        args.push(format!("{}_n", slice_params[0].name));
    }

    let call = format!("{}({})", fn_name, args.join(", "));

    let is_void = return_type == "()" || return_type.is_empty();
    if is_void {
        s.push_str(&format!("    {call};\n"));
        // Print the first &mut slice param.
        if let Some(mp) = params.iter().find(|p| p.ty.starts_with("&mut")) {
            let nvar = format!("{}_n", mp.name);
            s.push_str(&format!("    std::cout << {nvar} << \"\\n\";\n"));
            s.push_str(&format!(
                "    for (int _i = 0; _i < {nvar}; _i++) std::cout << {name}[_i] << \"\\n\";\n",
                nvar = nvar, name = mp.name,
            ));
        }
    } else {
        s.push_str(&format!("    auto _result = {call};\n"));
        s.push_str("    std::cout << _result << \"\\n\";\n");
    }

    s.push_str("    return 0;\n}\n");
    s
}

// ── Compilation ───────────────────────────────────────────────────────────────

pub fn compile_oracle(
    fn_name: &str,
    params: &[Param],
    return_type: &str,
    cpp_source_path: &Path,
) -> Result<PathBuf, String> {
    let harness = generate_harness(fn_name, params, return_type, cpp_source_path);

    let harness_path = std::env::temp_dir().join(format!("synth_harness_{}.cpp", fn_name));
    let binary_path = std::env::temp_dir().join(format!("synth_oracle_{}", fn_name));

    std::fs::write(&harness_path, harness).map_err(|e| e.to_string())?;

    let output = Command::new("g++")
        .args([
            "-O0",
            "-fsanitize=address,undefined",
            "-o",
            binary_path.to_str().unwrap(),
            harness_path.to_str().unwrap(),
        ])
        .output()
        .map_err(|e| format!("Failed to invoke g++: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("g++ compilation failed for {}:\n{}", fn_name, stderr));
    }

    Ok(binary_path)
}

// ── Running the oracle ────────────────────────────────────────────────────────

fn encode_value(val: &Value) -> String {
    match val {
        Value::Int(n) => format!("{}\n", n),
        Value::U32(n) => format!("{}\n", n),
        Value::U8(n) => format!("{}\n", n),
        Value::Usize(n) => format!("{}\n", n),
        Value::Bool(b) => format!("{}\n", if *b { 1 } else { 0 }),
        Value::Opt(None) => "0\n".to_string(),
        Value::Opt(Some(n)) => format!("{}\n", n),
        Value::Unit => String::new(),
        Value::SliceI32(v) | Value::SliceMutI32(v) => {
            let mut s = format!("{}\n", v.len());
            for x in v {
                s.push_str(&format!("{}\n", x));
            }
            s
        }
        Value::SliceU8(v) => {
            let mut s = format!("{}\n", v.len());
            for x in v {
                s.push_str(&format!("{}\n", x));
            }
            s
        }
    }
}

fn decode_output(output: &str, return_type: &str, params: &[Param]) -> Result<Value, String> {
    let lines: Vec<&str> = output.lines().collect();

    let is_void = return_type == "()" || return_type.is_empty();
    if is_void {
        // Output is the modified &mut slice: n\nv[0]\n...
        let n: usize = lines
            .first()
            .ok_or("missing n in oracle output")?
            .trim()
            .parse::<usize>()
            .map_err(|e| e.to_string())?;
        let mut_param = params
            .iter()
            .find(|p| p.ty.starts_with("&mut"))
            .ok_or("no &mut param for void function")?;
        match elem_type_of(&mut_param.ty).as_deref() {
            Some("i32") => {
                let v: Result<Vec<i32>, _> = lines[1..=n]
                    .iter()
                    .map(|l| l.trim().parse::<i32>())
                    .collect();
                Ok(Value::SliceMutI32(v.map_err(|e| e.to_string())?))
            }
            Some(other) => Err(format!("unsupported &mut elem type: {}", other)),
            None => Err("could not determine elem type of &mut param".into()),
        }
    } else {
        let s = lines.first().ok_or("empty oracle output")?.trim();
        match return_type {
            "i32" => s.parse::<i32>().map(Value::Int).map_err(|e| e.to_string()),
            "u32" => s.parse::<u32>().map(Value::U32).map_err(|e| e.to_string()),
            "u8" => s.parse::<u8>().map(Value::U8).map_err(|e| e.to_string()),
            "bool" => Ok(Value::Bool(s == "1" || s == "true")),
            _ => s.parse::<i32>().map(Value::Int).map_err(|e| e.to_string()),
        }
    }
}

pub fn run_oracle(
    binary: &Path,
    inputs: &[Value],
    return_type: &str,
    params: &[Param],
) -> Result<Value, String> {
    let stdin_data: String = inputs.iter().map(encode_value).collect();

    let mut child = Command::new(binary)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("Failed to spawn oracle: {}", e))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(stdin_data.as_bytes())
            .map_err(|e| format!("Failed to write oracle stdin: {}", e))?;
    }

    let out = child
        .wait_with_output()
        .map_err(|e| format!("Failed to wait on oracle: {}", e))?;

    if !out.status.success() {
        return Err("Oracle process exited non-zero".into());
    }

    decode_output(&String::from_utf8_lossy(&out.stdout), return_type, params)
}

// ── Test case serialization ───────────────────────────────────────────────────

/// Render a Value to the string format that `parse_env` / `matches_str` expect.
pub fn value_to_test_string(val: &Value) -> String {
    match val {
        Value::Int(n) => n.to_string(),
        Value::U32(n) => n.to_string(),
        Value::U8(n) => n.to_string(),
        Value::Usize(n) => n.to_string(),
        Value::Bool(b) => if *b { "true" } else { "false" }.to_string(),
        Value::Opt(None) => "None".to_string(),
        Value::Opt(Some(n)) => format!("Some({})", n),
        Value::Unit => "()".to_string(),
        Value::SliceI32(v) | Value::SliceMutI32(v) => {
            format!("[{}]", v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","))
        }
        Value::SliceU8(v) => {
            format!("[{}]", v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","))
        }
    }
}

// ── Public entry point ────────────────────────────────────────────────────────

/// Compile the oracle binary for `target` and generate `count` random test cases.
pub fn generate_test_cases(
    binary: &Path,
    params: &[Param],
    return_type: &str,
    seed: u64,
    count: usize,
) -> Result<Vec<TestCase>, String> {
    let mut rng = Lcg::new(seed);
    let mut cases = Vec::with_capacity(count);
    let mut attempts = 0;

    while cases.len() < count && attempts < count * 20 {
        attempts += 1;
        let inputs = gen_inputs(params, &mut rng);
        match run_oracle(binary, &inputs, return_type, params) {
            Ok(output) => {
                let input_strs: Vec<String> = inputs.iter().map(value_to_test_string).collect();
                let expected = value_to_test_string(&output);
                println!("  [{}] -> {}", input_strs.join(", "), expected);
                cases.push(TestCase {
                    inputs: input_strs,
                    expected_output: expected,
                });
            }
            Err(_) => {} // skip (e.g. C++ UB / crash on this input)
        }
    }

    if cases.is_empty() {
        Err("Oracle produced no successful outputs for any random input".into())
    } else {
        Ok(cases)
    }
}
