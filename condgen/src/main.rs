use std::{env, fs, path::Path, process};

use serde::{Deserialize, Serialize};

// ── processed/ JSON types ────────────────────────────────────────────────────

#[derive(Deserialize)]
struct Param {
    name: String,
    #[serde(rename = "type")]
    ty: String,
}

#[derive(Deserialize)]
struct Processed {
    name: String,
    params: Vec<Param>,
    return_type: String,
}

// ── OpenAI types ─────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    response_format: ResponseFormat,
}

#[derive(Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    kind: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: AssistantMessage,
}

#[derive(Deserialize)]
struct AssistantMessage {
    content: String,
}

#[derive(Deserialize)]
struct SpecParam {
    name: String,
    #[serde(rename = "type")]
    ty: String,
}

#[derive(Deserialize)]
struct SpecFn {
    name: String,
    params: Vec<SpecParam>,
    return_type: String,
    decreases: String,
    body: String,
}

#[derive(Deserialize)]
struct Conditions {
    requires: Vec<String>,
    ensures: Vec<String>,
    #[serde(default)]
    spec_fns: Vec<SpecFn>,
}

// ── arithmetic evaluator ─────────────────────────────────────────────────────

fn type_max(ty: &str) -> Option<i128> {
    match ty {
        "i8" => Some(i8::MAX as i128),
        "u8" => Some(u8::MAX as i128),
        "i16" => Some(i16::MAX as i128),
        "u16" => Some(u16::MAX as i128),
        "i32" => Some(i32::MAX as i128),
        "u32" => Some(u32::MAX as i128),
        "i64" => Some(i64::MAX as i128),
        "u64" => Some(u64::MAX as i128),
        "isize" => Some(isize::MAX as i128),
        "usize" => Some(usize::MAX as i128),
        _ => None,
    }
}

fn type_min(ty: &str) -> Option<i128> {
    match ty {
        "i8" => Some(i8::MIN as i128),
        "i16" => Some(i16::MIN as i128),
        "i32" => Some(i32::MIN as i128),
        "i64" => Some(i64::MIN as i128),
        "isize" => Some(isize::MIN as i128),
        "u8" | "u16" | "u32" | "u64" | "usize" => Some(0),
        _ => None,
    }
}

struct Parser<'a> {
    src: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(s: &'a str) -> Self {
        Self { src: s.as_bytes(), pos: 0 }
    }

    fn skip_ws(&mut self) {
        while self.pos < self.src.len() && self.src[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }

    fn peek(&mut self) -> Option<u8> {
        self.skip_ws();
        self.src.get(self.pos).copied()
    }

    fn eat(&mut self) -> u8 {
        let b = self.src[self.pos];
        self.pos += 1;
        b
    }

    fn parse_ident(&mut self) -> Option<&'a str> {
        self.skip_ws();
        let start = self.pos;
        while self.pos < self.src.len()
            && (self.src[self.pos].is_ascii_alphanumeric() || self.src[self.pos] == b'_')
        {
            self.pos += 1;
        }
        if self.pos == start {
            return None;
        }
        std::str::from_utf8(&self.src[start..self.pos]).ok()
    }

    fn parse_atom(&mut self) -> Option<i128> {
        self.skip_ws();
        match self.peek()? {
            b'(' => {
                self.eat();
                let val = self.parse_expr()?;
                self.skip_ws();
                if self.peek() == Some(b')') {
                    self.eat();
                }
                Some(val)
            }
            b'0'..=b'9' => {
                let start = self.pos;
                while self.pos < self.src.len() && self.src[self.pos].is_ascii_digit() {
                    self.pos += 1;
                }
                std::str::from_utf8(&self.src[start..self.pos]).ok()?.parse().ok()
            }
            _ => {
                let save = self.pos;
                let ty = self.parse_ident()?;
                self.skip_ws();
                if self.src[self.pos..].starts_with(b"::") {
                    self.pos += 2;
                    let suffix = self.parse_ident()?;
                    match suffix {
                        "MAX" => type_max(ty),
                        "MIN" => type_min(ty),
                        _ => { self.pos = save; None }
                    }
                } else {
                    self.pos = save;
                    None
                }
            }
        }
    }

    // ^ is right-associative, highest precedence
    fn parse_power(&mut self) -> Option<i128> {
        let base = self.parse_atom()?;
        self.skip_ws();
        if self.peek() == Some(b'^') {
            self.eat();
            let exp = self.parse_power()?;
            Some(i128::pow(base, u32::try_from(exp).ok()?))
        } else {
            Some(base)
        }
    }

    fn parse_term(&mut self) -> Option<i128> {
        let mut val = self.parse_power()?;
        loop {
            self.skip_ws();
            match self.peek() {
                Some(b'*') => { self.eat(); val = val.checked_mul(self.parse_power()?)?; }
                Some(b'/') => { self.eat(); let rhs = self.parse_power()?; if rhs == 0 { return None; } val /= rhs; }
                _ => break,
            }
        }
        Some(val)
    }

    fn parse_expr(&mut self) -> Option<i128> {
        let mut val = self.parse_term()?;
        loop {
            self.skip_ws();
            match self.peek() {
                Some(b'+') => { self.eat(); val = val.checked_add(self.parse_term()?)?; }
                Some(b'-') => { self.eat(); val = val.checked_sub(self.parse_term()?)?; }
                _ => break,
            }
        }
        Some(val)
    }

    fn is_done(&mut self) -> bool {
        self.skip_ws();
        self.pos >= self.src.len()
    }
}

