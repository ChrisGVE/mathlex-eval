# mathlex-eval

[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![CI](https://github.com/ChrisGVE/mathlex-eval/actions/workflows/ci.yml/badge.svg)](https://github.com/ChrisGVE/mathlex-eval/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/mathlex-eval.svg)](https://crates.io/crates/mathlex-eval)
[![docs.rs](https://docs.rs/mathlex-eval/badge.svg)](https://docs.rs/mathlex-eval)
[![Rust version](https://img.shields.io/badge/rust-1.85%2B-orange.svg)](https://www.rust-lang.org)

Numerical evaluator for mathematical expression ASTs produced by [mathlex](https://crates.io/crates/mathlex). Compile once with constants, evaluate many times with different arguments. Supports N-dimensional broadcasting via Cartesian product semantics.

## Quick start

Add to `Cargo.toml`:

```toml
[dependencies]
mathlex-eval = "0.1"
mathlex = "0.3"
```

Compile and evaluate an expression:

```rust
use std::collections::HashMap;
use mathlex::{BinaryOp, Expression};
use mathlex_eval::{compile, eval, EvalInput};

// Build AST for: 2*x + 3
let ast = Expression::Binary {
    op: BinaryOp::Add,
    left: Box::new(Expression::Binary {
        op: BinaryOp::Mul,
        left: Box::new(Expression::Integer(2)),
        right: Box::new(Expression::Variable("x".into())),
    }),
    right: Box::new(Expression::Integer(3)),
};

// Compile with no constants
let compiled = compile(&ast, &HashMap::new()).unwrap();

// Evaluate with x = 5
let mut args = HashMap::new();
args.insert("x", EvalInput::Scalar(5.0));
let result = eval(&compiled, args).unwrap().scalar().unwrap();
// result = Real(13.0)
```

## Broadcasting

Array arguments produce Cartesian product outputs. Scalars broadcast to all positions.

```rust
use mathlex_eval::EvalInput;

let mut args = HashMap::new();
args.insert("x", EvalInput::from(vec![1.0, 2.0, 3.0]));
args.insert("y", EvalInput::from(vec![10.0, 20.0]));

let handle = eval(&compiled, args).unwrap();
// handle.shape() == [3, 2]
// Output grid:
//          y=10    y=20
//   x=1  [   11,     21  ]
//   x=2  [   14,     24  ]
//   x=3  [   19,     29  ]
```

Three consumption modes:

- `handle.scalar()` — 0-d result, errors if output is not scalar
- `handle.to_array()` — eager N-d `ArrayD<NumericResult>`
- `handle.iter()` — lazy per-element streaming

## Complex numbers

Real inputs produce real results. Complex promotion happens automatically when needed:

- Imaginary unit constant (`MathConstant::I`)
- `sqrt` of negative numbers
- `ln` of negative numbers
- `asin`/`acos` outside `[-1, 1]`
- Complex `EvalInput` arguments

## Architecture

Two-phase compile/eval:

1. **Compile** — validate AST, substitute constants, resolve variables, fold constant subexpressions
2. **Evaluate** — substitute arguments, compute results with broadcasting

This separation enables constant folding at compile time and efficient repeated evaluation.

## Supported expressions

Accepted AST variants: `Integer`, `Float`, `Rational`, `Complex`, `Variable`, `Constant`, `Binary` (add, sub, mul, div, pow, mod), `Unary` (neg, factorial), `Function` (23 built-in), `Sum`, `Product`.

All other variants (derivatives, integrals, limits, vectors, matrices, etc.) return `CompileError::UnsupportedExpression`.

### Built-in functions

sin, cos, tan, asin, acos, atan, atan2, sinh, cosh, tanh, exp, ln, log2, log10, log, sqrt, cbrt, abs, floor, ceil, round, min, max

## Feature flags

| Flag | Default | Description |
|------|---------|-------------|
| `serde` | yes | Serialize/Deserialize for `CompiledExpr` and `NumericResult` |
| `parallel` | no | Rayon-based parallel broadcasting in `to_array()` |
| `ffi` | no | Swift FFI via swift-bridge |

## API reference

| Type | Role |
|------|------|
| `compile()` | Compile AST with constants into `CompiledExpr` |
| `eval()` | Create lazy eval handle from `CompiledExpr` + arguments |
| `CompiledExpr` | Opaque compiled expression |
| `EvalHandle` | Lazy result handle (consume via `scalar`/`to_array`/`iter`) |
| `EvalIter` | Streaming result iterator |
| `NumericResult` | Real or Complex result value |
| `EvalInput` | Argument value: scalar, array, or iterator |
| `CompileError` | Compilation failure (7 variants) |
| `EvalError` | Evaluation failure (5 variants) |

Full documentation: [docs.rs/mathlex-eval](https://docs.rs/mathlex-eval)

## License

MIT
