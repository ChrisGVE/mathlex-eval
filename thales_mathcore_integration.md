# mathlex-eval — thales/mathcore Integration Specification

**Document:** `mathlex-eval/thales_mathcore_integration.md`
**Version:** draft-1 (2026-04-22)
**Status:** draft, pre-freeze
**Target release:** mathlex-eval v0.2.0
**Depends on:**
- `mathcore-units/SPECIFICATION.md` draft-1 (2026-04-22)
- `mathcore-constants/SPECIFICATION.md` draft-1 (2026-04-22)
- `mathlex/thales_mathcore_integration.md` (being drafted in parallel;
  referenced here as the MI-1..MI-N requirement set)

This document specifies how mathlex-eval consumes `AnnotatedExpression`
(produced by mathlex per MI-1..MI-N) and produces numeric results with
attached units. Once accepted, the types, API signatures, and behavioral
invariants described here are frozen for the v0.2.x series. Additions are
permitted in minor versions; removals or behavioral regressions require a
coordinated major bump with mathlex and thales.

---

## 1. Scope, Architectural Principle, and Dependency List

### 1.1 Scope

This document covers:

- The shape of `AnnotatedExpression` as mathlex-eval receives it.
- The numeric evaluation pipeline, extended to handle constant-tagged nodes.
- The resolution of constant values from the mathcore-constants catalog.
- The unit pass-through contract: how mathlex-eval carries the factored
  output unit alongside the numeric result without performing any unit algebra.
- The `EvaluatedResult` type and the public `evaluate_annotated` entry point.
- The error model.
- Backward compatibility for Expression inputs that carry no annotations.
- Broadcasting semantics when units are present.
- Uncertainty propagation (deferred to v0.2.0+ sub-release; flagged as an
  open issue here).
- The mathlex-eval test strategy for this integration surface.

### 1.2 Architectural Principle: No Unit Conversion in mathlex-eval

**mathlex-eval does not perform unit conversion.** This is a hard invariant,
not an implementation convenience.

Unit conversions are substitutions applied to the Expression tree by mathlex
at Expression+ assembly time. By the time mathlex-eval receives an
`AnnotatedExpression`, all unit-heterogeneous sub-expressions have already been
rewritten: a sum of meters and feet has had the foot term multiplied by the
appropriate scale factor; a temperature in Celsius has had the affine offset
applied. mathlex-eval receives a tree whose numeric values are all in a
consistent unit, and a single `output_unit` annotation that names what that
unit is.

This design preserves the symbol-preservation invariant for physical constants.
If an expression uses `c` (the speed of light) and the caller's output unit is
km/s, mathlex converts the scale at Expression+ assembly, leaving `c` in the
tree tagged as `ConstantId::SpeedOfLight`. mathlex-eval evaluates the numeric
value of `c` from the mathcore-constants catalog and multiplies by the
conversion factor that is already present as a numeric literal node in the
tree. mathlex-eval never decides that "this unit needs converting" — that
decision was already made by mathlex, expressed in the tree structure.

Violating this principle — adding any unit-selection or unit-conversion logic
to mathlex-eval — would require mathlex-eval to depend on unitalg and would
duplicate conversion logic already encoded in the Expression tree. Rule 5 of
the thales workspace (zero technical debt) forbids this duplication.

### 1.3 Dependency List

mathlex-eval v0.2.0 adds the following direct crate dependencies:

| Crate | Purpose | Existing? |
|---|---|---|
| `mathlex` | `Expression`, `AnnotatedExpression`, `AnnotationSet` types | Yes (existing) |
| `mathcore-units` | `UnitExpression`, `UnitId`, `ConstantId` types carried in `AnnotationSet` | New |
| `mathcore-constants` | `lookup_constant`, `ConstantSpec` for constant-value resolution | New |

mathlex-eval does **not** depend on:

- `unitalg` — mathlex-eval performs no unit algebra. unitalg is strictly for
  dimension computation, system selection, and conversion-factor emission;
  none of those operations occur during numeric evaluation.
- `thales` — mathlex-eval is a subordinate evaluator, not a CAS. The
  dependency direction is one-way: thales may call mathlex-eval, but
  mathlex-eval does not call thales.

Feature-flag requirement: `mathcore-units` and `mathcore-constants` are
required dependencies for the `annotated` feature (new in v0.2.0). The existing
`evaluate` path (non-annotated `Expression`) compiles and runs without these
dependencies when the `annotated` feature is not enabled, preserving backward
compatibility for consumers that do not need unit-annotated evaluation.

---

## 2. Input Shape: the `AnnotatedExpression` Contract

### 2.1 What mathlex produces (per MI-1..MI-N)

mathlex, after parsing an Expression+ string, produces an `AnnotatedExpression`
that contains:

```rust
pub struct AnnotatedExpression {
    /// The main Expression tree, post-unit-conversion rewrites.
    /// All unit heterogeneity has been resolved by mathlex before delivery.
    /// Numeric scale factors introduced by unit conversion are present as
    /// ordinary literal nodes in the tree.
    pub expression: Expression,

    /// Per-node annotation payload. Every node that carries a unit or
    /// constant tag has an entry here, keyed by a node identity that
    /// corresponds to the Expression tree's node addressing scheme (per
    /// MI-1..MI-N). Nodes without annotations have an empty AnnotationSet
    /// or no entry, depending on the mathlex implementation.
    pub annotations: AnnotationSet,

    /// The factored output unit for the entire expression's result.
    /// Set by mathlex after calling unitalg to compute and factor the
    /// result unit. None when the expression is dimensionless or carries
    /// no unit annotations.
    pub output_unit: Option<UnitExpression>,
}
```

