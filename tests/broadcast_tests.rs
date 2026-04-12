//! Integration tests for broadcasting semantics.
//!
//! Broadcasting rules:
//! - Scalar inputs contribute no axis to the output shape.
//! - Array inputs each contribute one axis; axis order follows argument
//!   declaration order (first appearance in the AST, left-to-right DFS).
//! - The output is the Cartesian product over all array axes, row-major.
//! - An empty array input produces an empty output.

use std::collections::HashMap;

use approx::assert_abs_diff_eq;
use mathlex::{BinaryOp, Expression};

use mathlex_eval::{EvalError, EvalInput, NumericResult, compile, eval};

// ---------------------------------------------------------------------------
// AST helpers
// ---------------------------------------------------------------------------

fn var(name: &str) -> Expression {
    Expression::Variable(name.into())
}

fn int(v: i64) -> Expression {
    Expression::Integer(v)
}

/// Build the AST for `x^2 + y`.
fn ast_x_sq_plus_y() -> Expression {
    let x_sq = Expression::Binary {
        op: BinaryOp::Pow,
        left: Box::new(var("x")),
        right: Box::new(int(2)),
    };
    Expression::Binary {
        op: BinaryOp::Add,
        left: Box::new(x_sq),
        right: Box::new(var("y")),
    }
}

/// Build the AST for `x + y`.
fn ast_x_plus_y() -> Expression {
    Expression::Binary {
        op: BinaryOp::Add,
        left: Box::new(var("x")),
        right: Box::new(var("y")),
    }
}

/// Build the AST for `x * 2`.
fn ast_x_times_two() -> Expression {
    Expression::Binary {
        op: BinaryOp::Mul,
        left: Box::new(var("x")),
        right: Box::new(int(2)),
    }
}

fn no_constants() -> HashMap<&'static str, NumericResult> {
    HashMap::new()
}

// ---------------------------------------------------------------------------
// 1. All scalar args → 0-d output (shape empty, len 1)
// ---------------------------------------------------------------------------

#[test]
fn all_scalars_produce_zero_d_output() {
    let ast = ast_x_sq_plus_y();
    let compiled = compile(&ast, &no_constants()).unwrap();

    let mut args = HashMap::new();
    args.insert("x", EvalInput::Scalar(3.0));
    args.insert("y", EvalInput::Scalar(1.0));

    let handle = eval(&compiled, args).unwrap();
    assert!(
        handle.shape().is_empty(),
        "expected empty shape for all-scalar args, got {:?}",
        handle.shape()
    );
    assert_eq!(handle.len(), 1);
}

#[test]
fn all_scalars_scalar_value_correct() {
    // x^2 + y = 3^2 + 1 = 10
    let ast = ast_x_sq_plus_y();
    let compiled = compile(&ast, &no_constants()).unwrap();

    let mut args = HashMap::new();
    args.insert("x", EvalInput::Scalar(3.0));
    args.insert("y", EvalInput::Scalar(1.0));

    let result = eval(&compiled, args).unwrap().scalar().unwrap();
    assert_abs_diff_eq!(result.to_f64().unwrap(), 10.0, epsilon = 1e-10);
}

// ---------------------------------------------------------------------------
// 2. One array arg → 1-d output
// ---------------------------------------------------------------------------

#[test]
fn one_array_arg_produces_1d_output() {
    let ast = ast_x_times_two();
    let compiled = compile(&ast, &no_constants()).unwrap();

    let mut args = HashMap::new();
    args.insert("x", EvalInput::from(vec![1.0, 2.0, 3.0]));

    let handle = eval(&compiled, args).unwrap();
    assert_eq!(handle.shape(), &[3]);
    assert_eq!(handle.len(), 3);
}

#[test]
fn one_array_arg_values_correct() {
    // x * 2 over [1, 2, 3] → [2, 4, 6]
    let ast = ast_x_times_two();
    let compiled = compile(&ast, &no_constants()).unwrap();

    let mut args = HashMap::new();
    args.insert("x", EvalInput::from(vec![1.0, 2.0, 3.0]));

    let arr = eval(&compiled, args).unwrap().to_array().unwrap();
    assert_eq!(arr.shape(), &[3]);
    assert_eq!(*arr.get([0]).unwrap(), NumericResult::Real(2.0));
    assert_eq!(*arr.get([1]).unwrap(), NumericResult::Real(4.0));
    assert_eq!(*arr.get([2]).unwrap(), NumericResult::Real(6.0));
}

