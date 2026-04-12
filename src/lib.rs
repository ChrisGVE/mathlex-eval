//! # mathlex-eval
//!
//! Numerical evaluator for mathematical expression ASTs produced by
//! [`mathlex`](https://crates.io/crates/mathlex).
//!
//! Compiles a mathlex `Expression` AST into an efficient internal representation,
//! then evaluates it with variable substitution and N-dimensional broadcasting.
//!
//! ## Two-phase architecture
//!
//! 1. **Compile** — [`compile()`] validates the AST, substitutes constants, resolves
//!    variables, and folds constant subexpressions into an optimized [`CompiledExpr`].
//! 2. **Evaluate** — [`eval()`] creates a lazy [`EvalHandle`] that computes output
//!    shape from input shapes but defers computation until consumed via
//!    [`scalar()`](EvalHandle::scalar), [`to_array()`](EvalHandle::to_array), or
//!    [`iter()`](EvalHandle::iter).
//!
//! ## Quick start
//!
//! ```rust
//! use std::collections::HashMap;
//! use mathlex::{BinaryOp, Expression};
//! use mathlex_eval::{compile, eval, EvalInput};
//!
//! // Build AST for: 2*x + 3
//! let ast = Expression::Binary {
//!     op: BinaryOp::Add,
//!     left: Box::new(Expression::Binary {
//!         op: BinaryOp::Mul,
//!         left: Box::new(Expression::Integer(2)),
//!         right: Box::new(Expression::Variable("x".into())),
//!     }),
//!     right: Box::new(Expression::Integer(3)),
//! };
//!
//! let compiled = compile(&ast, &HashMap::new()).unwrap();
//!
//! let mut args = HashMap::new();
//! args.insert("x", EvalInput::Scalar(5.0));
//! let result = eval(&compiled, args).unwrap().scalar().unwrap();
//! assert_eq!(result.to_f64(), Some(13.0));
//! ```
//!
//! ## Feature flags
//!
//! | Flag | Default | Description |
//! |------|---------|-------------|
//! | `serde` | yes | Serialize/Deserialize for [`CompiledExpr`] and [`NumericResult`] |
//! | `parallel` | no | Rayon-based parallel broadcasting in [`EvalHandle::to_array()`] |
//! | `ffi` | no | Swift FFI bridge via swift-bridge |

pub(crate) mod broadcast;
pub mod compiler;
pub mod error;
pub mod eval;
#[cfg(feature = "ffi")]
pub mod ffi;

pub use compiler::compile;
pub use compiler::ir::CompiledExpr;
pub use error::{CompileError, EvalError};
pub use eval::{EvalHandle, EvalInput, EvalIter, NumericResult, eval};