### 2.2 `AnnotationSet` contents relevant to mathlex-eval

The `AnnotationSet` substrate (defined in the mathlex RFC, requirement M-R1)
attaches arbitrary metadata to Expression nodes. mathlex-eval reads two
annotation keys:

1. **`unit`** — key type `UnitExpression`. Present on nodes whose
   Expression subtree represents a quantity with a known unit (e.g., a
   variable bound to a measurement in meters, or a literal `9.8` annotated as
   acceleration in m·s⁻²). mathlex-eval does not perform arithmetic with this
   annotation; it reads it only to pass through to the result's `unit` field
   when the expression tree has exactly one root unit. The root-level
   `output_unit` field is the authoritative unit for the result; per-node
   `unit` annotations are informational.

2. **`constant`** — key type `ConstantId`. Present on `Expression::Variable`
   nodes that represent a named physical or mathematical constant (e.g., a
   node for `c` carries `ConstantId::SpeedOfLight`). mathlex-eval uses this
   annotation to resolve the node to a numeric value from the mathcore-constants
   catalog rather than requiring the caller to supply a variable binding.

All other annotation keys are ignored by mathlex-eval. Unrecognized keys
neither cause errors nor affect the evaluation result.

### 2.3 Empty `AnnotationSet` — backward compatibility

An `AnnotatedExpression` with an empty `AnnotationSet` and `output_unit:
None` is semantically equivalent to a bare `Expression`. Calling
`evaluate_annotated` on such an input produces the same result as calling the
existing `evaluate` entry point on the same `Expression`. This is the
backward-compatibility guarantee (see § 8).

### 2.4 What mathlex-eval does NOT read from annotations

- `unit` on sub-expression nodes for unit-arithmetic purposes. Unit
  arithmetic was completed by mathlex; the tree is already in consistent units.
- Any system-selection, conversion-factor, or dimension-check annotation.
  Those are unitalg concerns, completed at mathlex+unitalg assembly time.

---

## 3. Numeric Evaluation Pipeline

### 3.1 Overview

The evaluation pipeline is the same as the existing `evaluate` path, extended
at two points:

1. **Variable resolution** — before looking up a variable in the caller-supplied
   `variables` map, check whether the node carries a `constant` annotation. If
   so, resolve via mathcore-constants (§ 4) instead. The `variables` map is
   still consulted first for annotated variables, so callers can override
   constant values for testing or what-if scenarios.

2. **Result construction** — after the numeric traversal is complete, attach
   `output_unit` from the `AnnotatedExpression` to the `EvaluatedResult`
   without modification.

No other changes to evaluation order, operator semantics, function
evaluation, or broadcasting rules are required.

### 3.2 Evaluation traversal

The evaluator walks the `Expression` tree depth-first. At each node:

| Node kind | Action |
|---|---|
| `Integer(n)` | Convert to f64 |
| `Float(x)` | Use directly |
| `Rational(p, q)` | Compute p as f64 / q as f64 |
| `Variable(name)` | See § 3.3 |
| `BinaryOp(op, l, r)` | Evaluate l and r recursively; apply op |
| `UnaryOp(op, e)` | Evaluate e; apply op |
| `Function(name, args)` | Evaluate args; apply named function |
| `Sum / Product` | Evaluate over index range |
| All other variants | Evaluate per existing rules |

No node kind is added. The annotation is checked only at `Variable` nodes.

### 3.3 Variable resolution with constant fallback

```
fn resolve_variable(
    name: &str,
    node_annotations: Option<&NodeAnnotation>,
    variables: &HashMap<String, NumericValue>,
) -> Result<NumericValue, EvalError> {
    // Step 1: caller-supplied binding takes unconditional precedence.
    if let Some(v) = variables.get(name) {
        return Ok(v.clone());
    }

    // Step 2: if the node is tagged as a constant, resolve from catalog.
    if let Some(ann) = node_annotations {
        if let Some(constant_id) = ann.get_constant() {
            return resolve_constant(constant_id);
        }
    }

    // Step 3: no binding, no constant tag — missing variable.
    Err(EvalError::MissingVariable { name: name.to_owned() })
}
```

The precedence rule — caller map first, constant catalog second — is
intentional. It lets tests and scenario analyses override physical constants
without modifying the catalog. In production use, no binding is supplied for
physical constants; the catalog value is used automatically.

### 3.4 Function evaluation

Function evaluation is unchanged. The argument expressions are evaluated
recursively; the function is applied to the resulting numeric values.
No function receives a unit argument; functions operate on numbers only. The
unit of a function's output is determined by mathlex at annotation assembly
time and is already encoded in the tree or in `output_unit`.

### 3.5 Broadcasting

Broadcasting semantics (NumPy rules per mathlex-eval Principle 5 and
Architecture Rule 3) are unaffected by the addition of unit annotations.
The `output_unit` field is scalar — one `UnitExpression` for the whole
result, regardless of whether the result is a scalar, complex number, or
n-dimensional array. Per-element unit variation is not supported and is
not a goal for v0.2.0. See § 9 for details.