/// Try to evaluate a string as a pure arithmetic expression (no variables).
fn try_eval(expr: &str) -> Option<i128> {
    let mut p = Parser::new(expr);
    let val = p.parse_expr()?;
    if p.is_done() { Some(val) } else { None }
}

/// For `LHS <= EXPR` or `LHS >= EXPR`, evaluate EXPR if it's pure arithmetic.
fn eval_bounds(condition: &str) -> String {
    for op in ["<=", ">="] {
        if let Some(idx) = condition.find(op) {
            let rhs = condition[idx + op.len()..].trim();
            if let Some(val) = try_eval(rhs) {
                let lhs = condition[..idx + op.len()].trim_end();
                return format!("{lhs} {val}");
            }
        }
    }
    condition.to_string()
}

// ── helpers ──────────────────────────────────────────────────────────────────

fn is_numeric(ty: &str) -> bool {
    matches!(
        ty,
        "i8" | "u8"
            | "i16"
            | "u16"
            | "i32"
            | "u32"
            | "i64"
            | "u64"
            | "i128"
            | "u128"
            | "isize"
            | "usize"
    )
}

fn slice_params(params: &[Param]) -> Vec<&str> {
    params
        .iter()
        .filter(|p| p.ty.starts_with("&["))
        .map(|p| p.name.as_str())
        .collect()
}

/// Translate a single C++ condition expression to Verus syntax.
/// `slices`: names of params that are Rust slices (&[T])
/// `result_as_int`: whether to rewrite bare `result` → `result as int`
fn translate_expr(expr: &str, slices: &[&str], result_as_int: bool) -> String {
    let mut out = expr.to_string();

    // a.size() / a.length() / a.len() → a@.len()
    for &s in slices {
        for method in &["size()", "length()", "len()"] {
            out = out.replace(&format!("{s}.{method}"), &format!("{s}@.len()"));
        }
        // a[i] → a@[i as int]  (only when followed by [...])
        // Use a simple left-to-right scan to avoid double-replacing.
        out = rewrite_index(&out, s);
    }

    // result → result as int  (only in ensures, for numeric return types)
    if result_as_int {
        out = replace_result_token(&out);
    }

    // evaluate arithmetic bound expressions: `<= TYPE::MAX / (...)` → `<= 33025`
    out = eval_bounds(&out);

    out
}

/// Replace `s[...]` with `s@[... as int]` for a specific slice name.
fn rewrite_index(expr: &str, slice: &str) -> String {
    let pattern = format!("{slice}[");
    let mut result = String::new();
    let mut remaining = expr;
    while let Some(pos) = remaining.find(&pattern) {
        result.push_str(&remaining[..pos]);
        result.push_str(&format!("{slice}@["));
        remaining = &remaining[pos + pattern.len()..];
        // find the matching `]`
        if let Some(end) = remaining.find(']') {
            let inner = &remaining[..end];
            result.push_str(&format!("{inner} as int]"));
            remaining = &remaining[end + 1..];
        }
    }
    result.push_str(remaining);
    result
}

