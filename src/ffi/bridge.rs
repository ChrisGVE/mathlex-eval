//! swift-bridge declarations for the FFI boundary.
//!
//! These declarations generate the C headers and Swift bindings
//! that allow Swift code to call into Rust.

// swift-bridge macro generates identity casts that clippy flags
#![allow(clippy::unnecessary_cast)]
#[swift_bridge::bridge]
mod ffi {
    extern "Rust" {
        type RustCompiledExpr;
        type RustEvalHandle;
        type RustEvalIter;

        fn ffi_compile(ast_json: &str, constants_json: &str) -> Result<RustCompiledExpr, String>;

        fn ffi_eval_json(
            expr: &RustCompiledExpr,
            args_json: &str,
        ) -> Result<RustEvalHandle, String>;

        // CompiledExpr introspection
        fn ffi_argument_names(expr: &RustCompiledExpr) -> Vec<String>;
        fn ffi_is_complex(expr: &RustCompiledExpr) -> bool;

        // EvalHandle consumption
        fn ffi_shape(handle: &RustEvalHandle) -> Vec<i64>;
        fn ffi_len(handle: &RustEvalHandle) -> i64;
        fn ffi_scalar_json(handle: RustEvalHandle) -> Result<String, String>;
        fn ffi_to_array_json(handle: RustEvalHandle) -> Result<String, String>;
        fn ffi_into_iter(handle: RustEvalHandle) -> RustEvalIter;

        // EvalIter streaming
        fn ffi_iter_next(iter: &mut RustEvalIter) -> Option<String>;
    }
}

// --- Bridge type wrappers ---

use crate::{CompiledExpr, EvalHandle, EvalIter};

pub struct RustCompiledExpr(CompiledExpr);
pub struct RustEvalHandle(Option<EvalHandle>);
pub struct RustEvalIter(EvalIter);

fn ffi_compile(ast_json: &str, constants_json: &str) -> Result<RustCompiledExpr, String> {
    super::compile_from_json(ast_json, constants_json).map(RustCompiledExpr)
}

fn ffi_eval_json(expr: &RustCompiledExpr, args_json: &str) -> Result<RustEvalHandle, String> {
    super::eval_with_json(&expr.0, args_json).map(|h| RustEvalHandle(Some(h)))
}

fn ffi_argument_names(expr: &RustCompiledExpr) -> Vec<String> {
    super::expr_argument_names(&expr.0)
}

fn ffi_is_complex(expr: &RustCompiledExpr) -> bool {
    super::expr_is_complex(&expr.0)
}

fn ffi_shape(handle: &RustEvalHandle) -> Vec<i64> {
    handle
        .0
        .as_ref()
        .map(super::handle_shape)
        .unwrap_or_default()
}

fn ffi_len(handle: &RustEvalHandle) -> i64 {
    handle.0.as_ref().map(super::handle_len).unwrap_or(0)
}

fn ffi_scalar_json(mut handle: RustEvalHandle) -> Result<String, String> {
    let h = handle
        .0
        .take()
        .ok_or_else(|| "handle already consumed".to_string())?;
    super::handle_scalar_json(h)
}

fn ffi_to_array_json(mut handle: RustEvalHandle) -> Result<String, String> {
    let h = handle
        .0
        .take()
        .ok_or_else(|| "handle already consumed".to_string())?;
    super::handle_to_array_json(h)
}

fn ffi_into_iter(mut handle: RustEvalHandle) -> RustEvalIter {
    let h = handle.0.take().expect("handle already consumed");
    RustEvalIter(super::handle_into_iter(h))
}

fn ffi_iter_next(iter: &mut RustEvalIter) -> Option<String> {
    super::iter_next_result(&mut iter.0)
}