---

## 4. Constant-Value Resolution

### 4.1 Overview

When evaluation reaches a `Variable` node tagged with a `ConstantId`, the
evaluator calls `mathcore_constants::lookup_constant(id)` and resolves the
returned `ConstantSpec::value` to a `NumericValue`. The resolution algorithm
handles four cases: numeric literals, known mathematical constants, symbolic
composites, and unresolvable expressions.

### 4.2 Resolution algorithm

```
fn resolve_constant(id: ConstantId) -> Result<NumericValue, EvalError> {
    let spec = mathcore_constants::lookup_constant(id);
    resolve_expression_to_numeric(&spec.value)
}

fn resolve_expression_to_numeric(
    expr: &Expression,
) -> Result<NumericValue, EvalError> {
    match expr {
        // Case 1: Numeric literals — convert directly.
        Expression::Integer(n) => Ok(NumericValue::Scalar(*n as f64)),
        Expression::Float(x)   => Ok(NumericValue::Scalar(x.into_inner())),
        Expression::Rational(p, q) =>
            Ok(NumericValue::Scalar(*p as f64 / *q as f64)),

        // Case 2: Known mathematical constant atoms —
        //         use mathlex-eval's own IEEE 754 representations.
        Expression::Constant(MathConst::Pi)         =>
            Ok(NumericValue::Scalar(std::f64::consts::PI)),
        Expression::Constant(MathConst::E)          =>
            Ok(NumericValue::Scalar(std::f64::consts::E)),
        Expression::Constant(MathConst::EulerGamma) =>
            Ok(NumericValue::Scalar(0.577_215_664_901_532_86_f64)),
        Expression::Constant(MathConst::Phi)        =>
            Ok(NumericValue::Scalar(1.618_033_988_749_895_f64)),

        // Case 3: Symbolic composite (e.g., ℏ = h / (2·π)).
        //         Recursively evaluate the inner expression.
        //         Any Variable nodes in the inner expression that are
        //         tagged with a ConstantId are resolved recursively.
        //         Variable nodes with no tag and no caller binding fail
        //         with MissingVariable.
        other => {
            // Evaluate `other` as if it were a standalone Expression,
            // with an empty variables map and the constant-resolution
            // path active for any Variable nodes that carry constant tags.
            evaluate_inner_constant_expr(other)
        }
    }
}
```

### 4.3 Recursive resolution of symbolic composites

`ReducedPlanckConstant` (ℏ) is stored in mathcore-constants as
`Expression::BinaryOp(Div, Variable("h"), BinaryOp(Mul, Integer(2), Constant(Pi)))`,
where `Variable("h")` carries the annotation `ConstantId::PlanckConstant`.

When mathlex-eval reaches a node tagged with `ConstantId::ReducedPlanckConstant`,
it calls `resolve_constant(ReducedPlanckConstant)`, which calls
`resolve_expression_to_numeric` on the symbolic composite. The recursive call
reaches `Variable("h")`, which carries `ConstantId::PlanckConstant`, and
resolves to the numeric value of h. The recursion bottoms out at numeric
literals and mathematical-constant atoms. The maximum recursion depth is
bounded by the catalog's derivation depth (at most three or four levels for
any current catalog entry).

`StefanBoltzmannConstant` (σ = 2π⁵ k_B⁴ / (15 h³ c²)) is similarly
resolved recursively: each of k_B, h, and c bottoms out at their exact
defined-SI numeric values within one further level of recursion.

### 4.4 Precision note

This resolution produces IEEE 754 double-precision results. The constants
catalog stores defined-exact constants with their exact digit counts (per
MC-10). When converted to f64, some truncation is unavoidable for irrational
or high-precision values. This is acceptable for v0.2.0 (Architecture Rule 4:
f64 default, opt-in precision). Arbitrary-precision evaluation of constants is
deferred to a future feature-flag addition.

`ConstantSpec::uncertainty` is present on measured constants. mathlex-eval
v0.2.0 reads the `value` field only; it does not consume `uncertainty`. The
field must not cause an error when present. See § 10 for the deferred
uncertainty-propagation work.

### 4.5 Catalog lookup failure

`mathcore_constants::lookup_constant(id)` panics in debug builds and returns a
defined fallback in release builds when called with an id that has no catalog
entry (per MC-5 in the mathcore-constants spec). The catalog completeness CI
test (MC-17) guarantees no `ConstantId` variant lacks a `ConstantSpec` entry.
In the event that the catalog is incomplete (possible during development, before
the CI test has run), mathlex-eval converts the panic or fallback into
`EvalError::MissingConstant { id }` at the call boundary, surfacing the defect
cleanly.

---

## 5. Unit Pass-Through

### 5.1 What mathlex-eval does with units

mathlex-eval does not compute with units. It does not:

- Check whether the expression's dimension is consistent.
- Select a unit system.
- Convert between units.
- Call any function from unitalg.

All of those operations were completed by mathlex at Expression+ assembly time,
using unitalg as specified in `mathlex/thales_mathcore_integration.md` (MI-1..MI-N).

mathlex-eval's only obligation to units is to carry the `output_unit` field
from the input `AnnotatedExpression` to the `EvaluatedResult`, unmodified.
This unit represents what the numeric result means in physical terms.

### 5.2 Unit preservation

