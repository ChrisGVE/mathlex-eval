use std::collections::HashMap;

use mathlex::{BinaryOp, ExprKind, Expression, MathConstant, MathFloat, UnaryOp};

use mathlex_eval::{CompileError, EvalError, EvalInput, NumericResult, compile, eval};

fn int(v: i64) -> Expression {
    Expression::integer(v)
}

fn var(name: &str) -> Expression {
    Expression::variable(name)
}

// === CompileError variants ===

#[test]
fn compile_error_unsupported_vector() {
    let ast = Expression::vector(vec![int(1)]);
    let err = compile(&ast, &HashMap::new()).unwrap_err();
    match err {
        CompileError::UnsupportedExpression { variant, .. } => {
            assert_eq!(variant, "Vector");
        }
        _ => panic!("expected UnsupportedExpression, got {:?}", err),
    }
}

#[test]
fn compile_error_unsupported_matrix() {
    let ast = Expression::matrix(vec![vec![int(1)]]);
    let err = compile(&ast, &HashMap::new()).unwrap_err();
    match err {
        CompileError::UnsupportedExpression { variant, .. } => {
            assert_eq!(variant, "Matrix");
        }
        _ => panic!("expected UnsupportedExpression"),
    }
}

#[test]
fn compile_error_unsupported_derivative() {
    let ast = ExprKind::Derivative {
        expr: Box::new(var("x")),
        var: "x".into(),
        order: 1,
    }
    .into();
    let err = compile(&ast, &HashMap::new()).unwrap_err();
    assert!(matches!(err, CompileError::UnsupportedExpression { .. }));
}

#[test]
fn compile_error_unsupported_integral() {
    let ast = ExprKind::Integral {
        integrand: Box::new(var("x")),
        var: "x".into(),
        bounds: None,
    }
    .into();
    let err = compile(&ast, &HashMap::new()).unwrap_err();
    assert!(matches!(err, CompileError::UnsupportedExpression { .. }));
}

#[test]
fn compile_error_unsupported_nabla() {
    let err = compile(&Expression::nabla(), &HashMap::new()).unwrap_err();
    assert!(matches!(err, CompileError::UnsupportedExpression { .. }));
}

#[test]
fn compile_error_unsupported_empty_set() {
    let err = compile(&Expression::empty_set(), &HashMap::new()).unwrap_err();
    assert!(matches!(err, CompileError::UnsupportedExpression { .. }));
}

#[test]
fn compile_error_unsupported_plus_minus() {
    let ast = ExprKind::Binary {
        op: BinaryOp::PlusMinus,
        left: Box::new(int(1)),
        right: Box::new(int(2)),
    }
    .into();
    let err = compile(&ast, &HashMap::new()).unwrap_err();
    assert!(matches!(err, CompileError::UnsupportedExpression { .. }));
}

#[test]
fn compile_error_unsupported_transpose() {
    let ast = ExprKind::Unary {
        op: UnaryOp::Transpose,
        operand: Box::new(int(1)),
    }
    .into();
    let err = compile(&ast, &HashMap::new()).unwrap_err();
    assert!(matches!(err, CompileError::UnsupportedExpression { .. }));
}

#[test]
fn compile_error_unknown_function() {
    let ast = ExprKind::Function {
        name: "nonexistent_fn".into(),
        args: vec![int(1)],
    }
    .into();
    let err = compile(&ast, &HashMap::new()).unwrap_err();
    match err {
        CompileError::UnknownFunction { name } => {
            assert_eq!(name, "nonexistent_fn");
        }
        _ => panic!("expected UnknownFunction"),
    }
}

#[test]
fn compile_error_arity_mismatch() {
    let ast = ExprKind::Function {
        name: "sin".into(),
        args: vec![int(1), int(2)],
    }
    .into();
    let err = compile(&ast, &HashMap::new()).unwrap_err();
    match err {
        CompileError::ArityMismatch {
            function,
            expected,
            got,
        } => {
            assert_eq!(function, "sin");
            assert_eq!(expected, 1);
            assert_eq!(got, 2);
        }
        _ => panic!("expected ArityMismatch"),
    }
}

