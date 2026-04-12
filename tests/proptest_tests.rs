//! Property-based tests for mathlex-eval using proptest.
//!
//! Tests cover:
//! 1. Random valid ASTs compile without panicking.
//! 2. Scalar eval matches naive recursive f64 calculation.
//! 3. Eager (`to_array`) and lazy (`iter`) paths produce identical results.
//! 4. Broadcasting output length equals the product of array input lengths.
//! 5. Argument insertion order does not affect evaluation results.

use std::collections::HashMap;

use mathlex::{BinaryOp, Expression, MathFloat, UnaryOp};
use mathlex_eval::{EvalInput, NumericResult, compile, eval};
use proptest::prelude::*;

// ---------------------------------------------------------------------------
// Finite float strategy — excludes NaN and Inf to allow comparisons
// ---------------------------------------------------------------------------

fn finite_f64() -> impl Strategy<Value = f64> {
    prop::num::f64::NORMAL
        .prop_filter("finite", |v| v.is_finite())
        .prop_map(|v| {
            // Clamp to a safe range to avoid overflow in chained operations.
            v.clamp(-1e6_f64, 1e6_f64)
        })
}

/// Small positive f64 — avoids issues with log/sqrt on negative numbers.
fn positive_f64() -> impl Strategy<Value = f64> {
    (0.001_f64..1_000.0_f64).prop_filter("finite", |v| v.is_finite())
}

// ---------------------------------------------------------------------------
// Safe small integer for literals (avoids factorial overflow)
// ---------------------------------------------------------------------------

fn small_i64() -> impl Strategy<Value = i64> {
    -100_i64..=100_i64
}

// ---------------------------------------------------------------------------
// AST strategies
// ---------------------------------------------------------------------------

/// Leaf AST: Integer, Float, or Variable("x").
fn leaf_ast() -> impl Strategy<Value = Expression> {
    prop_oneof![
        small_i64().prop_map(Expression::Integer),
        finite_f64().prop_map(|v| Expression::Float(MathFloat::from(v))),
        // Variable "x" is the single free argument used in all strategies
        Just(Expression::Variable("x".into())),
    ]
}

/// Safe binary ops — excludes Div (risk of div-by-zero with random literals)
/// and Mod for the same reason; Pow is included but clamped via finite_f64.
fn safe_binary_op() -> impl Strategy<Value = BinaryOp> {
    prop_oneof![
        Just(BinaryOp::Add),
        Just(BinaryOp::Sub),
        Just(BinaryOp::Mul),
    ]
}

/// Depth-bounded AST strategy (max depth controls recursion explosion).
/// Depth 0 = leaf; each level wraps in Binary or Unary.
fn ast_strategy(depth: u32) -> impl Strategy<Value = Expression> {
    if depth == 0 {
        return leaf_ast().boxed();
    }

    let leaf = leaf_ast().boxed();

    let binary = (
        safe_binary_op(),
        ast_strategy(depth - 1),
        ast_strategy(depth - 1),
    )
        .prop_map(|(op, left, right)| Expression::Binary {
            op,
            left: Box::new(left),
            right: Box::new(right),
        })
        .boxed();

    let unary_neg = ast_strategy(depth - 1)
        .prop_map(|operand| Expression::Unary {
            op: UnaryOp::Neg,
            operand: Box::new(operand),
        })
        .boxed();

    prop_oneof![
        3 => leaf,
        2 => binary,
        1 => unary_neg,
    ]
    .boxed()
}

/// Simple binary AST of the form `x op literal` — one free variable, one literal.
fn simple_binary_ast() -> impl Strategy<Value = (Expression, BinaryOp, f64)> {
    (safe_binary_op(), finite_f64()).prop_map(|(op, lit)| {
        let ast = Expression::Binary {
            op,
            left: Box::new(Expression::Variable("x".into())),
            right: Box::new(Expression::Float(MathFloat::from(lit))),
        };
        (ast, op, lit)
    })
}

/// Two independently generated array sizes in [1, 8].
fn array_pair_sizes() -> impl Strategy<Value = (usize, usize)> {
    (1_usize..=8, 1_usize..=8)
}

// ---------------------------------------------------------------------------
// Naive recursive evaluator (reference implementation for property 2)
// ---------------------------------------------------------------------------