```rust
pub struct EvaluatedResult {
    /// The numeric result of evaluating the Expression tree.
    pub value: NumericValue,

    /// The physical unit of `value`, as determined by mathlex at
    /// annotation assembly time. None when the expression is dimensionless
    /// or carries no unit annotations.
    /// mathlex-eval copies this field from AnnotatedExpression::output_unit
    /// without inspection or modification.
    pub unit: Option<UnitExpression>,

    /// Informational warnings that do not prevent a result from being
    /// returned (e.g., domain promotions, precision-limit notices).
    pub warnings: Vec<EvalWarning>,
}
```

### 5.3 No per-element unit

`unit` is a single `Option<UnitExpression>` regardless of whether `value` is
a scalar or an array. The unit applies uniformly to all elements of the result.
If the caller requires per-element unit variation (e.g., a mixed-unit array),
they must evaluate each element separately. mathlex-eval does not support
mixed-unit arrays and does not validate that array inputs carry consistent units
— mathlex is responsible for that validation at parse time.

### 5.4 Unit from output_unit only

The `EvaluatedResult::unit` field is populated exclusively from
`AnnotatedExpression::output_unit`. Per-node `unit` annotations in the
`AnnotationSet` are not aggregated, propagated, or used to construct the
result unit. The reason: propagating per-node units through arithmetic would
require unit algebra (multiply, divide, power) — that is unitalg's
responsibility, not mathlex-eval's. mathlex has already done this work; the
factored result is in `output_unit`.

---

## 6. `EvaluatedResult` Shape and Public API Surface

### 6.1 Types

```rust
/// Primary output type for annotated evaluation.
pub struct EvaluatedResult {
    pub value: NumericValue,
    pub unit: Option<UnitExpression>,
    pub warnings: Vec<EvalWarning>,
}

/// The numeric payload of an evaluation result.
pub enum NumericValue {
    /// Single real number.
    Scalar(f64),
    /// Single complex number (promoted from Scalar when a domain-extending
    /// operation produces an imaginary component).
    Complex(Complex64),
    /// N-dimensional real array (NumPy-style broadcasting result).
    Array(ndarray::ArrayD<f64>),
    /// N-dimensional complex array.
    ComplexArray(ndarray::ArrayD<Complex64>),
}

/// Non-fatal conditions attached to a result.
pub enum EvalWarning {
    /// A domain-extending operation promoted the result from real to complex.
    DomainPromotion { node_description: String },
    /// A constant value was truncated to f64 from a higher-precision source.
    PrecisionTruncation { constant_id: ConstantId },
    /// An arithmetic operation produced a non-finite value (inf or NaN)
    /// for at least one element; result is still returned.
    NonFiniteValue,
}
```

### 6.2 Entry point

```rust
/// Evaluate an annotated expression.
///
/// `variables` maps variable names to numeric values. Variables annotated
/// with a ConstantId are resolved from the mathcore-constants catalog unless
/// overridden by an entry in `variables`.
///
/// Returns an EvaluatedResult carrying the numeric value and the attached
/// unit (copied from expr.output_unit).
pub fn evaluate_annotated(
    expr: &AnnotatedExpression,
    variables: &HashMap<String, NumericValue>,
) -> Result<EvaluatedResult, EvalError>;
```

### 6.3 Existing entry point unchanged

The existing non-annotated entry point:

```rust
pub fn evaluate(
    expr: &Expression,
    variables: &HashMap<String, NumericValue>,
) -> Result<NumericValue, EvalError>;
```

remains unchanged and continues to work exactly as it does in v0.1.x.
`evaluate_annotated` is the new entry point; it does not replace or wrap
`evaluate` internally — the two paths share evaluation logic but are separate
public functions.

### 6.4 Internal-only bridge

An internal function bridges the two entry points for the shared arithmetic
core:

```rust
fn evaluate_inner(
    expr: &Expression,
    annotations: &AnnotationSet,
    variables: &HashMap<String, NumericValue>,
) -> Result<NumericValue, EvalError>;
```

`evaluate` calls `evaluate_inner` with an empty `AnnotationSet`. `evaluate_annotated`
calls `evaluate_inner` with the `AnnotationSet` from the `AnnotatedExpression`, then
wraps the `NumericValue` result in an `EvaluatedResult` with the `output_unit`.

---

## 7. Error Model

### 7.1 Error enum

```rust
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "kind", content = "value"))]
pub enum EvalError {
    /// A variable appeared in the expression with no value in the caller's
    /// map and no ConstantId annotation in the AnnotationSet.
    MissingVariable {
        name: String,
        /// Source position from the Expression node, if available.
        position: Option<SourcePosition>,
    },
    /// A ConstantId annotation was present but the mathcore-constants catalog
    /// had no entry for that id. Should not occur when catalog is complete;
    /// surfaces catalog defects during development.
    MissingConstant {
        id: ConstantId,
    },
    /// Division by zero encountered during evaluation.
    DivisionByZero,
    /// A function received an argument outside its domain, in real mode
    /// (e.g., sqrt of a negative number when complex promotion is disabled).
    DomainError {
        fn_name: String,
        arg: String,
    },
    /// Two array arguments had incompatible shapes for broadcasting.
    BroadcastMismatch {
        left_shape: Vec<usize>,
        right_shape: Vec<usize>,
    },
    /// The inner Expression of a symbolic constant definition could not be
    /// evaluated (e.g., it contains a Variable with no binding or annotation).
    ConstantResolutionError {
        id: ConstantId,
        cause: Box<EvalError>,
    },
}
```