fn is_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Replace standalone `result` token with `result as int`,
/// skipping if the LLM already wrote `result as int`.
fn replace_result_token(expr: &str) -> String {
    let token = b"result";
    let bytes = expr.as_bytes();
    let mut out = String::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i..].starts_with(token) {
            let before_ok = i == 0 || !is_ident_char(bytes[i - 1]);
            let after = i + token.len();
            let after_ok = after >= bytes.len() || !is_ident_char(bytes[after]);
            if before_ok && after_ok {
                // check if already followed by `as int`
                let suffix = expr[after..].trim_start();
                if suffix.starts_with("as int") {
                    // pass through unchanged up to end of `as int`
                    let skip_to = after + expr[after..].find("as int").unwrap() + "as int".len();
                    out.push_str(&expr[i..skip_to]);
                    i = skip_to;
                } else {
                    out.push_str("result as int");
                    i = after;
                }
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

fn rust_signature(p: &Processed) -> String {
    let params = p
        .params
        .iter()
        .map(|param| format!("{}: {}", param.name, param.ty))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "pub fn {}({}) -> (result: {})",
        p.name, params, p.return_type
    )
}

// ── OpenAI call ───────────────────────────────────────────────────────────────

const SYSTEM_PROMPT: &str = include_str!("../prompt.txt");

fn infer_conditions(cpp_source: &str, sig: &str, api_key: &str) -> anyhow::Result<Conditions> {
    let system = SYSTEM_PROMPT;

    let user =
        format!("C++ source:\n```cpp\n{cpp_source}```\n\nRust signature:\n```rust\n{sig}\n```");

    let req = ChatRequest {
        model: "@azure-1/gpt-4o".into(),
        messages: vec![
            Message {
                role: "system".into(),
                content: system.into(),
            },
            Message {
                role: "user".into(),
                content: user,
            },
        ],
        response_format: ResponseFormat {
            kind: "json_object".into(),
        },
    };

    let client = reqwest::blocking::Client::new();
    let raw = client
        .post("https://genai.vocareum.com/v1/chat/completions")
        .bearer_auth(api_key)
        .json(&req)
        .send()?
        .text()?;

    let resp: ChatResponse = serde_json::from_str(&raw)
        .map_err(|e| anyhow::anyhow!("parse error: {e}\nraw response: {raw}"))?;

    let content = &resp.choices[0].message.content;
    Ok(serde_json::from_str(content)?)
}

// ── rendering ─────────────────────────────────────────────────────────────────

fn render_spec_fn(sf: &SpecFn) -> String {
    let params = sf
        .params
        .iter()
        .map(|p| format!("{}: {}", p.name, p.ty))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "pub open spec fn {}({}) -> {}\n    decreases {}\n{{\n    {}\n}}\n",
        sf.name, params, sf.return_type, sf.decreases, sf.body
    )
}

fn render(processed: &Processed, conditions: &Conditions) -> String {
    let slices = slice_params(&processed.params);
    let numeric = is_numeric(&processed.return_type);

    let spec_fns: Vec<String> = conditions.spec_fns.iter().map(render_spec_fn).collect();

    let requires: Vec<String> = conditions
        .requires
        .iter()
        .map(|c| format!("        {},", translate_expr(c, &slices, false)))
        .collect();

    let ensures: Vec<String> = conditions
        .ensures
        .iter()
        .map(|c| format!("        {},", translate_expr(c, &slices, numeric)))
        .collect();

    let sig = rust_signature(processed);

    let spec_block = if spec_fns.is_empty() {
        String::new()
    } else {
        spec_fns.join("\n") + "\n"
    };

    format!(
        "use vstd::prelude::*;\n\nverus! {{\n\n{spec_block}{sig}\n    requires\n{}\n    ensures\n{}\n{{\n    assume(false);\n}}\n\n}} // verus!\n",
        requires.join("\n"),
        ensures.join("\n"),
    )
}

// ── main ──────────────────────────────────────────────────────────────────────

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        eprintln!("Usage: condgen <cpp_file> <processed_json> <output_rs>");
        process::exit(1);
    }

    let cpp_source = fs::read_to_string(&args[1]).unwrap_or_else(|e| {
        eprintln!("Error reading {}: {e}", args[1]);
        process::exit(1);
    });
    let processed_str = fs::read_to_string(&args[2]).unwrap_or_else(|e| {
        eprintln!("Error reading {}: {e}", args[2]);
        process::exit(1);
    });
    let output_path = &args[3];

    let processed: Processed = serde_json::from_str(&processed_str).unwrap_or_else(|e| {
        eprintln!("Error parsing {}: {e}", args[2]);
        process::exit(1);
    });

    let api_key = env::var("OPENAI_API_KEY").unwrap_or_else(|_| {
        eprintln!("OPENAI_API_KEY not set");
        process::exit(1);
    });

    let sig = rust_signature(&processed);
    println!("Inferring conditions for {} ...", processed.name);

    let conditions = infer_conditions(&cpp_source, &sig, &api_key).unwrap_or_else(|e| {
        eprintln!("OpenAI error: {e}");
        process::exit(1);
    });

    if let Some(parent) = Path::new(output_path).parent() {
        fs::create_dir_all(parent).ok();
    }

    let output = render(&processed, &conditions);
    fs::write(output_path, &output).unwrap_or_else(|e| {
        eprintln!("Error writing {output_path}: {e}");
        process::exit(1);
    });

    println!("Written: {output_path}");
}