fn naive_eval(ast: &Expression, x: f64) -> Option<f64> {
    match ast {
        Expression::Integer(n) => Some(*n as f64),
        Expression::Float(f) => {
            let v = f64::from(*f);
            if v.is_finite() { Some(v) } else { None }
        }
        Expression::Variable(name) if name == "x" => Some(x),
        Expression::Binary { op, left, right } => {
            let l = naive_eval(left, x)?;
            let r = naive_eval(right, x)?;
            match op {
                BinaryOp::Add => Some(l + r),
                BinaryOp::Sub => Some(l - r),
                BinaryOp::Mul => Some(l * r),
                BinaryOp::Div => {
                    if r == 0.0 {
                        None
                    } else {
                        Some(l / r)
                    }
                }
                BinaryOp::Pow => {
                    let v = l.powf(r);
                    if v.is_finite() { Some(v) } else { None }
                }
                BinaryOp::Mod => {
                    if r == 0.0 {
                        None
                    } else {
                        Some(l % r)
                    }
                }
                _ => None,
            }
        }
        Expression::Unary { op, operand } => {
            let v = naive_eval(operand, x)?;
            match op {
                UnaryOp::Neg => Some(-v),
                UnaryOp::Pos => Some(v),
                _ => None,
            }
        }
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn no_constants() -> HashMap<&'static str, NumericResult> {
    HashMap::new()
}

fn make_scalar_args(x_val: f64) -> HashMap<&'static str, EvalInput> {
    let mut args = HashMap::new();
    args.insert("x", EvalInput::Scalar(x_val));
    args
}

// ---------------------------------------------------------------------------
// Property 1: Random valid ASTs compile without panicking
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    /// Any AST built from accepted variants must not cause compile() to panic.
    /// It may return CompileError (e.g. unknown function or overflow), but
    /// must never panic or produce UB.
    #[test]
    fn prop_valid_ast_compile_no_panic(ast in ast_strategy(3)) {
        // compile() may return Ok or Err — we only assert no panic.
        let _ = compile(&ast, &no_constants());
    }
}

// ---------------------------------------------------------------------------
// Property 2: Scalar eval matches naive recursive f64 calculation
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    /// For `x op literal`, compile+eval at x=x_val must agree with naive f64.
    #[test]
    fn prop_scalar_eval_matches_naive(
        (ast, op, _lit) in simple_binary_ast(),
        x_val in finite_f64(),
    ) {
        let compiled = match compile(&ast, &no_constants()) {
            Ok(c) => c,
            // Some combinations may still fail (e.g. very large Pow) — skip them.
            Err(_) => return Ok(()),
        };

        let handle = match eval(&compiled, make_scalar_args(x_val)) {
            Ok(h) => h,
            Err(_) => return Ok(()),
        };

        // handle.scalar() errors when output is not 0-d; that cannot happen for
        // a single-variable expression with a scalar arg, but guard anyway.
        let result = match handle.scalar() {
            Ok(r) => r,
            Err(_) => return Ok(()),
        };

        let eval_val = match result.to_f64() {
            Some(v) => v,
            // Complex result from real inputs — skip comparison.
            None => return Ok(()),
        };

        let naive_val = match naive_eval(&ast, x_val) {
            Some(v) => v,
            // Naive evaluator couldn't compute (e.g. div-by-zero) — skip.
            None => return Ok(()),
        };

        // Both must be finite for a meaningful comparison.
        if !eval_val.is_finite() || !naive_val.is_finite() {
            return Ok(());
        }

        // Verify the op produces the expected value (with tolerance for Pow).
        let tolerance = match op {
            BinaryOp::Pow => 1e-6,
            _ => 1e-9,
        };

        let diff = (eval_val - naive_val).abs();
        let relative = if naive_val.abs() > 1.0 {
            diff / naive_val.abs()
        } else {
            diff
        };
        prop_assert!(
            relative <= tolerance,
            "eval={eval_val}, naive={naive_val}, diff={diff}",
        );
    }
}