### 7.2 Error behavior

`EvalError::MissingVariable` is produced when the evaluator reaches a
`Variable` node and neither the caller's map nor the constant catalog can
provide a value. The `position` field is populated from any source-position
metadata present on the `Expression` node, allowing callers to point the user
at the problematic token in the input string.

`EvalError::MissingConstant` is a developer-facing error. In a correctly
assembled system (catalog complete, CI passing), it cannot occur. mathlex-eval
surfaces it rather than panicking so that integration tests can detect catalog
gaps programmatically.

`EvalError::ConstantResolutionError` wraps a recursive failure: the
symbolic expression stored in a `ConstantSpec::value` itself failed to evaluate.
The `id` field identifies the constant whose resolution triggered the failure;
the `cause` field carries the inner error. This chain allows callers to diagnose
nested resolution failures (e.g., a symbolic constant that references another
constant not yet in the catalog).

Non-fatal conditions (domain promotion, precision truncation, non-finite
arithmetic) produce `EvalWarning` entries in `EvaluatedResult::warnings`
rather than errors. The result is still returned.

### 7.3 Error enum is additive

`EvalError` is marked `#[non_exhaustive]`. Adding new variants is a minor
version bump. Removing or renaming variants is a major version bump coordinated
with consumers.

---

## 8. Backward Compatibility

### 8.1 Non-annotated expressions evaluate identically

An `Expression` without any unit or constant annotations must produce the same
numeric result via `evaluate_annotated(AnnotatedExpression { expression: expr,
annotations: empty, output_unit: None })` as via `evaluate(expr, variables)`.
This is guaranteed by construction: `evaluate_annotated` calls `evaluate_inner`
with an empty `AnnotationSet`, which makes the annotation-lookup paths no-ops.
The result carries `unit: None`.

### 8.2 Non-annotated variable behavior is unchanged

A `Variable` node in an unannotated expression that is present in the caller's
`variables` map resolves to its map value. A `Variable` not in the map
produces `EvalError::MissingVariable`, same as before. No constant-catalog
lookup is attempted for unannotated variables.

### 8.3 New dependency under a feature flag

The `mathcore-units` and `mathcore-constants` dependencies are gated behind the
`annotated` feature. Consumers that do not enable this feature do not link
these crates. The existing published behavior (mathlex-eval v0.1.x) is
reproduced exactly when the `annotated` feature is not enabled.

```toml
[features]
default = ["std"]
std    = []
annotated = ["dep:mathcore-units", "dep:mathcore-constants"]
serde  = ["dep:serde"]
```

The `evaluate_annotated` function and the `EvaluatedResult`, `EvalWarning`,
and `ConstantId`-bearing `EvalError` variants are compiled only when the
`annotated` feature is enabled.

### 8.4 `evaluate` function signature is unchanged

The return type of the existing `evaluate` function remains `Result<NumericValue,
EvalError>`. No field is added to `NumericValue` or removed from `EvalError` in
a way that breaks existing match arms.

---

## 9. Broadcasting Semantics with Units

### 9.1 Units are scalar, not per-element

Broadcasting semantics (NumPy rules, mathlex-eval Architecture Rule 3)
are applied to the numeric values in the Expression tree independently of
units. The unit annotation is not part of the broadcast computation.

When the result of `evaluate_annotated` is an `ndarray::ArrayD<f64>`, the
`unit` field of `EvaluatedResult` is the single `UnitExpression` that applies
to every element of the array. mathlex-eval does not validate, compute, or
transform units on a per-element basis.

### 9.2 Unit compatibility at the array level

If two sub-expressions have incompatible units and should broadcast together,
mathlex is responsible for detecting and rejecting that at parse time (before
producing an `AnnotatedExpression`). If mathlex produces an `AnnotatedExpression`
for a mixed-unit broadcasted expression, mathlex-eval trusts the annotation and
evaluates numerically. The trust is warranted because mathlex called unitalg to
validate unit compatibility and inject conversion factors before delivering the
tree.

### 9.3 Array result, single output unit

The `output_unit` field in `AnnotatedExpression` is set by mathlex to the
factored unit of the full expression result. For a broadcasted expression, this
is the unit of each scalar element in the result array. mathlex-eval copies
this to `EvaluatedResult::unit` unchanged.

Example: an expression `v * t` where `v` is an array of velocities in m/s and
`t` is a scalar time in seconds produces an array of distances. mathlex sets
`output_unit` to `UnitExpression::Atom { id: UnitId::Meter, prefix: None }`.
mathlex-eval evaluates the element-wise product via its existing broadcasting
rules and attaches `Meter` to the `EvaluatedResult`. The caller receives an
`ArrayD<f64>` of distance values in meters.

---

## 10. Uncertainty Propagation (Deferred to v0.3.0+)

### 10.1 Current status

mathlex-eval v0.2.0 does not propagate uncertainty through evaluated
expressions. The `ConstantSpec::uncertainty` field (defined in
`mathcore-constants/SPECIFICATION.md` § 2.4) is read only to avoid errors when
present; its value is not consumed.

### 10.2 What would be needed

A future uncertainty-propagation feature would require:

