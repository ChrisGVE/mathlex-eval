use std::collections::HashMap;

use mathlex::{BinaryOp, MathFloat, UnaryOp};

use mathlex_eval::{CompileError, NumericResult, compile};

fn int(v: i64) -> mathlex::Expression {
    mathlex::Expression::Integer(v)
}

fn var(name: &str) -> mathlex::Expression {
    mathlex::Expression::Variable(name.into())
}

fn float(v: f64) -> mathlex::Expression {
    mathlex::Expression::Float(MathFloat::from(v))
}

fn no_constants() -> HashMap<&'static str, NumericResult> {
    HashMap::new()
}

fn eval_scalar_no_args(compiled: &mathlex_eval::CompiledExpr) -> NumericResult {
    let handle = mathlex_eval::eval(compiled, HashMap::new()).unwrap();
    handle.scalar().unwrap()
}

// ---------------------------------------------------------------------------
// 6. Unknown function -> UnknownFunction
// ---------------------------------------------------------------------------

#[test]
fn unknown_function_error() {
    let ast = mathlex::Expression::Function {
        name: "frobnicate".into(),
        args: vec![var("x")],
    };
    let err = compile(&ast, &no_constants()).unwrap_err();
    match err {
        CompileError::UnknownFunction { name } => {
            assert_eq!(name, "frobnicate");
        }
        other => panic!("expected UnknownFunction, got {:?}", other),
    }
}

#[test]
fn unknown_function_empty_name_error() {
    let ast = mathlex::Expression::Function {
        name: "".into(),
        args: vec![],
    };
    let err = compile(&ast, &no_constants()).unwrap_err();
    assert!(matches!(err, CompileError::UnknownFunction { .. }));
}

// ---------------------------------------------------------------------------
// 7. Arity mismatch -> ArityMismatch
// ---------------------------------------------------------------------------

#[test]
fn arity_mismatch_sin_two_args() {
    let ast = mathlex::Expression::Function {
        name: "sin".into(),
        args: vec![var("x"), var("y")],
    };
    let err = compile(&ast, &no_constants()).unwrap_err();
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
        other => panic!("expected ArityMismatch, got {:?}", other),
    }
}

