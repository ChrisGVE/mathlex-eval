# mathlex-eval — Design Specification

## Overview

`mathlex-eval` is a numerical evaluator for mathematical expression ASTs produced by
[mathlex](https://crates.io/crates/mathlex). It compiles a mathlex `Expression` AST into
an efficient internal representation, then evaluates it with variable substitution and
N-dimensional broadcasting support.

The library follows a two-phase compile/eval architecture: compile once with constants,
evaluate many times with different arguments. This separation enables constant folding at
compile time and efficient repeated evaluation — the core performance requirement.

**Scope (v1):** Scalar arithmetic, built-in math functions, finite sums/products. Vector,
matrix, and tensor AST variants are deferred to v1.x but the architecture accommodates
them without breaking changes.

**Future requirement (not v1):** Chained/tree evaluation — a DAG of ASTs where outputs of
parent nodes feed into inputs of child nodes. This informs the concurrency model but is
not implemented in v1.

## Crate structure

Single crate with feature flags, matching `mathlex`'s pattern:

```
mathlex-eval/
  src/
    lib.rs              # Public API re-exports, crate docs
    error.rs            # CompileError, EvalError
    compiler/
      mod.rs            # compile() entry point
      validate.rs       # AST validation (reject non-numerical variants)
      fold.rs           # Constant folding, variable resolution, scope handling
      ir.rs             # CompiledExpr definition
    eval/
      mod.rs            # eval() entry point, EvalHandle
      scalar.rs         # Core scalar evaluation of CompiledExpr
      functions.rs      # Built-in math function dispatch (sin, cos, exp, log, ...)
      numeric.rs        # NumericResult type, real/complex promotion logic
    broadcast/
      mod.rs            # Broadcasting engine, index iteration, iterator caching
    ffi/                # (behind "ffi" feature flag)
      mod.rs            # swift-bridge definitions, JSON boundary, callback adapters
  swift/                # Swift convenience wrapper (SPM package)
  tests/
    compile_tests.rs
    eval_tests.rs
    broadcast_tests.rs
    complex_tests.rs
    error_tests.rs
  benches/
    benchmarks.rs
  examples/
    basic_eval.rs       # Compile and evaluate a simple expression
    broadcasting.rs     # Array and grid evaluation
    complex_numbers.rs  # Real/complex promotion
    iterator_input.rs   # Streaming evaluation with iterator arguments
  Cargo.toml
  Package.swift         # SPM manifest
  .spi.yml              # Swift Package Index config
```

### Feature flags

- `default = ["serde"]`
- `ffi` — Swift FFI via swift-bridge (enables `ffi/` module, build-dependency on
  swift-bridge-build)
- `serde` — Serialization support for CompiledExpr and NumericResult
- `parallel` — Rayon-based parallelism for broadcasting and concurrent evaluation

### Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| mathlex | 0.3 | AST types (Expression, BinaryOp, etc.) |
| num-complex | 0.4 | Complex<f64> arithmetic |
| ndarray | 0.16 | N-dimensional arrays for broadcasting results |
| thiserror | 2 | Ergonomic error type derivation |
| ordered-float | 4 | Interop with mathlex's MathFloat |
| swift-bridge | 0.1 | Swift FFI (optional, `ffi` feature) |
| rayon | 1.10 | Parallel evaluation (optional, `parallel` feature) |
| serde / serde_json | 1.0 | Serialization (optional, `serde` feature) |

Dev dependencies: `proptest`, `criterion`, `approx`.

## Architecture: Two-phase compile/eval

### Phase 1 — Compile

`compile(ast, constants) -> Result<CompiledExpr, CompileError>`

Takes a mathlex `Expression` reference and a constants map. Produces a `CompiledExpr` —
an optimized internal representation ready for repeated evaluation.

Three logical passes (validate and fold are separate concerns, fold produces the IR
directly):

**Pass 1 — Validate (compiler/validate.rs):**
Walk the AST and reject non-numerical variants with a clear error message.

Accepted variants: `Integer`, `Float`, `Rational`, `Complex`, `Variable`, `Constant`,
`Binary`, `Unary`, `Function`, `Sum`, `Product`.

All other variants produce `CompileError::UnsupportedExpression`.

**Pass 2 — Bind constants, resolve variables, fold (compiler/fold.rs):**

1. Replace `Variable(name)` with `Literal(value)` when name is in the constants map.
2. Resolve math constants: `Constant(Pi)` → `Literal(π)`, `Constant(E)` → `Literal(e)`,
   `Constant(ImaginaryUnit)` → `ComplexLiteral { re: 0.0, im: 1.0 }`.
3. Map remaining free `Variable(name)` to `Argument(index)` — index assigned by order of
   first appearance during AST traversal.
4. Recognize sum/product index variables as locally bound — push onto a scope stack,
   assign to `Index(slot)` separate from arguments.
5. Fold constant subexpressions: if both children of a Binary node are literals, evaluate
   immediately (e.g., `2 * π` → `Literal(6.283...)`).
6. Verify sum/product bounds resolve to concrete `i64` after folding — error if not.
7. Any remaining unresolved variable → `CompileError::UnresolvedVariable`.

Output is the `CompiledExpr` IR directly — no separate pass 3.

### Phase 2 — Evaluate

`eval(expr, args) -> Result<EvalHandle, EvalError>`

Takes a `CompiledExpr` reference and an arguments map. Returns an `EvalHandle` — a lazy
handle that computes the output shape from input shapes but defers actual evaluation until
the caller chooses a consumption mode.

No computation runs at `eval()` call time. The handle validates argument names and
computes the output shape, then waits.

## Core types

### CompiledExpr (compiler/ir.rs)

Internal IR. Opaque to callers — only inspectable via `argument_names()` and
`is_complex()`.

```rust
pub struct CompiledExpr {
    root: CompiledNode,
    argument_names: Vec<String>,
    is_complex: bool,
}

enum CompiledNode {
    // Leaf nodes
    Literal(f64),
    ComplexLiteral { re: f64, im: f64 },
    Argument(usize),    // index into argument values, resolved by name at compile time
    Index(usize),       // bound variable from sum/product, separate from arguments

    // Arithmetic
    Binary { op: BinaryOp, left: Box<CompiledNode>, right: Box<CompiledNode> },
    Unary { op: UnaryOp, operand: Box<CompiledNode> },

    // Built-in functions
    Function { kind: BuiltinFn, args: Vec<CompiledNode> },

    // Finite aggregation (bounds must be concrete i64 after folding)
    Sum { index: usize, lower: i64, upper: i64, body: Box<CompiledNode> },
    Product { index: usize, lower: i64, upper: i64, body: Box<CompiledNode> },
}
```

Design notes:
- `Argument(usize)` — positional index for fast lookup at eval time (no string matching).
- `Index(usize)` — separate from Argument because index variables get values from the
  sum/product loop, not from user input.
- `Literal` / `ComplexLiteral` — all constants and foldable subexpressions resolved at
  compile time.
- `BuiltinFn` — enum, not string. Type-safe dispatch at eval time.

### Internal operator/function enums (compiler/ir.rs, eval/functions.rs)

```rust
enum BinaryOp { Add, Sub, Mul, Div, Pow, Mod }
enum UnaryOp { Neg, Factorial }

enum BuiltinFn {
    Sin, Cos, Tan, Asin, Acos, Atan, Atan2,
    Sinh, Cosh, Tanh,
    Exp, Ln, Log2, Log10, Log,   // Log = log_base(value)
    Sqrt, Cbrt, Abs,
    Floor, Ceil, Round,
    Min, Max,
}
```

Subset of mathlex's operators — only numerically evaluable ones. Unrecognized function
names → `CompileError::UnknownFunction`. Conservative set for v1; new functions are
additive (non-breaking).

### NumericResult (eval/numeric.rs)

```rust
pub enum NumericResult {
    Real(f64),
    Complex(Complex<f64>),
}
```

Implements `From<f64>`, `From<Complex<f64>>`, and arithmetic operations that promote to
complex when needed (real + complex → complex, sqrt of negative → complex).

### EvalInput

```rust
pub enum EvalInput {
    Scalar(f64),
    Complex(Complex<f64>),
    Array(ArrayD<f64>),
    ComplexArray(ArrayD<Complex<f64>>),
    Iter(Box<dyn Iterator<Item = f64>>),
    ComplexIter(Box<dyn Iterator<Item = Complex<f64>>>),
}
```

Any number of arguments can be iterators. The broadcasting engine caches iterator values
internally (see Broadcasting section).

### EvalHandle (eval/mod.rs)

Lazy evaluation handle. Returned by `eval()`. Computes output shape from input shapes but
defers computation until consumed.

```rust
pub struct EvalHandle { /* internal */ }

impl EvalHandle {
    /// Output shape (empty for scalar, [n] for 1-D, [n,m] for 2-D, etc.)
    /// For iterator inputs, the corresponding axis is unknown until exhausted.
    pub fn shape(&self) -> &[Option<usize>];

    /// Total output elements (None if any iterator input has unknown length)
    pub fn len(&self) -> Option<usize>;

    /// Consume as scalar (errors if output is not 0-d)
    pub fn scalar(self) -> Result<NumericResult, EvalError>;

    /// Consume eagerly into a full N-dimensional array
    pub fn to_array(self) -> Result<ArrayD<NumericResult>, EvalError>;

    /// Consume lazily — yields results as they become computable
    pub fn iter(self) -> EvalIter;
}
```

When iterator inputs are present, `shape()` returns `None` for the corresponding axis
until the iterator is exhausted. `to_array()` blocks until all iterators are drained.
`iter()` streams results incrementally using the border computation strategy.

### EvalIter

```rust
pub struct EvalIter { /* internal */ }

impl Iterator for EvalIter {
    type Item = Result<NumericResult, EvalError>;
}
```

Yields one result per output element. `None` = exhausted. Errors are per-element (e.g.,
division by zero at a specific input combination) rather than failing the entire batch.

## Public API surface

Nine public types total:

| Type | Role |
|------|------|
| `compile()` | Compile AST with constants → CompiledExpr |
| `eval()` | Create lazy eval handle from CompiledExpr + arguments |
| `CompiledExpr` | Opaque compiled expression (inspectable: argument_names, is_complex) |
| `EvalHandle` | Lazy result handle (consume via scalar/to_array/iter) |
| `EvalIter` | Streaming result iterator |
| `NumericResult` | Real or Complex result value |
| `EvalInput` | Argument value: scalar, array, or iterator (real or complex) |
| `CompileError` | Compilation failure (7 variants) |
| `EvalError` | Evaluation failure (5 variants) |

### compile()

```rust
pub fn compile(
    ast: &Expression,
    constants: &HashMap<&str, NumericResult>,
) -> Result<CompiledExpr, CompileError>;
```

### eval()

```rust
pub fn eval(
    expr: &CompiledExpr,
    args: &HashMap<&str, EvalInput>,
) -> Result<EvalHandle, EvalError>;
```

### CompiledExpr (public interface)

```rust
impl CompiledExpr {
    pub fn argument_names(&self) -> &[String];
    pub fn is_complex(&self) -> bool;
}
```

## Error types

### CompileError

```rust
#[derive(Debug, thiserror::Error)]
pub enum CompileError {
    #[error("unsupported expression variant '{variant}': {context}")]
    UnsupportedExpression { variant: &'static str, context: String },

    #[error("unresolved variable '{name}'")]
    UnresolvedVariable { name: String },

    #[error("{construct} bounds must be integer: {bound}")]
    NonIntegerBounds { construct: &'static str, bound: String },

    #[error("unknown function '{name}'")]
    UnknownFunction { name: String },

    #[error("function '{function}' expects {expected} args, got {got}")]
    ArityMismatch { function: String, expected: usize, got: usize },

    #[error("division by zero during constant folding")]
    DivisionByZero,

    #[error("numeric overflow during constant folding: {context}")]
    NumericOverflow { context: String },
}
```

### EvalError

```rust
#[derive(Debug, thiserror::Error)]
pub enum EvalError {
    #[error("unknown argument '{name}'")]
    UnknownArgument { name: String },

    #[error("missing argument '{name}'")]
    MissingArgument { name: String },

    #[error("division by zero")]
    DivisionByZero,

    #[error("numeric overflow")]
    NumericOverflow,

    #[error("incompatible array shapes: {details}")]
    ShapeMismatch { details: String },
}
```

Two distinct error types — compile errors and eval errors happen at different times,
carry different context, and callers handle them differently.

## Broadcasting semantics

### Rules

Broadcasting follows Cartesian product semantics over argument arrays:

1. Each array argument contributes one axis to the output shape.
2. Scalar arguments contribute no axis (broadcast to all positions).
3. Axis ordering follows argument declaration order (order of first appearance in the AST
   during compilation, inspectable via `compiled.argument_names()`).
4. Output dimensions = product of all array lengths.

### Examples

`f(x, y) = x² + y` with two array arguments:

```
args = { "x": Array([1, 2, 3]),  "y": Array([10, 20]) }
Output shape: [3, 2]

         y=10    y=20
x=1  [   11,     21  ]
x=2  [   14,     24  ]
x=3  [   19,     29  ]
```

Mixed scalar + array:

```
args = { "x": Scalar(2.0),  "y": Array([10, 20, 30]) }
Output shape: [3]
→ [14, 24, 34]
```

All scalars:

```
args = { "x": Scalar(2.0),  "y": Scalar(10.0) }
Output shape: []  (0-d)
→ 14.0
```

### Iterator caching and border computation

When arguments include iterators, the broadcasting engine caches yielded values
incrementally. As each iterator yields a new value, the engine computes the "border" — the
new row/column of results that became computable.

```
x = Iter yields: 1, 2, 3
y = Iter yields: 10, 20

Step 1: x→1, y→10 → cache x=[1], y=[10]
  compute: f(1,10)                        ← initial cell

Step 2: x→2, y→20 → cache x=[1,2], y=[10,20]
  compute: f(2,10), f(1,20), f(2,20)      ← new border

Step 3: iterators exhausted → full grid cached
  (all cells already computed via border steps)
```

When consumed via `to_array()`, the engine blocks until all iterators are exhausted then
returns the full materialized array. When consumed via `iter()`, results stream out as
they become computable via border computation.

The simpler alternative (cache all, then compute full grid) is also valid when border
tracking bookkeeping is not justified by the input size.

### Implementation (broadcast/mod.rs)

The broadcasting module produces an index iterator over the Cartesian product of all
argument arrays. For the eager path (`to_array`), with the `parallel` feature enabled,
rayon parallelizes over the flat index space. For the lazy path (`iter`), the broadcast
iterator yields one index combination at a time.

Row-major ordering: last axis varies fastest, matching ndarray's default memory layout.

## Concurrency model

`rayon` behind the `parallel` feature flag.

**v1 uses:**
- Broadcasting parallelism in `to_array()` — rayon `par_iter` over the flat index space
- Concurrent evaluation of independent ASTs — `par_iter` over a collection of
  CompiledExpr/args pairs

**Future use (chaining/DAG):**
- `rayon::scope` for parallel execution of independent subtrees in the DAG
- Sequential ordering for parent→child edges
- rayon's work-stealing naturally handles mixed parallel/sequential DAG shapes

## FFI layer (Swift)

Feature-gated behind `ffi`. Full parity with the Rust API, including iterator I/O.

### Boundary protocol

- Structured types cross the boundary as JSON (mathlex `Expression` already has serde
  support)
- Streaming types (iterators) cross via callback adapters
- Errors cross as `String` (human-readable messages)
- Opaque Rust types (`CompiledExpr`, `EvalHandle`, `EvalIter`) are held by Swift as
  pointers managed by swift-bridge (automatic Drop)

### swift-bridge interface

```rust
#[swift_bridge::bridge]
mod ffi {
    extern "Rust" {
        type CompiledExpr;
        type EvalHandle;
        type EvalIter;

        fn compile_from_json(ast_json: &str, constants_json: &str)
            -> Result<CompiledExpr, String>;

        fn eval_with_json(expr: &CompiledExpr, args_json: &str)
            -> Result<EvalHandle, String>;

        fn eval_with_iters(
            expr: &CompiledExpr,
            args_json: &str,
            iter_arg_names: Vec<String>,
            next_values: Vec<Box<dyn FnMut() -> Option<f64>>>,
        ) -> Result<EvalHandle, String>;

        // EvalHandle methods
        fn shape(handle: &EvalHandle) -> Vec<i64>;
        fn len(handle: &EvalHandle) -> i64;
        fn to_array_json(handle: EvalHandle) -> Result<String, String>;
        fn scalar_json(handle: EvalHandle) -> Result<String, String>;
        fn into_iter(handle: EvalHandle) -> EvalIter;

        // EvalIter — maps to Swift IteratorProtocol
        fn next_result(iter: &mut EvalIter) -> Option<String>;

        // Introspection
        fn argument_names(expr: &CompiledExpr) -> Vec<String>;
        fn is_complex(expr: &CompiledExpr) -> bool;
    }
}
```

### Swift convenience wrapper

A thin Swift package in `swift/` wraps the raw bridge in idiomatic Swift types. Conforms
`EvalIter` wrapper to `Sequence`/`IteratorProtocol`. Provides typed `MathEvalResult`
instead of raw JSON strings.

### Iterator input from Swift

Swift closures cross the FFI boundary via swift-bridge's opaque callable support. The
Rust side wraps the callback in a `SwiftIterAdapter` implementing `Iterator<Item = f64>`,
which plugs into `EvalInput::Iter` transparently.

```rust
struct SwiftIterAdapter {
    callback: Box<dyn FnMut() -> Option<f64>>,
}

impl Iterator for SwiftIterAdapter {
    type Item = f64;
    fn next(&mut self) -> Option<f64> { (self.callback)() }
}
```

Multiple iterator arguments from Swift are supported — each gets its own callback. The
caching and border computation engine handles them identically to native Rust iterators.

## Documentation

Documentation is a first-class deliverable, not an afterthought.

### Rust documentation (docs.rs / crates.io)

- Full `//!` crate-level docs in `lib.rs` with overview, quick-start example, and feature
  flag documentation
- `///` doc comments on every public type, method, and enum variant
- Compilable doc examples (```` ```rust ```` blocks that are tested by `cargo test`)
- `examples/` directory with standalone runnable examples:
  - `basic_eval.rs` — compile and evaluate `2*x + 3` with a single value
  - `broadcasting.rs` — array and grid evaluation via eval() with array inputs
  - `complex_numbers.rs` — real/complex promotion, complex constants
  - `iterator_input.rs` — streaming evaluation with iterator arguments

### Swift documentation (SPI / SPM)

- DocC-compatible documentation comments on all public Swift types
- `.spi.yml` for Swift Package Index
- `swift/Examples/` directory with:
  - `BasicEval.swift` — compile, evaluate, consume result
  - `Broadcasting.swift` — array arguments, grid evaluation
  - `StreamingEval.swift` — iterator input from Swift, consuming results via for-in
- `Package.swift` at repo root for SPM distribution

### README.md

Single README covering both Rust and Swift usage:
- Badges: License, CI, crates.io release, docs.rs, SPM compatibility, Swift versions,
  supported platforms
- Value proposition: what it does, who it's for
- Quick-start for Rust (cargo add, compile, eval)
- Quick-start for Swift (SPM dependency, compile, eval)
- Feature flags documentation
- API overview with code examples
- Link to full docs (docs.rs for Rust, SPI for Swift)

## Testing strategy

### Unit tests

**compile_tests.rs:**
- Valid scalar ASTs compile successfully
- Each non-numerical variant → `UnsupportedExpression` with correct variant name
- Constant substitution produces correct compiled form (verified via eval round-trip)
- Math constant resolution (π, e, i)
- Constant folding: `2 * π` → single literal
- Unknown function → `UnknownFunction`
- Arity mismatch → `ArityMismatch`
- Unresolved variable → `UnresolvedVariable`
- Sum/product non-integer bounds → `NonIntegerBounds`
- Sum/product index scoping: index shadows outer variable of same name

**eval_tests.rs:**
- Arithmetic: addition, subtraction, multiplication, division, power, modulo
- All built-in functions against known values (sin(π/2)=1, exp(0)=1, etc.)
- Rational: `3/4` → `0.75`
- Nested: `sin(x^2 + 1)`
- Sum: `Σ_{k=1}^{5} k` → 15
- Product: `Π_{k=1}^{4} k` → 24
- Division by zero → `EvalError::DivisionByZero`
- Numeric edge cases: very large/small values, near-zero denominators

**complex_tests.rs:**
- Real inputs → `NumericResult::Real`
- Complex constant → result promotes to complex
- `sqrt(-1)` → complex promotion
- Complex arithmetic: `(a+bi) * (c+di)`
- Mixed real/complex arguments

**broadcast_tests.rs:**
- All scalar args → 0-d output
- One array arg → 1-d output
- Two array args → 2-d Cartesian product
- Mixed scalar + array → broadcast
- Shape inspection before consumption
- Eager (`to_array`) and lazy (`iter`) produce identical results
- Empty array input → empty output
- Iterator input: single iterator, multiple iterators, mixed array + iterator
- Border computation correctness: intermediate streamed results match final array

**error_tests.rs:**
- Every `CompileError` variant exercised
- Every `EvalError` variant exercised
- Per-element errors in iterator mode

### Property-based tests (proptest)

- Random valid ASTs always compile without panic
- Scalar eval matches naive recursive evaluation
- Eager and lazy paths produce identical results for same inputs
- Broadcasting output length = product of array lengths
- Commutativity: order of arguments in map doesn't affect results

### Benchmarks (criterion)

- Single-point eval: simple vs complex expressions
- Broadcasting scaling: 10, 100, 1K, 10K, 100K points
- Grid scaling: N×M dimensions
- Compile cost vs eval cost ratio
- Parallel vs sequential (with/without `parallel` feature)

## CI/CD

### GitHub Actions

**On push (all branches):**
- `cargo fmt --check`
- `cargo clippy -- -D warnings`
- `cargo test` (all features)
- `cargo test --no-default-features`
- Swift build + test (SPM)

**On semantic version tag push to main:**
- Publish to crates.io
- Swift package is automatically available via SPM (tag-based)

**Documentation:**
- docs.rs built automatically on crates.io publish
- SPI documentation via `.spi.yml` configuration

## Extension points (v1.x, not v1)

These are explicitly NOT in v1 scope but the architecture accommodates them:

### Vector/matrix AST evaluation
Add new `CompiledNode` variants (`Vector`, `Matrix`, `DotProduct`, etc.). The compile
pipeline's validate pass accepts them; fold handles element-wise folding. Eval dispatches
on the new variants. No API change — `NumericResult` may need a `Vector`/`Matrix` variant.

### Chained/tree evaluation (DAG)
ASTs arranged in a DAG where outputs feed into inputs of downstream ASTs. Parent output
maps to child input variable(s), including complex → real/imaginary split. Independent
subtrees run concurrently via rayon. Uses the same `compile` + `eval` primitives
internally — the DAG scheduler calls `eval` on each node and wires results forward.

### Convergent infinite series
If sum/product bounds are symbolic (not concrete integers), attempt numerical convergence
with user-specified tolerance and max iterations. Requires new `CompileOption` for
convergence parameters. Deferred because this blurs the line with CAS functionality.
