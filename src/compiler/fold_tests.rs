use super::*;
use approx::assert_abs_diff_eq;
use mathlex::MathFloat;

fn int(v: i64) -> Expression {
    Expression::Integer(v)
}

fn var(name: &str) -> Expression {
    Expression::Variable(name.into())
}

fn float(v: f64) -> Expression {
    Expression::Float(MathFloat::from(v))
}

fn empty_constants() -> HashMap<&'static str, NumericResult> {
    HashMap::new()
}

#[test]
fn fold_integer_literal() {
    let expr = fold(&int(42), &empty_constants()).unwrap();
    assert!(matches!(expr.root, CompiledNode::Literal(v) if v == 42.0));
    assert!(expr.argument_names.is_empty());
}

#[test]
fn fold_float_literal() {
    let expr = fold(&float(2.75), &empty_constants()).unwrap();
    assert!(matches!(expr.root, CompiledNode::Literal(v) if (v - 2.75).abs() < 1e-10));
}

#[test]
fn fold_variable_becomes_argument() {
    let expr = fold(&var("x"), &empty_constants()).unwrap();
    assert!(matches!(expr.root, CompiledNode::Argument(0)));
    assert_eq!(expr.argument_names(), &["x"]);
}

#[test]
fn fold_two_variables_get_distinct_indices() {
    let ast = Expression::Binary {
        op: mathlex::BinaryOp::Add,
        left: Box::new(var("x")),
        right: Box::new(var("y")),
    };
    let expr = fold(&ast, &empty_constants()).unwrap();
    assert_eq!(expr.argument_names(), &["x", "y"]);
}

#[test]
fn fold_same_variable_reuses_index() {
    let ast = Expression::Binary {
        op: mathlex::BinaryOp::Add,
        left: Box::new(var("x")),
        right: Box::new(var("x")),
    };
    let expr = fold(&ast, &empty_constants()).unwrap();
    assert_eq!(expr.argument_names(), &["x"]);
}

#[test]
fn fold_constant_substitution() {
    let mut constants = HashMap::new();
    constants.insert("a", NumericResult::Real(5.0));
    let expr = fold(&var("a"), &constants).unwrap();
    assert!(matches!(expr.root, CompiledNode::Literal(v) if v == 5.0));
    assert!(expr.argument_names.is_empty());
}

#[test]
fn fold_pi_constant() {
    let ast = Expression::Constant(MathConstant::Pi);
    let expr = fold(&ast, &empty_constants()).unwrap();
    if let CompiledNode::Literal(v) = expr.root {
        assert_abs_diff_eq!(v, std::f64::consts::PI, epsilon = 1e-15);
    } else {
        panic!("expected literal");
    }
}

#[test]
fn fold_e_constant() {
    let ast = Expression::Constant(MathConstant::E);
    let expr = fold(&ast, &empty_constants()).unwrap();
    if let CompiledNode::Literal(v) = expr.root {
        assert_abs_diff_eq!(v, std::f64::consts::E, epsilon = 1e-15);
    } else {
        panic!("expected literal");
    }
}

#[test]
fn fold_imaginary_unit() {
    let ast = Expression::Constant(MathConstant::I);
    let expr = fold(&ast, &empty_constants()).unwrap();
    assert!(matches!(
        expr.root,
        CompiledNode::ComplexLiteral { re, im } if re == 0.0 && im == 1.0
    ));
    assert!(expr.is_complex());
}

#[test]
fn fold_constant_expression_folded() {
    let ast = Expression::Binary {
        op: mathlex::BinaryOp::Mul,
        left: Box::new(int(2)),
        right: Box::new(Expression::Constant(MathConstant::Pi)),
    };
    let expr = fold(&ast, &empty_constants()).unwrap();
    if let CompiledNode::Literal(v) = expr.root {
        assert_abs_diff_eq!(v, 2.0 * std::f64::consts::PI, epsilon = 1e-15);
    } else {
        panic!("expected folded literal, got {:?}", expr.root);
    }
}

#[test]
fn fold_mixed_constant_variable_not_folded() {
    let ast = Expression::Binary {
        op: mathlex::BinaryOp::Add,
        left: Box::new(var("x")),
        right: Box::new(int(1)),
    };
    let expr = fold(&ast, &empty_constants()).unwrap();
    assert!(matches!(expr.root, CompiledNode::Binary { .. }));
}

#[test]
fn fold_division_by_zero_error() {
    let ast = Expression::Binary {
        op: mathlex::BinaryOp::Div,
        left: Box::new(int(1)),
        right: Box::new(int(0)),
    };
    let err = fold(&ast, &empty_constants()).unwrap_err();
    assert!(matches!(err, CompileError::DivisionByZero));
}