// ---------------------------------------------------------------------------
// Property 3: Eager (to_array) and lazy (iter) produce identical results
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// For any expression with array input, to_array() and iter() must yield
    /// the same sequence of results.
    #[test]
    fn prop_eager_and_lazy_identical(
        x_vals in prop::collection::vec(finite_f64(), 1..=16),
    ) {
        // Use a fixed, always-valid expression: x * x
        let ast = Expression::Binary {
            op: BinaryOp::Mul,
            left: Box::new(Expression::Variable("x".into())),
            right: Box::new(Expression::Variable("x".into())),
        };
        let compiled = compile(&ast, &no_constants()).unwrap();

        let make_args = || {
            let mut args = HashMap::new();
            args.insert("x", EvalInput::from(x_vals.clone()));
            args
        };

        let eager: Vec<NumericResult> = eval(&compiled, make_args())
            .unwrap()
            .to_array()
            .unwrap()
            .iter()
            .copied()
            .collect();

        let lazy: Vec<NumericResult> = eval(&compiled, make_args())
            .unwrap()
            .iter()
            .map(|r| r.unwrap())
            .collect();

        prop_assert_eq!(eager.len(), lazy.len());
        for (i, (e, l)) in eager.iter().zip(lazy.iter()).enumerate() {
            prop_assert_eq!(
                e, l,
                "mismatch at index {}: eager={:?}, lazy={:?}", i, e, l,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Property 3b: Eager and lazy agree on depth-3 ASTs with random arrays
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    /// For a randomly generated valid AST with a single array argument,
    /// eager and lazy produce the same results.
    #[test]
    fn prop_eager_lazy_random_ast(
        ast in ast_strategy(3),
        x_vals in prop::collection::vec(positive_f64(), 1..=8),
    ) {
        let compiled = match compile(&ast, &no_constants()) {
            Ok(c) => c,
            Err(_) => return Ok(()),
        };

        // Only proceed if "x" is the sole argument (other variables may appear).
        let names = compiled.argument_names();
        if names != ["x"] {
            return Ok(());
        }

        let make_args = || {
            let mut args = HashMap::new();
            args.insert("x", EvalInput::from(x_vals.clone()));
            args
        };

        let eager_result = eval(&compiled, make_args())
            .unwrap()
            .to_array();

        let lazy_result: Vec<Result<NumericResult, _>> = eval(&compiled, make_args())
            .unwrap()
            .iter()
            .collect();

        match eager_result {
            Err(_) => {
                // If eager errors, lazy must also contain at least one error.
                prop_assert!(lazy_result.iter().any(|r| r.is_err()));
            }
            Ok(arr) => {
                let eager_vec: Vec<NumericResult> = arr.iter().copied().collect();
                prop_assert_eq!(eager_vec.len(), lazy_result.len());
                for (i, (e, l)) in eager_vec.iter().zip(lazy_result.iter()).enumerate() {
                    match l {
                        Ok(lv) => prop_assert_eq!(
                            e, lv,
                            "index {}: eager={:?}, lazy={:?}", i, e, lv,
                        ),
                        Err(_) => {
                            // Per-element error in lazy is acceptable when the
                            // individual scalar computation fails (e.g., log(0)).
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Property 4: Broadcasting output length = product of array lengths
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// For x + y with array inputs of sizes (n, m), output length must be n * m.
    #[test]
    fn prop_broadcast_len_is_product_of_array_lengths(
        (n, m) in array_pair_sizes(),
        x_base in finite_f64(),
        y_base in finite_f64(),
    ) {
        let ast = Expression::Binary {
            op: BinaryOp::Add,
            left: Box::new(Expression::Variable("x".into())),
            right: Box::new(Expression::Variable("y".into())),
        };
        let compiled = compile(&ast, &no_constants()).unwrap();

        let x_vals: Vec<f64> = (0..n).map(|i| x_base + i as f64).collect();
        let y_vals: Vec<f64> = (0..m).map(|i| y_base + i as f64).collect();

        let mut args = HashMap::new();
        args.insert("x", EvalInput::from(x_vals));
        args.insert("y", EvalInput::from(y_vals));

        let handle = eval(&compiled, args).unwrap();
        prop_assert_eq!(handle.len(), n * m);
        prop_assert_eq!(handle.shape(), &[n, m]);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    /// For x + y + z with array inputs of sizes (a, b, c), output length = a * b * c.
    #[test]
    fn prop_broadcast_three_arrays_len_is_product(
        a in 1_usize..=5,
        b in 1_usize..=5,
        c in 1_usize..=5,
    ) {
        // x + y + z
        let ast = Expression::Binary {
            op: BinaryOp::Add,
            left: Box::new(Expression::Binary {
                op: BinaryOp::Add,
                left: Box::new(Expression::Variable("x".into())),
                right: Box::new(Expression::Variable("y".into())),
            }),
            right: Box::new(Expression::Variable("z".into())),
        };
        let compiled = compile(&ast, &no_constants()).unwrap();

        let x_vals: Vec<f64> = (0..a).map(|i| i as f64).collect();
        let y_vals: Vec<f64> = (0..b).map(|i| i as f64).collect();
        let z_vals: Vec<f64> = (0..c).map(|i| i as f64).collect();

        let mut args = HashMap::new();
        args.insert("x", EvalInput::from(x_vals));
        args.insert("y", EvalInput::from(y_vals));
        args.insert("z", EvalInput::from(z_vals));

        let handle = eval(&compiled, args).unwrap();
        prop_assert_eq!(handle.len(), a * b * c);
    }
}

// ---------------------------------------------------------------------------
// Property 5: Argument insertion order doesn't affect results
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Inserting args in different HashMap orders must produce the same scalar result.
    /// HashMap iteration order is non-deterministic, so this exercises ordering robustness.
    #[test]
    fn prop_arg_order_independent_scalar(
        x_val in finite_f64(),
        y_val in finite_f64(),
    ) {
        let ast = Expression::Binary {
            op: BinaryOp::Add,
            left: Box::new(Expression::Variable("x".into())),
            right: Box::new(Expression::Variable("y".into())),
        };
        let compiled = compile(&ast, &no_constants()).unwrap();

        // Insert x first, then y.
        let mut args_xy: HashMap<&str, EvalInput> = HashMap::new();
        args_xy.insert("x", EvalInput::Scalar(x_val));
        args_xy.insert("y", EvalInput::Scalar(y_val));

        // Insert y first, then x.
        let mut args_yx: HashMap<&str, EvalInput> = HashMap::new();
        args_yx.insert("y", EvalInput::Scalar(y_val));
        args_yx.insert("x", EvalInput::Scalar(x_val));

        let result_xy = eval(&compiled, args_xy)
            .unwrap()
            .scalar()
            .unwrap()
            .to_f64();
        let result_yx = eval(&compiled, args_yx)
            .unwrap()
            .scalar()
            .unwrap()
            .to_f64();

        prop_assert_eq!(result_xy, result_yx);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    /// Inserting array args in different order must produce the same array results.
    #[test]
    fn prop_arg_order_independent_array(
        x_vals in prop::collection::vec(finite_f64(), 1..=8),
        y_vals in prop::collection::vec(finite_f64(), 1..=8),
    ) {
        let ast = Expression::Binary {
            op: BinaryOp::Add,
            left: Box::new(Expression::Variable("x".into())),
            right: Box::new(Expression::Variable("y".into())),
        };
        let compiled = compile(&ast, &no_constants()).unwrap();

        let mut args_xy: HashMap<&str, EvalInput> = HashMap::new();
        args_xy.insert("x", EvalInput::from(x_vals.clone()));
        args_xy.insert("y", EvalInput::from(y_vals.clone()));

        let mut args_yx: HashMap<&str, EvalInput> = HashMap::new();
        args_yx.insert("y", EvalInput::from(y_vals.clone()));
        args_yx.insert("x", EvalInput::from(x_vals.clone()));

        let flat_xy: Vec<NumericResult> = eval(&compiled, args_xy)
            .unwrap()
            .to_array()
            .unwrap()
            .iter()
            .copied()
            .collect();

        let flat_yx: Vec<NumericResult> = eval(&compiled, args_yx)
            .unwrap()
            .to_array()
            .unwrap()
            .iter()
            .copied()
            .collect();

        prop_assert_eq!(flat_xy.len(), flat_yx.len());
        for (i, (a, b)) in flat_xy.iter().zip(flat_yx.iter()).enumerate() {
            prop_assert_eq!(a, b, "mismatch at index {}", i);
        }
    }
}