1. Reading `ConstantSpec::uncertainty` to obtain σ (standard uncertainty) for
   each constant used in the expression.
2. Applying first-order error propagation (partial derivatives of the expression
   with respect to each uncertain constant, multiplied by the corresponding σ).
3. Returning an additional field in `EvaluatedResult` (or a parallel result
   type) carrying the propagated uncertainty as a `NumericValue`.
4. Handling correlations between constants derived from the same CODATA
   measurement (e.g., ε₀ and μ₀ are correlated through α; treating them as
   independent overstates the propagated uncertainty slightly). Full covariance
   treatment requires the CODATA covariance matrix, which is not provided by
   mathcore-constants in v0.1.0.

### 10.3 Design constraint for v0.2.0

Because uncertainty propagation is deferred, the public API must not paint
itself into a corner. The `EvaluatedResult` struct and `evaluate_annotated`
signature are designed to accommodate a future `uncertainty:
Option<NumericValue>` field in `EvaluatedResult` without a breaking change
(addition is non-breaking).

**Open issue ME-OPEN-1:** Uncertainty propagation (first-order Taylor expansion,
no covariance). Target: mathlex-eval v0.3.0. Blocked on: mathcore-constants
covariance metadata not available in v0.1.0; requires decision on whether to
support Monte Carlo propagation or only first-order.

---

## 11. Test Strategy

### 11.1 Backward-compatibility tests

For every existing test in the mathlex-eval test suite:

- Run the same `Expression` through `evaluate_annotated` with an empty
  `AnnotationSet` and `output_unit: None`.
- Assert the resulting `value` equals the output of the existing `evaluate`
  call.
- Assert `unit` is `None`.

These tests require no new setup — they reuse existing fixtures and serve as
the primary regression guard for the backward-compatibility guarantee (§ 8).

```rust
// tests/backward_compat.rs
#[test]
fn annotated_empty_matches_bare_evaluate() {
    let expr = parse("2 * x + 1");
    let vars = [("x".to_owned(), NumericValue::Scalar(3.0))].into();
    let bare = evaluate(&expr, &vars).unwrap();
    let annotated = evaluate_annotated(
        &AnnotatedExpression { expression: expr.clone(),
                               annotations: AnnotationSet::empty(),
                               output_unit: None },
        &vars,
    ).unwrap();
    assert_eq!(bare, annotated.value);
    assert!(annotated.unit.is_none());
}
```

### 11.2 Constant-resolution tests

For each `ConstantId` variant in mathcore-constants:

