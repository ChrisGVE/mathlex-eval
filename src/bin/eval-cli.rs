//! Toy CLI for mathlex-eval. Reads from stdin, writes results to stdout, errors to stderr.
//!
//! # Input format
//!
//! Lines are prefixed with `expr:`, `subst:`, or `var:`.
//! Blank lines and `#` comments are skipped.
//! A block is terminated by a blank line or EOF.
//!
//! ## Expression
//! ```text
//! expr: 2*x + 3
//! expr: sin(x^2) + cos(y)
//! expr: \frac{x}{2} + 1         # LaTeX (prefix with latex:)
//! ```
//!
//! ## Constant substitution (optional)
//! Python-style dict:
//! ```text
//! subst: {a: 2.5, b: 3.14}
//! ```
//!
//! ## Variables
//! Python-style dict with scalars or arrays:
//! ```text
//! var: {x: 5.0, y: 10.0}
//! var: {x: [1, 2, 3], y: [10, 20]}
//! ```
//!
//! Single-variable shorthand (when compiled expression has exactly one argument):
//! ```text
//! var: 5.0
//! var: [1, 2, 3, 4, 5]
//! ```
//!
//! # Example file
//! ```text
//! expr: 2*x + 3
//! var: {x: [1, 2, 3]}
//!
//! expr: a*x^2 + b
//! subst: {a: 2, b: -1}
//! var: {x: [0, 1, 2, 3]}
//! ```

use std::collections::HashMap;
use std::io::{self, BufRead};

use mathlex_eval::{EvalInput, NumericResult, compile, eval};

fn main() {
    let stdin = io::stdin();
    let mut block = Block::default();

    for (line_num, line) in stdin.lock().lines().enumerate() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                eprintln!("error: reading stdin line {}: {}", line_num + 1, e);
                std::process::exit(1);
            }
        };

        let trimmed = line.trim();

        // Skip comments
        if trimmed.starts_with('#') {
            continue;
        }

        // Blank line = execute block
        if trimmed.is_empty() {
            if block.has_expr() {
                execute_block(&block);
                block = Block::default();
            }
            continue;
        }

        // Parse prefixed lines
        if let Some(rest) = strip_prefix(trimmed, "expr:") {
            block.expr = Some(rest.to_string());
        } else if let Some(rest) = strip_prefix(trimmed, "latex:") {
            block.latex = Some(rest.to_string());
        } else if let Some(rest) = strip_prefix(trimmed, "subst:") {
            block.subst = Some(rest.to_string());
        } else if let Some(rest) = strip_prefix(trimmed, "var:") {
            block.var = Some(rest.to_string());
        } else {
            eprintln!("error: line {}: unknown prefix: {}", line_num + 1, trimmed);
        }
    }

    // Execute remaining block at EOF
    if block.has_expr() {
        execute_block(&block);
    }
}

#[derive(Default)]
struct Block {
    expr: Option<String>,
    latex: Option<String>,
    subst: Option<String>,
    var: Option<String>,
}

impl Block {
    fn has_expr(&self) -> bool {
        self.expr.is_some() || self.latex.is_some()
    }
}

fn execute_block(block: &Block) {
    // Parse expression
    let ast = if let Some(ref expr_str) = block.expr {
        match mathlex::parse(expr_str) {
            Ok(ast) => ast,
            Err(e) => {
                eprintln!("error: parse failed: {}", e);
                return;
            }
        }
    } else if let Some(ref latex_str) = block.latex {
        match mathlex::parse_latex(latex_str) {
            Ok(ast) => ast,
            Err(e) => {
                eprintln!("error: latex parse failed: {}", e);
                return;
            }
        }
    } else {
        return;
    };

    // Parse substitutions
    let constants = match &block.subst {
        Some(s) => match parse_dict(s) {
            Ok(dict) => dict
                .into_iter()
                .map(|(k, v)| (k, scalar_to_numeric(&v)))
                .collect::<Vec<_>>(),
            Err(e) => {
                eprintln!("error: subst parse failed: {}", e);
                return;
            }
        },
        None => Vec::new(),
    };
    let constants_map: HashMap<&str, NumericResult> =
        constants.iter().map(|(k, v)| (k.as_str(), *v)).collect();

    // Compile
    let compiled = match compile(&ast, &constants_map) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: compile failed: {}", e);
            return;
        }
    };

    let arg_names = compiled.argument_names();

    // Parse variables
    let args: HashMap<&str, EvalInput> = match &block.var {
        Some(var_str) => match parse_var(var_str.trim(), arg_names) {
            Ok(a) => a,
            Err(e) => {
                eprintln!("error: var parse failed: {}", e);
                return;
            }
        },
        None => HashMap::new(),
    };

    // Evaluate
    let handle = match eval(&compiled, args) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("error: eval failed: {}", e);
            return;
        }
    };

    // Output
    let expr_label = block
        .expr
        .as_deref()
        .or(block.latex.as_deref())
        .unwrap_or("?");

    if handle.shape().is_empty() {
        // Scalar
        match handle.scalar() {
            Ok(v) => println!("{} = {}", expr_label, fmt_result(&v)),
            Err(e) => eprintln!("error: {}", e),
        }
    } else {
        // Array
        println!("{} [shape: {:?}]", expr_label, handle.shape());
        for result in handle.iter() {
            match result {
                Ok(v) => println!("  {}", fmt_result(&v)),
                Err(e) => println!("  ERROR: {}", e),
            }
        }
    }
}