// ---------------------------------------------------------------------------
// 3. Two array args → 2-d Cartesian product (verify exact grid values)
// ---------------------------------------------------------------------------

#[test]
fn two_array_args_produce_2d_output() {
    // x + y, x in [1, 2], y in [10, 20, 30]
    let ast = ast_x_plus_y();
    let compiled = compile(&ast, &no_constants()).unwrap();

    let mut args = HashMap::new();
    args.insert("x", EvalInput::from(vec![1.0, 2.0]));
    args.insert("y", EvalInput::from(vec![10.0, 20.0, 30.0]));

    let handle = eval(&compiled, args).unwrap();
    // axis 0 = x (2 elements), axis 1 = y (3 elements)
    assert_eq!(handle.shape(), &[2, 3]);
    assert_eq!(handle.len(), 6);
}

#[test]
fn two_array_args_grid_values_correct() {
    // x^2 + y, x in [1, 2, 3], y in [10, 20]
    // Expected grid (row = x-index, col = y-index):
    //   (1^2+10, 1^2+20) = (11, 21)
    //   (2^2+10, 2^2+20) = (14, 24)
    //   (3^2+10, 3^2+20) = (19, 29)
    let ast = ast_x_sq_plus_y();
    let compiled = compile(&ast, &no_constants()).unwrap();

    let mut args = HashMap::new();
    args.insert("x", EvalInput::from(vec![1.0, 2.0, 3.0]));
    args.insert("y", EvalInput::from(vec![10.0, 20.0]));

    let arr = eval(&compiled, args).unwrap().to_array().unwrap();
    assert_eq!(arr.shape(), &[3, 2]);

    assert_eq!(*arr.get([0, 0]).unwrap(), NumericResult::Real(11.0));
    assert_eq!(*arr.get([0, 1]).unwrap(), NumericResult::Real(21.0));
    assert_eq!(*arr.get([1, 0]).unwrap(), NumericResult::Real(14.0));
    assert_eq!(*arr.get([1, 1]).unwrap(), NumericResult::Real(24.0));
    assert_eq!(*arr.get([2, 0]).unwrap(), NumericResult::Real(19.0));
    assert_eq!(*arr.get([2, 1]).unwrap(), NumericResult::Real(29.0));
}

// ---------------------------------------------------------------------------
// 4. Mixed scalar + array → broadcast scalar over every array position
// ---------------------------------------------------------------------------

#[test]
fn scalar_broadcasts_over_array() {
    // x^2 + y, x = array [1, 2, 3], y = scalar 0
    // Expected: [1, 4, 9]
    let ast = ast_x_sq_plus_y();
    let compiled = compile(&ast, &no_constants()).unwrap();

    let mut args = HashMap::new();
    args.insert("x", EvalInput::from(vec![1.0, 2.0, 3.0]));
    args.insert("y", EvalInput::Scalar(0.0));

    let handle = eval(&compiled, args).unwrap();
    assert_eq!(handle.shape(), &[3]);

    let arr = handle.to_array().unwrap();
    assert_eq!(*arr.get([0]).unwrap(), NumericResult::Real(1.0));
    assert_eq!(*arr.get([1]).unwrap(), NumericResult::Real(4.0));
    assert_eq!(*arr.get([2]).unwrap(), NumericResult::Real(9.0));
}