1. Construct a `Variable("x")` node with a `constant` annotation for that id.
2. Call `evaluate_annotated` with an empty `variables` map.
3. Assert the result is a `Scalar(f64)` with the expected approximate value
   (within 1 ULP relative to the IEEE 754 double representation of the
   constant's value).

Defined-exact constants (SpeedOfLight, PlanckConstant, ElementaryCharge,
BoltzmannConstant, AvogadroNumber) are tested for exact bit equality after
rounding, not just approximate equality, because their exact values are
representable (or nearly so) in f64.

Symbolic composites (ReducedPlanckConstant, MolarGasConstant, FaradayConstant,
StefanBoltzmannConstant, Parsec) are tested to within 1e-6 relative error,
with the expected value computed independently from the defined-exact input
constants.

```rust
// tests/constant_resolution.rs
#[test]
fn speed_of_light_resolves_correctly() {
    let expr = annotated_variable("c", ConstantId::SpeedOfLight);
    let result = evaluate_annotated(&expr, &HashMap::new()).unwrap();
    // 299_792_458 m/s, exactly representable as f64
    assert_eq!(result.value, NumericValue::Scalar(299_792_458.0_f64));
}

#[test]
fn reduced_planck_resolves_from_symbolic_composite() {
    let expr = annotated_variable("hbar", ConstantId::ReducedPlanckConstant);
    let result = evaluate_annotated(&expr, &HashMap::new()).unwrap();
    let expected = 6.626_070_15e-34_f64 / (2.0 * std::f64::consts::PI);
    assert_relative_eq!(result.value.as_scalar().unwrap(), expected, max_relative = 1e-10);
}
```

### 11.3 Caller-override tests

Verify that a caller-supplied variable binding overrides a constant annotation:

```rust
#[test]
fn caller_overrides_constant_annotation() {
    let expr = annotated_variable("c", ConstantId::SpeedOfLight);
    let vars = [("c".to_owned(), NumericValue::Scalar(1.0))].into();
    let result = evaluate_annotated(&expr, &vars).unwrap();
    assert_eq!(result.value, NumericValue::Scalar(1.0));
}
```

### 11.4 Unit pass-through tests

```rust
#[test]
fn output_unit_is_copied_unchanged() {
    let meter_per_second = UnitExpression::Binary {
        op: BinaryOp::Div,
        left:  Box::new(UnitExpression::Atom { id: UnitId::Meter,  prefix: None }),
        right: Box::new(UnitExpression::Atom { id: UnitId::Second, prefix: None }),
    };
    let expr = AnnotatedExpression {
        expression: parse("v"),
        annotations: annotated_var("v", unit_annotation(meter_per_second.clone())),
        output_unit: Some(meter_per_second.clone()),
    };
    let vars = [("v".to_owned(), NumericValue::Scalar(10.0))].into();
    let result = evaluate_annotated(&expr, &vars).unwrap();
    assert_eq!(result.unit, Some(meter_per_second));
}
```

### 11.5 Error-path tests

- `MissingVariable`: unannotated variable not in caller map.
- `MissingConstant`: constant annotation with an id not in catalog (test
  using a mock catalog or a future-catalog-id injection path).
- `ConstantResolutionError`: a symbolic composite whose inner expression
  fails (e.g., a symbolic composite referencing a variable that is neither
  in the catalog nor in the caller's map).
- `DivisionByZero`: expression `1 / x` with `x = 0.0`.
- `BroadcastMismatch`: two array arguments of incompatible shape.

### 11.6 Broadcasting with units

```rust
#[test]
fn broadcasted_array_carries_single_unit() {
    let meter = UnitExpression::Atom { id: UnitId::Meter, prefix: None };
    let expr = AnnotatedExpression {
        expression: parse("v * 2"),
        annotations: AnnotationSet::empty(),
        output_unit: Some(meter.clone()),
    };
    let v = ndarray::array![1.0_f64, 2.0, 3.0].into_dyn();
    let vars = [("v".to_owned(), NumericValue::Array(v))].into();
    let result = evaluate_annotated(&expr, &vars).unwrap();
    match result.value {
        NumericValue::Array(arr) =>
            assert_eq!(arr, ndarray::array![2.0_f64, 4.0, 6.0].into_dyn()),
        _ => panic!("expected array"),
    }
    assert_eq!(result.unit, Some(meter));
}
```

---

## 12. Requirements (ME-1..ME-N)

| ID | Requirement | Severity |
|---|---|---|
| ME-1 | `evaluate_annotated(expr, variables)` is the single public entry point for annotated evaluation; it accepts `&AnnotatedExpression` and returns `Result<EvaluatedResult, EvalError>` | Blocker |
| ME-2 | `EvaluatedResult` carries three fields: `value: NumericValue`, `unit: Option<UnitExpression>`, `warnings: Vec<EvalWarning>` | Blocker |
| ME-3 | `EvaluatedResult::unit` is copied unchanged from `AnnotatedExpression::output_unit`; mathlex-eval performs no unit arithmetic | Blocker |
| ME-4 | Variable nodes tagged with a `ConstantId` annotation are resolved from `mathcore_constants::lookup_constant(id)` when not present in the caller's `variables` map | Blocker |
| ME-5 | Caller-supplied `variables` entries override constant-catalog values unconditionally | Blocker |
| ME-6 | Numeric literals in `ConstantSpec::value` (Integer, Float, Rational) are converted to f64 directly | Blocker |
| ME-7 | Mathematical constant atoms (`Pi`, `E`, `EulerMascheroni`, `Phi`) resolve to their IEEE 754 double representations via mathlex-eval's built-in table | Blocker |
| ME-8 | Symbolic composite constants (ℏ, R, F, σ, Parsec) are resolved by recursively evaluating the inner `Expression` stored in `ConstantSpec::value` | Blocker |
| ME-9 | Recursion for symbolic composites bottoms out at numeric literals and mathematical-constant atoms; no infinite recursion permitted | Blocker |
| ME-10 | `EvalError::MissingConstant` is returned when `lookup_constant` cannot find an entry; the call does not panic in a production build | Blocker |
| ME-11 | `EvalError::ConstantResolutionError { id, cause }` wraps recursive resolution failures and preserves the chain | Required |
| ME-12 | `ConstantSpec::uncertainty` is present-and-ignored; mathlex-eval v0.2.0 does not consume the uncertainty field | Blocker |
| ME-13 | Calling `evaluate_annotated` with an empty `AnnotationSet` and `output_unit: None` produces the same `NumericValue` as calling `evaluate` on the same `Expression` | Blocker |
| ME-14 | The existing `evaluate(expr, variables)` entry point is unchanged in signature and behavior | Blocker |
| ME-15 | `mathcore-units` and `mathcore-constants` are added as dependencies only under the `annotated` feature flag | Required |
| ME-16 | `evaluate_annotated` and its types are compiled only when the `annotated` feature is enabled | Required |
| ME-17 | Broadcasting semantics follow NumPy rules unchanged; the `unit` field in `EvaluatedResult` is a single scalar `UnitExpression` regardless of the result shape | Blocker |
| ME-18 | Mixed-unit array inputs are trusted from mathlex; mathlex-eval performs no unit compatibility check on array elements | Required |
| ME-19 | `EvalError` is `#[non_exhaustive]`; adding new variants is a minor bump; removing or renaming is a major bump | Required |
| ME-20 | `EvalWarning::PrecisionTruncation { constant_id }` is emitted when a ConstantSpec value is truncated on conversion to f64 (defined-exact or high-precision measured constants that exceed f64 precision) | Required |
| ME-21 | Constant-resolution tests cover every `ConstantId` variant, testing exact bit equality for SI-2019 defined-exact constants and relative-error bounds for symbolic composites and measured constants | Required |
| ME-22 | Caller-override tests verify that a `variables` map entry silences the constant-catalog lookup for the same name | Required |
| ME-23 | Backward-compatibility tests run the full existing test suite through `evaluate_annotated` with empty annotations and assert result equality | Blocker |
| ME-24 | Source-position info from the `Expression` node is carried into `EvalError::MissingVariable::position` where available | Required |

---

## 13. Resolved Decisions and Open Issues

### 13.1 Resolved decisions

1. **No unit conversion in mathlex-eval.** The architectural principle in § 1.2
   is a firm decision, not a tentative design. Unit conversion is a mathlex+unitalg
   responsibility completed before `AnnotatedExpression` is delivered. mathlex-eval
   never calls unitalg, never inspects unit dimensions, and never selects a
   conversion factor. This keeps mathlex-eval's dependency graph minimal and
   eliminates any risk of duplicate or inconsistent conversion logic.

2. **`output_unit` is the result unit, not per-node units.** Per-node unit
   annotations in the `AnnotationSet` are informational for downstream consumers
   (e.g., a UI that wants to label sub-expressions). mathlex-eval reads only
   `output_unit` for the result. Aggregating per-node units through arithmetic
   would require unit algebra and would duplicate mathlex's work.

3. **Caller map takes precedence over constant catalog.** A caller that supplies
   a binding for a variable named "c" overrides the speed-of-light constant,
   even if the node is annotated with `ConstantId::SpeedOfLight`. This supports
   unit testing, sensitivity analysis, and what-if scenarios without modifying
   the catalog. The annotation is not "mandatory" — it is a default resolution
   path.

4. **Symbolic composites are resolved recursively by mathlex-eval.** An
   alternative would be to require mathlex to pre-resolve all symbolic composites
   to numeric literals before delivering the `AnnotatedExpression`. This was
   rejected because it would force mathlex to depend on mathcore-constants (not
   its concern) and would prevent the CAS from seeing the symbolic derivation
   (e.g., thales benefits from knowing ℏ = h/2π symbolically). mathlex-eval is
   the right place for the numeric resolution step.

5. **`annotated` feature flag gates the new dependencies.** Existing mathlex-eval
   consumers that do not need unit-annotated evaluation should not be forced to
   link mathcore-units and mathcore-constants. The feature flag is the clean
   boundary. The trade-off: consumers that enable `annotated` take on two
   additional compile-time dependencies.

6. **f64 precision is acceptable for v0.2.0.** All defined-exact constants have
   values representable in IEEE 754 double precision to the limits of the format
   (the SI-exact digits for c, h, e, k_B, N_A fit comfortably in 53 mantissa
   bits for the leading digits). The residual truncation for high-precision
   measured constants (CODATA 2022 values with 10–12 significant digits) is
   within the noise of f64 arithmetic. Arbitrary-precision support is deferred
   to a future feature flag per Architecture Rule 4.

7. **`EvalWarning::PrecisionTruncation` for catalog constants.** When a catalog
   value's exact digit count exceeds f64 precision (which it may for some
   measured constants), a warning is emitted rather than an error. The numeric
   result is still valid at f64 precision. This is informational only.

8. **Uncertainty propagation is deferred.** The `ConstantSpec::uncertainty` field
   is present in the catalog today. mathlex-eval v0.2.0 ignores it. Adding a
   read of the field in a later version is a non-breaking addition. The
   `EvaluatedResult` struct is designed to accommodate a future
   `uncertainty: Option<NumericValue>` field without requiring a major version bump.

### 13.2 Open issues

**ME-OPEN-1: Uncertainty propagation (first-order Taylor expansion)**

Deferred from v0.2.0. Target: v0.3.0. Requirements: mathcore-constants must
expose or provide a mechanism to retrieve correlated uncertainties (or at
minimum independent standard uncertainties) for all catalog constants. A
decision is needed on whether to support only first-order linear propagation
(Jacobian-based) or also Monte Carlo propagation. Blocking dependency: no
CODATA covariance matrix in the current mathcore-constants catalog.

**ME-OPEN-2: Arbitrary-precision constant evaluation**

f64 resolution is the v0.2.0 target. For CAS use cases that require exact
symbolic arithmetic through constants (e.g., proving that two expressions
involving h, k_B, and c are equal symbolically), numeric evaluation at f64
precision is insufficient. A future `arbitrary-precision` feature flag that
resolves constants using an arbitrary-precision numeric library would address
this. Deferred pending a concrete consumer request.

**ME-OPEN-3: `MathConst::EulerGamma` symbolic atom in mathlex**

Whether `Expression::Constant(MathConst::EulerGamma)` is a supported
mathlex variant is not confirmed as of this writing (see MC-FLAG-2 in
`mathcore-constants/SPECIFICATION.md`). The constant-resolution algorithm
in § 4.2 includes a branch for `MathConst::EulerGamma`; if mathlex does
not expose this variant, mathlex-eval would never reach that branch, and
`EulerMascheroni` would be resolved via the symbolic-composite path (stored
as `Expression::Float` in the catalog per the v0.1.0 pragmatic decision).
This is not a blocking issue for v0.2.0 but should be confirmed and tested
once mathlex v0.4.0 ships.

**ME-OPEN-4: `AnnotatedExpression` type location**

This specification assumes `AnnotatedExpression` is defined in `mathlex`
and exported from the `mathlex` crate root. The exact type definition,
`AnnotationSet` API surface (how a `NodeAnnotation` is accessed, how the
`constant` annotation key is addressed), and node-identity scheme are
defined in `mathlex/thales_mathcore_integration.md` (MI-1..MI-N, being
drafted in parallel). mathlex-eval's implementation must align with the
final form of those types once that spec is frozen. This is a known
dependency that will resolve when both specs are accepted.