#[test]
fn fold_unknown_function_error() {
    let ast = Expression::Function {
        name: "foobar".into(),
        args: vec![int(1)],
    };
    let err = fold(&ast, &empty_constants()).unwrap_err();
    assert!(matches!(err, CompileError::UnknownFunction { .. }));
}

#[test]
fn fold_arity_mismatch_error() {
    let ast = Expression::Function {
        name: "sin".into(),
        args: vec![int(1), int(2)],
    };
    let err = fold(&ast, &empty_constants()).unwrap_err();
    assert!(matches!(err, CompileError::ArityMismatch { .. }));
}

#[test]
fn fold_sum_basic() {
    let ast = Expression::Sum {
        index: "k".into(),
        lower: Box::new(int(1)),
        upper: Box::new(int(5)),
        body: Box::new(var("k")),
    };
    let expr = fold(&ast, &empty_constants()).unwrap();
    assert!(matches!(
        expr.root,
        CompiledNode::Sum {
            lower: 1,
            upper: 5,
            ..
        }
    ));
    assert!(expr.argument_names.is_empty());
}

#[test]
fn fold_sum_index_shadows_variable() {
    let ast = Expression::Binary {
        op: mathlex::BinaryOp::Add,
        left: Box::new(var("x")),
        right: Box::new(Expression::Sum {
            index: "x".into(),
            lower: Box::new(int(1)),
            upper: Box::new(int(3)),
            body: Box::new(var("x")),
        }),
    };
    let expr = fold(&ast, &empty_constants()).unwrap();
    assert_eq!(expr.argument_names(), &["x"]);
    if let CompiledNode::Binary { right, .. } = &expr.root {
        if let CompiledNode::Sum { body, .. } = right.as_ref() {
            assert!(matches!(body.as_ref(), CompiledNode::Index(_)));
        } else {
            panic!("expected Sum");
        }
    } else {
        panic!("expected Binary");
    }
}

#[test]
fn fold_sum_non_integer_bounds_error() {
    let ast = Expression::Sum {
        index: "k".into(),
        lower: Box::new(float(1.5)),
        upper: Box::new(int(5)),
        body: Box::new(var("k")),
    };
    let err = fold(&ast, &empty_constants()).unwrap_err();
    assert!(matches!(err, CompileError::NonIntegerBounds { .. }));
}

#[test]
fn fold_rational() {
    let ast = Expression::Rational {
        numerator: Box::new(int(3)),
        denominator: Box::new(int(4)),
    };
    let expr = fold(&ast, &empty_constants()).unwrap();
    if let CompiledNode::Literal(v) = expr.root {
        assert_abs_diff_eq!(v, 0.75, epsilon = 1e-15);
    } else {
        panic!("expected folded literal");
    }
}

#[test]
fn fold_function_with_literal_args_folded() {
    let ast = Expression::Function {
        name: "sin".into(),
        args: vec![int(0)],
    };
    let expr = fold(&ast, &empty_constants()).unwrap();
    if let CompiledNode::Literal(v) = expr.root {
        assert_abs_diff_eq!(v, 0.0, epsilon = 1e-15);
    } else {
        panic!("expected folded literal");
    }
}

#[test]
fn fold_function_with_variable_args_not_folded() {
    let ast = Expression::Function {
        name: "sin".into(),
        args: vec![var("x")],
    };
    let expr = fold(&ast, &empty_constants()).unwrap();
    assert!(matches!(expr.root, CompiledNode::Function { .. }));
}

#[test]
fn fold_factorial() {
    let ast = Expression::Unary {
        op: mathlex::UnaryOp::Factorial,
        operand: Box::new(int(5)),
    };
    let expr = fold(&ast, &empty_constants()).unwrap();
    if let CompiledNode::Literal(v) = expr.root {
        assert_abs_diff_eq!(v, 120.0, epsilon = 1e-10);
    } else {
        panic!("expected folded literal");
    }
}

#[test]
fn fold_negation() {
    let ast = Expression::Unary {
        op: mathlex::UnaryOp::Neg,
        operand: Box::new(int(5)),
    };
    let expr = fold(&ast, &empty_constants()).unwrap();
    assert!(matches!(expr.root, CompiledNode::Literal(v) if v == -5.0));
}

#[test]
fn fold_pos_is_identity() {
    let ast = Expression::Unary {
        op: mathlex::UnaryOp::Pos,
        operand: Box::new(int(5)),
    };
    let expr = fold(&ast, &empty_constants()).unwrap();
    assert!(matches!(expr.root, CompiledNode::Literal(v) if v == 5.0));
}

#[test]
fn fold_complex_constant_sets_flag() {
    let mut constants = HashMap::new();
    constants.insert(
        "z",
        NumericResult::Complex(num_complex::Complex::new(1.0, 2.0)),
    );
    let expr = fold(&var("z"), &constants).unwrap();
    assert!(expr.is_complex());
}