#[test]
fn compile_error_non_integer_bounds() {
    let ast = ExprKind::Sum {
        index: "k".into(),
        lower: Box::new(Expression::float(MathFloat::from(1.5))),
        upper: Box::new(int(5)),
        body: Box::new(var("k")),
    }
    .into();
    let err = compile(&ast, &HashMap::new()).unwrap_err();
    assert!(matches!(err, CompileError::NonIntegerBounds { .. }));
}

#[test]
fn compile_error_division_by_zero() {
    let ast = ExprKind::Binary {
        op: BinaryOp::Div,
        left: Box::new(int(1)),
        right: Box::new(int(0)),
    }
    .into();
    let err = compile(&ast, &HashMap::new()).unwrap_err();
    assert!(matches!(err, CompileError::DivisionByZero));
}

#[test]
fn compile_error_quaternion_constant() {
    let ast = Expression::constant(MathConstant::J);
    let err = compile(&ast, &HashMap::new()).unwrap_err();
    assert!(matches!(err, CompileError::UnsupportedExpression { .. }));
}

// === EvalError variants ===

#[test]
fn eval_error_unknown_argument() {
    let ast = ExprKind::Binary {
        op: BinaryOp::Add,
        left: Box::new(var("x")),
        right: Box::new(int(1)),
    }
    .into();
    let compiled = compile(&ast, &HashMap::new()).unwrap();
    let mut args = HashMap::new();
    args.insert("x", EvalInput::Scalar(1.0));
    args.insert("z", EvalInput::Scalar(2.0));
    let err = eval(&compiled, args).unwrap_err();
    match err {
        EvalError::UnknownArgument { name } => assert_eq!(name, "z"),
        _ => panic!("expected UnknownArgument"),
    }
}

#[test]
fn eval_error_missing_argument() {
    let ast = ExprKind::Binary {
        op: BinaryOp::Add,
        left: Box::new(var("x")),
        right: Box::new(var("y")),
    }
    .into();
    let compiled = compile(&ast, &HashMap::new()).unwrap();
    let mut args = HashMap::new();
    args.insert("x", EvalInput::Scalar(1.0));
    let err = eval(&compiled, args).unwrap_err();
    match err {
        EvalError::MissingArgument { name } => assert_eq!(name, "y"),
        _ => panic!("expected MissingArgument"),
    }
}

#[test]
fn eval_error_division_by_zero() {
    let ast = ExprKind::Binary {
        op: BinaryOp::Div,
        left: Box::new(var("x")),
        right: Box::new(var("y")),
    }
    .into();
    let compiled = compile(&ast, &HashMap::new()).unwrap();
    let mut args = HashMap::new();
    args.insert("x", EvalInput::Scalar(5.0));
    args.insert("y", EvalInput::Scalar(0.0));
    let handle = eval(&compiled, args).unwrap();
    let err = handle.scalar().unwrap_err();
    assert!(matches!(err, EvalError::DivisionByZero));
}

#[test]
fn eval_error_shape_mismatch_scalar_on_array() {
    let ast = var("x");
    let compiled = compile(&ast, &HashMap::new()).unwrap();
    let mut args = HashMap::new();
    args.insert("x", EvalInput::from(vec![1.0, 2.0, 3.0]));
    let handle = eval(&compiled, args).unwrap();
    let err = handle.scalar().unwrap_err();
    assert!(matches!(err, EvalError::ShapeMismatch { .. }));
}

#[test]
fn eval_per_element_error_in_iterator() {
    // 1/x with x=[2, 0, 4] → [Ok(0.5), Err(DivByZero), Ok(0.25)]
    let ast = ExprKind::Binary {
        op: BinaryOp::Div,
        left: Box::new(int(1)),
        right: Box::new(var("x")),
    }
    .into();
    let compiled = compile(&ast, &HashMap::new()).unwrap();
    let mut args = HashMap::new();
    args.insert("x", EvalInput::from(vec![2.0, 0.0, 4.0]));
    let handle = eval(&compiled, args).unwrap();
    let results: Vec<Result<NumericResult, EvalError>> = handle.iter().collect();
    assert!(results[0].is_ok());
    assert!(results[1].is_err());
    assert!(results[2].is_ok());
}