fn fmt_result(r: &NumericResult) -> String {
    match r {
        NumericResult::Real(v) => format!("{v}"),
        NumericResult::Complex(c) => {
            if c.im >= 0.0 {
                format!("{} + {}i", c.re, c.im)
            } else {
                format!("{} - {}i", c.re, -c.im)
            }
        }
    }
}

// --- Parsing helpers ---

fn strip_prefix<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    s.strip_prefix(prefix).map(|r| r.trim())
}

/// Parse a Python-style dict: `{k: v, k2: v2}` or `{k: [1,2,3], k2: 5}`
/// Values can be f64 or `[f64, ...]`.
fn parse_dict(s: &str) -> Result<Vec<(String, Value)>, String> {
    let s = s.trim();
    let s = s
        .strip_prefix('{')
        .and_then(|s| s.strip_suffix('}'))
        .ok_or("expected {key: value, ...}")?
        .trim();

    if s.is_empty() {
        return Ok(Vec::new());
    }

    let mut result = Vec::new();
    let mut rest = s;

    while !rest.is_empty() {
        // Parse key
        let colon_pos = rest
            .find(':')
            .ok_or_else(|| format!("expected ':' after key in '{rest}'"))?;
        let key = rest[..colon_pos].trim().to_string();
        rest = rest[colon_pos + 1..].trim();

        // Parse value
        let (value, consumed) = parse_value(rest)?;
        result.push((key, value));
        rest = rest[consumed..].trim();

        // Skip comma
        if let Some(r) = rest.strip_prefix(',') {
            rest = r.trim();
        }
    }

    Ok(result)
}

#[derive(Debug)]
enum Value {
    Scalar(f64),
    Array(Vec<f64>),
}

fn scalar_to_numeric(v: &Value) -> NumericResult {
    match v {
        Value::Scalar(f) => NumericResult::Real(*f),
        Value::Array(a) => NumericResult::Real(a[0]), // for subst, take first
    }
}

fn parse_value(s: &str) -> Result<(Value, usize), String> {
    let s = s.trim_start();
    if s.starts_with('[') {
        // Array
        let bracket_end = s.find(']').ok_or("unclosed '[' in array value")?;
        let inner = s[1..bracket_end].trim();
        let values: Result<Vec<f64>, String> = inner
            .split(',')
            .filter(|p| !p.trim().is_empty())
            .map(|p| {
                p.trim()
                    .parse::<f64>()
                    .map_err(|e| format!("invalid number '{}': {}", p.trim(), e))
            })
            .collect();
        Ok((Value::Array(values?), bracket_end + 1))
    } else {
        // Scalar — read until comma, closing brace, or end
        let end = s.find([',', '}']).unwrap_or(s.len());
        let num_str = s[..end].trim();
        let val = num_str
            .parse::<f64>()
            .map_err(|e| format!("invalid number '{}': {}", num_str, e))?;
        Ok((Value::Scalar(val), end))
    }
}

/// Parse variable specification.
/// Either a dict `{x: 5, y: [1,2]}` or shorthand `5.0` / `[1,2,3]`.
fn parse_var<'a>(s: &str, arg_names: &'a [String]) -> Result<HashMap<&'a str, EvalInput>, String> {
    let s = s.trim();
    let mut args = HashMap::new();

    if s.starts_with('{') {
        // Dict form
        let dict = parse_dict(s)?;
        for (key, value) in dict {
            let name = arg_names
                .iter()
                .find(|n| **n == key)
                .ok_or_else(|| format!("unknown variable '{key}'"))?;
            args.insert(name.as_str(), value_to_input(value));
        }
    } else if arg_names.len() == 1 {
        // Shorthand: single variable
        let (value, _) = parse_value(s)?;
        args.insert(arg_names[0].as_str(), value_to_input(value));
    } else if arg_names.is_empty() {
        // No variables needed — ignore var line
        if !s.is_empty() {
            eprintln!("warning: var provided but expression has no free variables");
        }
    } else {
        return Err(format!(
            "multiple variables ({}) require dict syntax: {{x: ..., y: ...}}",
            arg_names.join(", ")
        ));
    }

    Ok(args)
}

fn value_to_input(v: Value) -> EvalInput {
    match v {
        Value::Scalar(f) => EvalInput::Scalar(f),
        Value::Array(a) => EvalInput::from(a),
    }
}
