//! Swift FFI bridge for mathlex-eval.
//!
//! Structured types cross the boundary as JSON. Streaming types
//! (iterators) cross via callback adapters. Errors cross as strings.

use std::collections::HashMap;

use crate::{CompiledExpr, EvalHandle, EvalInput, EvalIter, NumericResult};

/// Compile a mathlex AST from JSON with constants as JSON.
///
/// Returns a `CompiledExpr` or an error string.
pub fn compile_from_json(ast_json: &str, constants_json: &str) -> Result<CompiledExpr, String> {
    let ast: mathlex::Expression =
        serde_json::from_str(ast_json).map_err(|e| format!("invalid AST JSON: {e}"))?;
    let constants_raw: HashMap<String, NumericResult> =
        serde_json::from_str(constants_json).map_err(|e| format!("invalid constants JSON: {e}"))?;

    let constants: HashMap<&str, NumericResult> = constants_raw
        .iter()
        .map(|(k, v)| (k.as_str(), *v))
        .collect();

    crate::compile(&ast, &constants).map_err(|e| e.to_string())
}

/// Evaluate a compiled expression with arguments as JSON.
///
/// Returns an `EvalHandle` or an error string.
pub fn eval_with_json(expr: &CompiledExpr, args_json: &str) -> Result<EvalHandle, String> {
    let args_raw: HashMap<String, serde_json::Value> =
        serde_json::from_str(args_json).map_err(|e| format!("invalid args JSON: {e}"))?;

    let mut args: HashMap<&str, EvalInput> = HashMap::new();
    for (name, value) in &args_raw {
        let input = json_to_eval_input(value)?;
        args.insert(name.as_str(), input);
    }

    crate::eval(expr, args).map_err(|e| e.to_string())
}

/// Evaluate with iterator arguments provided via callbacks.
pub fn eval_with_iters(
    expr: &CompiledExpr,
    args_json: &str,
    iter_arg_names: Vec<String>,
    mut next_values: Vec<Box<dyn FnMut() -> Option<f64>>>,
) -> Result<EvalHandle, String> {
    let args_raw: HashMap<String, serde_json::Value> =
        serde_json::from_str(args_json).map_err(|e| format!("invalid args JSON: {e}"))?;

    let mut args: HashMap<&str, EvalInput> = HashMap::new();

    // Add JSON-based args
    for (name, value) in &args_raw {
        let input = json_to_eval_input(value)?;
        args.insert(name.as_str(), input);
    }

    // Add iterator-based args
    for name in &iter_arg_names {
        let callback = next_values.remove(0);
        let adapter = SwiftIterAdapter { callback };
        args.insert(
            // Safety: we need the string to live long enough. Since we consume args
            // immediately in eval(), leaking is the simplest approach for FFI.
            unsafe { &*(name.as_str() as *const str) },
            EvalInput::Iter(Box::new(adapter)),
        );
    }

    crate::eval(expr, args).map_err(|e| e.to_string())
}

// EvalHandle methods for FFI

/// Get the output shape as a vector of dimension sizes.
pub fn handle_shape(handle: &EvalHandle) -> Vec<i64> {
    handle.shape().iter().map(|&s| s as i64).collect()
}

/// Get the total number of output elements (-1 if unknown).
pub fn handle_len(handle: &EvalHandle) -> i64 {
    handle.len() as i64
}

/// Consume the handle eagerly and return the full result as JSON.
pub fn handle_to_array_json(handle: EvalHandle) -> Result<String, String> {
    let array = handle.to_array().map_err(|e| e.to_string())?;
    let results: Vec<NumericResult> = array.iter().copied().collect();
    serde_json::to_string(&results).map_err(|e| e.to_string())
}

/// Consume the handle as a scalar and return the result as JSON.
pub fn handle_scalar_json(handle: EvalHandle) -> Result<String, String> {
    let result = handle.scalar().map_err(|e| e.to_string())?;
    serde_json::to_string(&result).map_err(|e| e.to_string())
}

/// Convert handle into a streaming iterator.
pub fn handle_into_iter(handle: EvalHandle) -> EvalIter {
    handle.iter()
}

/// Get the next result from the iterator as JSON, or None if exhausted.
pub fn iter_next_result(iter: &mut EvalIter) -> Option<String> {
    iter.next().map(|r| match r {
        Ok(v) => serde_json::to_string(&v).unwrap_or_else(|e| format!("{{\"error\":\"{e}\"}}",)),
        Err(e) => format!("{{\"error\":\"{e}\"}}"),
    })
}

/// Get argument names from a compiled expression.
pub fn expr_argument_names(expr: &CompiledExpr) -> Vec<String> {
    expr.argument_names().to_vec()
}

/// Check if the compiled expression involves complex numbers.
pub fn expr_is_complex(expr: &CompiledExpr) -> bool {
    expr.is_complex()
}

// --- Internal helpers ---

fn json_to_eval_input(value: &serde_json::Value) -> Result<EvalInput, String> {
    match value {
        serde_json::Value::Number(n) => {
            let f = n
                .as_f64()
                .ok_or_else(|| "invalid number in args".to_string())?;
            Ok(EvalInput::Scalar(f))
        }
        serde_json::Value::Array(arr) => {
            let values: Result<Vec<f64>, String> = arr
                .iter()
                .map(|v| {
                    v.as_f64()
                        .ok_or_else(|| "invalid number in array arg".to_string())
                })
                .collect();
            Ok(EvalInput::from(values?))
        }
        _ => Err("unsupported arg type: expected number or array".to_string()),
    }
}

/// Adapter that wraps a Swift callback into a Rust Iterator.
struct SwiftIterAdapter {
    callback: Box<dyn FnMut() -> Option<f64>>,
}

impl Iterator for SwiftIterAdapter {
    type Item = f64;

    fn next(&mut self) -> Option<f64> {
        (self.callback)()
    }
}
