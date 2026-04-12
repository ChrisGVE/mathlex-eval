//! # mathlex-eval
//!
//! Numerical evaluator for mathematical expression ASTs produced by
//! [`mathlex`](https://crates.io/crates/mathlex).
//!
//! Compiles a mathlex `Expression` AST into an efficient internal representation,
//! then evaluates it with variable substitution and broadcasting support.

pub mod error;

pub use error::{CompileError, EvalError};