#[test]
fn scalar_broadcasts_over_2d_result() {
    // x + y + c, with c as a compile-time constant; x and y both arrays
    // Use AST: x + y where c = 100 supplied as a constant
    let ast = Expression::Binary {
        op: BinaryOp::Add,
        left: Box::new(ast_x_plus_y()),
        right: Box::new(var("c")),
    };
    let mut constants = HashMap::new();
    constants.insert("c", NumericResult::Real(100.0));
    let compiled = compile(&ast, &constants).unwrap();

    let mut args = HashMap::new();
    args.insert("x", EvalInput::from(vec![1.0, 2.0]));
    args.insert("y", EvalInput::from(vec![10.0, 20.0]));

    let arr = eval(&compiled, args).unwrap().to_array().unwrap();
    assert_eq!(arr.shape(), &[2, 2]);
    // (1+10+100, 1+20+100) = (111, 121)
    // (2+10+100, 2+20+100) = (112, 122)
    assert_eq!(*arr.get([0, 0]).unwrap(), NumericResult::Real(111.0));
    assert_eq!(*arr.get([0, 1]).unwrap(), NumericResult::Real(121.0));
    assert_eq!(*arr.get([1, 0]).unwrap(), NumericResult::Real(112.0));
    assert_eq!(*arr.get([1, 1]).unwrap(), NumericResult::Real(122.0));
}

// ---------------------------------------------------------------------------
// 5. Shape inspection before consumption
// ---------------------------------------------------------------------------

#[test]
fn shape_inspectable_before_consumption() {
    let ast = ast_x_sq_plus_y();
    let compiled = compile(&ast, &no_constants()).unwrap();

    let mut args = HashMap::new();
    args.insert("x", EvalInput::from(vec![1.0, 2.0, 3.0]));
    args.insert("y", EvalInput::from(vec![10.0, 20.0]));

    let handle = eval(&compiled, args).unwrap();
    // Shape and len are observable before consuming the handle.
    let shape = handle.shape().to_vec();
    let len = handle.len();
    let is_empty = handle.is_empty();

    assert_eq!(shape, vec![3, 2]);
    assert_eq!(len, 6);
    assert!(!is_empty);

    // Now consume — values must still be correct.
    let arr = handle.to_array().unwrap();
    assert_eq!(arr.shape(), &[3, 2]);
}

// ---------------------------------------------------------------------------
// 6. Eager (to_array) and lazy (iter) produce identical results
// ---------------------------------------------------------------------------