#[test]
fn arity_mismatch_atan2_one_arg() {
    let ast = mathlex::Expression::Function {
        name: "atan2".into(),
        args: vec![var("y")],
    };
    let err = compile(&ast, &no_constants()).unwrap_err();
    match err {
        CompileError::ArityMismatch {
            function,
            expected,
            got,
        } => {
            assert_eq!(function, "atan2");
            assert_eq!(expected, 2);
            assert_eq!(got, 1);
        }
        other => panic!("expected ArityMismatch, got {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// 8. Sum/product non-integer bounds -> NonIntegerBounds
// ---------------------------------------------------------------------------

#[test]
fn sum_float_lower_bound_rejected() {
    let ast = mathlex::Expression::Sum {
        index: "k".into(),
        lower: Box::new(float(1.5)),
        upper: Box::new(int(5)),
        body: Box::new(var("k")),
    };
    let err = compile(&ast, &no_constants()).unwrap_err();
    match err {
        CompileError::NonIntegerBounds { construct, .. } => {
            assert_eq!(construct, "sum");
        }
        other => panic!("expected NonIntegerBounds, got {:?}", other),
    }
}

#[test]
fn sum_float_upper_bound_rejected() {
    let ast = mathlex::Expression::Sum {
        index: "k".into(),
        lower: Box::new(int(1)),
        upper: Box::new(float(4.9)),
        body: Box::new(var("k")),
    };
    let err = compile(&ast, &no_constants()).unwrap_err();
    assert!(matches!(err, CompileError::NonIntegerBounds { .. }));
}

#[test]
fn product_float_lower_bound_rejected() {
    let ast = mathlex::Expression::Product {
        index: "k".into(),
        lower: Box::new(float(0.5)),
        upper: Box::new(int(4)),
        body: Box::new(var("k")),
    };
    let err = compile(&ast, &no_constants()).unwrap_err();
    match err {
        CompileError::NonIntegerBounds { construct, .. } => {
            assert_eq!(construct, "product");
        }
        other => panic!("expected NonIntegerBounds, got {:?}", other),
    }
}

#[test]
fn sum_integer_bounds_accepted() {
    let ast = mathlex::Expression::Sum {
        index: "k".into(),
        lower: Box::new(int(1)),
        upper: Box::new(int(5)),
        body: Box::new(var("k")),
    };
    assert!(compile(&ast, &no_constants()).is_ok());
}

// ---------------------------------------------------------------------------
// 9. Sum/product index scoping: index shadows outer variable of same name
// ---------------------------------------------------------------------------

#[test]
fn sum_index_shadows_outer_variable_of_same_name() {
    let ast = mathlex::Expression::Binary {
        op: BinaryOp::Add,
        left: Box::new(var("x")),
        right: Box::new(mathlex::Expression::Sum {
            index: "x".into(),
            lower: Box::new(int(1)),
            upper: Box::new(int(3)),
            body: Box::new(var("x")),
        }),
    };
    let compiled = compile(&ast, &no_constants()).unwrap();
    assert_eq!(compiled.argument_names(), &["x"]);
}

#[test]
fn sum_index_scoping_eval_correctness() {
    use approx::assert_abs_diff_eq;
    let ast = mathlex::Expression::Sum {
        index: "k".into(),
        lower: Box::new(int(1)),
        upper: Box::new(int(4)),
        body: Box::new(var("k")),
    };
    let compiled = compile(&ast, &no_constants()).unwrap();
    assert!(compiled.argument_names().is_empty());
    let result = eval_scalar_no_args(&compiled);
    assert_abs_diff_eq!(result.to_f64().unwrap(), 10.0, epsilon = 1e-12);
}

#[test]
fn product_index_scoping_eval_correctness() {
    use approx::assert_abs_diff_eq;
    let ast = mathlex::Expression::Product {
        index: "k".into(),
        lower: Box::new(int(1)),
        upper: Box::new(int(4)),
        body: Box::new(var("k")),
    };
    let compiled = compile(&ast, &no_constants()).unwrap();
    assert!(compiled.argument_names().is_empty());
    let result = eval_scalar_no_args(&compiled);
    assert_abs_diff_eq!(result.to_f64().unwrap(), 24.0, epsilon = 1e-12);
}

#[test]
fn sum_index_does_not_leak_outside_scope() {
    let ast = mathlex::Expression::Binary {
        op: BinaryOp::Add,
        left: Box::new(var("x")),
        right: Box::new(mathlex::Expression::Sum {
            index: "k".into(),
            lower: Box::new(int(1)),
            upper: Box::new(int(3)),
            body: Box::new(var("k")),
        }),
    };
    let compiled = compile(&ast, &no_constants()).unwrap();
    assert_eq!(compiled.argument_names(), &["x"]);
    assert!(!compiled.argument_names().contains(&"k".to_string()));
}

// ---------------------------------------------------------------------------
// 10. PlusMinus/MinusPlus rejected
// ---------------------------------------------------------------------------

#[test]
fn plus_minus_rejected_as_unsupported() {
    let ast = mathlex::Expression::Binary {
        op: BinaryOp::PlusMinus,
        left: Box::new(int(1)),
        right: Box::new(int(2)),
    };
    let err = compile(&ast, &no_constants()).unwrap_err();
    match &err {
        CompileError::UnsupportedExpression { variant, .. } => {
            assert!(
                variant.contains("PlusMinus"),
                "expected variant containing 'PlusMinus', got '{}'",
                variant
            );
        }
        other => panic!("expected UnsupportedExpression, got {:?}", other),
    }
}

#[test]
fn minus_plus_rejected_as_unsupported() {
    let ast = mathlex::Expression::Binary {
        op: BinaryOp::MinusPlus,
        left: Box::new(int(3)),
        right: Box::new(int(1)),
    };
    let err = compile(&ast, &no_constants()).unwrap_err();
    match &err {
        CompileError::UnsupportedExpression { variant, .. } => {
            assert!(
                variant.contains("MinusPlus"),
                "expected variant containing 'MinusPlus', got '{}'",
                variant
            );
        }
        other => panic!("expected UnsupportedExpression, got {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// 11. Transpose rejected
// ---------------------------------------------------------------------------

#[test]
fn transpose_rejected_as_unsupported() {
    let ast = mathlex::Expression::Unary {
        op: UnaryOp::Transpose,
        operand: Box::new(int(1)),
    };
    let err = compile(&ast, &no_constants()).unwrap_err();
    match &err {
        CompileError::UnsupportedExpression { variant, .. } => {
            assert!(
                variant.contains("Transpose"),
                "expected variant containing 'Transpose', got '{}'",
                variant
            );
        }
        other => panic!("expected UnsupportedExpression, got {:?}", other),
    }
}
