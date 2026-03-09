use serde::Deserialize;
use std::path::Path;
use crate::eval::Value;

#[derive(Deserialize, Clone, Debug)]
pub struct Param {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct TestCase {
    pub inputs: Vec<String>,
    pub expected_output: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Target {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: String,
    pub example_rust: Option<String>,
    pub test_cases: Vec<TestCase>,
}

pub fn load_target(dataset_dir: &Path, name: &str) -> Result<Target, String> {
    let path = dataset_dir.join(format!("{}.json", name));
    if !path.exists() {
        let available: Vec<String> = std::fs::read_dir(dataset_dir)
            .map(|rd| rd.filter_map(|e| {
                let e = e.ok()?;
                let n = e.file_name().into_string().ok()?;
                n.ends_with(".json").then(|| n[..n.len()-5].to_string())
            }).collect())
            .unwrap_or_default();
        let mut available = available;
        available.sort();
        return Err(format!("Target '{}' not found in {:?}\nAvailable: {}", name, dataset_dir, available.join(", ")));
    }
    let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

pub fn load_symbols(path: &Path) -> Result<Vec<String>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    Ok(text.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| l.to_string())
        .collect())
}

pub fn parse_env(test_case: &TestCase, params: &[Param]) -> crate::eval::Env {
    params.iter().zip(test_case.inputs.iter()).map(|(param, input)| {
        let val = match param.ty.as_str() {
            "bool" => Value::Bool(input == "true"),
            _ => Value::Int(input.parse::<i32>().expect("invalid i32 input")),
        };
        (param.name.clone(), val)
    }).collect()
}
