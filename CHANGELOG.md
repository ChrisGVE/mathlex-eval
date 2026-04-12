# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