#[test]
fn eager_and_lazy_produce_identical_results_1d() {
    let ast = ast_x_sq_plus_y();
    let compiled = compile(&ast, &no_constants()).unwrap();

    let make_args = || {
        let mut args = HashMap::new();
        args.insert("x", EvalInput::from(vec![1.0, 2.0, 3.0]));
        args.insert("y", EvalInput::Scalar(10.0));
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

    assert_eq!(eager, lazy);
}

#[test]
fn eager_and_lazy_produce_identical_results_2d() {
    let ast = ast_x_sq_plus_y();
    let compiled = compile(&ast, &no_constants()).unwrap();

    let make_args = || {
        let mut args = HashMap::new();
        args.insert("x", EvalInput::from(vec![1.0, 2.0, 3.0]));
        args.insert("y", EvalInput::from(vec![10.0, 20.0]));
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

    assert_eq!(eager, lazy);
    assert_eq!(eager.len(), 6);
}

// ---------------------------------------------------------------------------
// 7. Empty array input → empty output (is_empty true, shape [0])
// ---------------------------------------------------------------------------

#[test]
fn empty_array_produces_empty_output() {
    let ast = ast_x_times_two();
    let compiled = compile(&ast, &no_constants()).unwrap();

    let mut args = HashMap::new();
    args.insert("x", EvalInput::from(vec![] as Vec<f64>));

    let handle = eval(&compiled, args).unwrap();
    assert!(handle.is_empty());
    assert_eq!(handle.len(), 0);
    assert_eq!(handle.shape(), &[0]);
}

#[test]
fn empty_array_to_array_returns_empty_array() {
    let ast = ast_x_times_two();
    let compiled = compile(&ast, &no_constants()).unwrap();

    let mut args = HashMap::new();
    args.insert("x", EvalInput::from(vec![] as Vec<f64>));

    let arr = eval(&compiled, args).unwrap().to_array().unwrap();
    assert_eq!(arr.len(), 0);
}

#[test]
fn empty_array_iter_yields_no_elements() {
    let ast = ast_x_times_two();
    let compiled = compile(&ast, &no_constants()).unwrap();

    let mut args = HashMap::new();
    args.insert("x", EvalInput::from(vec![] as Vec<f64>));

    let items: Vec<_> = eval(&compiled, args).unwrap().iter().collect();
    assert!(items.is_empty());
}

#[test]
fn one_empty_array_with_nonempty_array_produces_empty_output() {
    // Cartesian product with an empty factor is always empty.
    let ast = ast_x_plus_y();
    let compiled = compile(&ast, &no_constants()).unwrap();

    let mut args = HashMap::new();
    args.insert("x", EvalInput::from(vec![] as Vec<f64>));
    args.insert("y", EvalInput::from(vec![1.0, 2.0, 3.0]));

    let handle = eval(&compiled, args).unwrap();
    assert!(handle.is_empty());
}

// ---------------------------------------------------------------------------
// 8. Iterator input: single iterator, verify materialized correctly
// ---------------------------------------------------------------------------

#[test]
fn iter_input_materialized_to_correct_values() {
    // x * 2 where x comes from an iterator over [5, 6, 7]
    let ast = ast_x_times_two();
    let compiled = compile(&ast, &no_constants()).unwrap();

    let mut args = HashMap::new();
    args.insert(
        "x",
        EvalInput::Iter(Box::new(vec![5.0, 6.0, 7.0].into_iter())),
    );

    let arr = eval(&compiled, args).unwrap().to_array().unwrap();
    assert_eq!(arr.shape(), &[3]);
    assert_eq!(*arr.get([0]).unwrap(), NumericResult::Real(10.0));
    assert_eq!(*arr.get([1]).unwrap(), NumericResult::Real(12.0));
    assert_eq!(*arr.get([2]).unwrap(), NumericResult::Real(14.0));
}

#[test]
fn iter_input_shape_matches_array_input() {
    // An iterator over N elements should produce the same shape as a Vec of N elements.
    let ast = ast_x_times_two();
    let compiled = compile(&ast, &no_constants()).unwrap();

    let values = vec![1.0, 2.0, 3.0, 4.0];

    let mut args_iter = HashMap::new();
    args_iter.insert("x", EvalInput::Iter(Box::new(values.clone().into_iter())));

    let mut args_arr = HashMap::new();
    args_arr.insert("x", EvalInput::from(values));

    let shape_from_iter = eval(&compiled, args_iter).unwrap().shape().to_vec();
    let shape_from_arr = eval(&compiled, args_arr).unwrap().shape().to_vec();

    assert_eq!(shape_from_iter, shape_from_arr);
}

#[test]
fn iter_input_combined_with_array_produces_2d() {
    // x + y where x comes from iterator, y from array
    let ast = ast_x_plus_y();
    let compiled = compile(&ast, &no_constants()).unwrap();

    let mut args = HashMap::new();
    args.insert("x", EvalInput::Iter(Box::new(vec![1.0, 2.0].into_iter())));
    args.insert("y", EvalInput::from(vec![10.0, 20.0]));

    let handle = eval(&compiled, args).unwrap();
    assert_eq!(handle.shape(), &[2, 2]);
    assert_eq!(handle.len(), 4);
}

// ---------------------------------------------------------------------------
// 9. scalar() on non-scalar output → ShapeMismatch error
// ---------------------------------------------------------------------------

#[test]
fn scalar_on_1d_output_returns_shape_mismatch() {
    let ast = ast_x_times_two();
    let compiled = compile(&ast, &no_constants()).unwrap();

    let mut args = HashMap::new();
    args.insert("x", EvalInput::from(vec![1.0, 2.0, 3.0]));

    let handle = eval(&compiled, args).unwrap();
    let err = handle.scalar().unwrap_err();
    assert!(
        matches!(err, EvalError::ShapeMismatch { .. }),
        "expected ShapeMismatch, got {:?}",
        err
    );
}

#[test]
fn scalar_on_2d_output_returns_shape_mismatch() {
    let ast = ast_x_plus_y();
    let compiled = compile(&ast, &no_constants()).unwrap();

    let mut args = HashMap::new();
    args.insert("x", EvalInput::from(vec![1.0, 2.0]));
    args.insert("y", EvalInput::from(vec![10.0, 20.0]));

    let handle = eval(&compiled, args).unwrap();
    let err = handle.scalar().unwrap_err();
    assert!(matches!(err, EvalError::ShapeMismatch { .. }));
}
