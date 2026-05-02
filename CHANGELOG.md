# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.1] - 2026-05-02

### Fixed

- Replaced `workspace = true` mathlex dependency with explicit version
  pin so `cargo publish` works in standalone CI (non-workspace) contexts.

## [0.2.0] - 2026-05-02

### Breaking Changes

- **mathlex dependency bumped from 0.3 to 0.4.** The upstream `Expression` type
  changed from a flat enum to a struct with `kind: ExprKind` and
  `annotations: AnnotationSet`. All pattern matching and construction sites
  updated accordingly. Consumers that construct `Expression` ASTs manually must
  migrate to the new API (see mathlex v0.4.0 CHANGELOG for the migration guide).

### Changed

- All internal match arms migrated from `Expression::Variant` to
  `match &expr.kind { ExprKind::Variant { .. } }`.
- All AST construction migrated to convenience constructors
  (`Expression::integer()`, `Expression::variable()`, etc.) or
  `ExprKind::Variant { .. }.into()`.

## [0.1.1] - 2026-04-12

### Added

- Two-phase compile/eval architecture: compile once with constants, evaluate many times
- 23 built-in math functions (trig, hyperbolic, exponential, logarithmic, rounding, min/max)
- N-dimensional Cartesian product broadcasting over argument arrays
- Three consumption modes: scalar, eager array (`to_array`), lazy iterator (`iter`)
- Real/complex number support with automatic promotion
- Constant folding at compile time
- Sum and product finite aggregation (sigma/pi notation)
- `serde` feature for serialization of `CompiledExpr` and `NumericResult`
- `parallel` feature for rayon-based parallel broadcasting
- `ffi` feature with Swift FFI bridge (JSON boundary, callback iterators)
- Swift wrapper package with SPM support
- Property-based tests via proptest
- Criterion benchmarks for scalar eval and broadcasting scaling
- GitHub Actions CI (fmt, clippy, test, docs) and release workflow
