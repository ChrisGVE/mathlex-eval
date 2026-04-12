use std::collections::HashMap;

use approx::assert_abs_diff_eq;
use mathlex::{BinaryOp, Direction, MathConstant, MathFloat, UnaryOp};

use mathlex_eval::{CompileError, EvalInput, NumericResult, compile, eval};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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

/// Evaluate a no-argument compiled expression and unwrap the scalar result.
fn eval_scalar_no_args(compiled: &mathlex_eval::CompiledExpr) -> NumericResult {
    let handle = eval(compiled, HashMap::new()).unwrap();
    handle.scalar().unwrap()
}

/// Evaluate a compiled expression with a single real-scalar argument.
fn eval_scalar_one_arg(
    compiled: &mathlex_eval::CompiledExpr,
    name: &str,
    value: f64,
) -> NumericResult {
    let mut args: HashMap<&str, EvalInput> = HashMap::new();
    args.insert(name, EvalInput::Scalar(value));
    let handle = eval(compiled, args).unwrap();
    handle.scalar().unwrap()
}

// ---------------------------------------------------------------------------
// 1. Valid scalar ASTs compile successfully
// ---------------------------------------------------------------------------

#[test]
fn compile_integer_literal() {
    let compiled = compile(&int(42), &no_constants()).unwrap();
    assert!(compiled.argument_names().is_empty());
    assert!(!compiled.is_complex());
}

#[test]
fn compile_float_literal() {
    let compiled = compile(&float(2.75), &no_constants()).unwrap();
    assert!(compiled.argument_names().is_empty());
    assert!(!compiled.is_complex());
}

#[test]
fn compile_variable_becomes_argument() {
    let compiled = compile(&var("x"), &no_constants()).unwrap();
    assert_eq!(compiled.argument_names(), &["x"]);
    assert!(!compiled.is_complex());
}

#[test]
fn compile_binary_add_two_variables() {
    let ast = mathlex::Expression::Binary {
        op: BinaryOp::Add,
        left: Box::new(var("x")),
        right: Box::new(var("y")),
    };
    let compiled = compile(&ast, &no_constants()).unwrap();
    assert_eq!(compiled.argument_names(), &["x", "y"]);
}

#[test]
fn compile_unary_neg_variable() {
    let ast = mathlex::Expression::Unary {
        op: UnaryOp::Neg,
        operand: Box::new(var("x")),
    };
    let compiled = compile(&ast, &no_constants()).unwrap();
    assert_eq!(compiled.argument_names(), &["x"]);
}

#[test]
fn compile_unary_factorial_literal() {
    let ast = mathlex::Expression::Unary {
        op: UnaryOp::Factorial,
        operand: Box::new(int(4)),
    };
    let compiled = compile(&ast, &no_constants()).unwrap();
    assert!(compiled.argument_names().is_empty());
}

#[test]
fn compile_function_sin_variable() {
    let ast = mathlex::Expression::Function {
        name: "sin".into(),
        args: vec![var("x")],
    };
    let compiled = compile(&ast, &no_constants()).unwrap();
    assert_eq!(compiled.argument_names(), &["x"]);
}

// ---------------------------------------------------------------------------
// 2. Non-numerical variants -> UnsupportedExpression with correct variant name
// ---------------------------------------------------------------------------

fn assert_unsupported(ast: &mathlex::Expression, expected_variant: &str) {
    let err = compile(ast, &no_constants()).unwrap_err();
    match &err {
        CompileError::UnsupportedExpression { variant, .. } => {
            assert_eq!(
                *variant, expected_variant,
                "expected variant '{}', got '{}'",
                expected_variant, variant
            );
        }
        other => panic!("expected UnsupportedExpression, got {:?}", other),
    }
}

#[test]
fn unsupported_vector() {
    assert_unsupported(&mathlex::Expression::Vector(vec![int(1)]), "Vector");
}

#[test]
fn unsupported_matrix() {
    assert_unsupported(&mathlex::Expression::Matrix(vec![vec![int(1)]]), "Matrix");
}

#[test]
fn unsupported_derivative() {
    let ast = mathlex::Expression::Derivative {
        expr: Box::new(var("x")),
        var: "x".into(),
        order: 1,
    };
    assert_unsupported(&ast, "Derivative");
}

#[test]
fn unsupported_integral() {
    let ast = mathlex::Expression::Integral {
        integrand: Box::new(var("x")),
        var: "x".into(),
        bounds: None,
    };
    assert_unsupported(&ast, "Integral");
}

#[test]
fn unsupported_limit() {
    let ast = mathlex::Expression::Limit {
        expr: Box::new(var("x")),
        var: "x".into(),
        to: Box::new(int(0)),
        direction: Direction::Both,
    };
    assert_unsupported(&ast, "Limit");
}

#[test]
fn unsupported_equation() {
    let ast = mathlex::Expression::Equation {
        left: Box::new(var("x")),
        right: Box::new(int(5)),
    };
    assert_unsupported(&ast, "Equation");
}

#[test]
fn unsupported_nabla() {
    assert_unsupported(&mathlex::Expression::Nabla, "Nabla");
}

#[test]
fn unsupported_empty_set() {
    assert_unsupported(&mathlex::Expression::EmptySet, "EmptySet");
}

// ---------------------------------------------------------------------------
// 3. Constant substitution
// ---------------------------------------------------------------------------

#[test]
fn constant_substitution_removes_from_arguments() {
    let ast = mathlex::Expression::Binary {
        op: BinaryOp::Mul,
        left: Box::new(var("a")),
        right: Box::new(var("x")),
    };
    let mut constants = HashMap::new();
    constants.insert("a", NumericResult::Real(3.0));
    let compiled = compile(&ast, &constants).unwrap();
    assert_eq!(compiled.argument_names(), &["x"]);
}

#[test]
fn constant_substitution_eval_round_trip() {
    let ast = mathlex::Expression::Binary {
        op: BinaryOp::Mul,
        left: Box::new(var("a")),
        right: Box::new(var("x")),
    };
    let mut constants = HashMap::new();
    constants.insert("a", NumericResult::Real(4.0));
    let compiled = compile(&ast, &constants).unwrap();
    let result = eval_scalar_one_arg(&compiled, "x", 5.0);
    assert_abs_diff_eq!(result.to_f64().unwrap(), 20.0, epsilon = 1e-12);
}

#[test]
fn complex_constant_substitution_sets_is_complex() {
    use num_complex::Complex;
    let mut constants = HashMap::new();
    constants.insert("z", NumericResult::Complex(Complex::new(1.0, 2.0)));
    let compiled = compile(&var("z"), &constants).unwrap();
    assert!(compiled.is_complex());
    assert!(compiled.argument_names().is_empty());
}

// ---------------------------------------------------------------------------
// 4. Math constant resolution
// ---------------------------------------------------------------------------

#[test]
fn math_constant_pi_no_arguments() {
    let ast = mathlex::Expression::Constant(MathConstant::Pi);
    let compiled = compile(&ast, &no_constants()).unwrap();
    assert!(compiled.argument_names().is_empty());
    assert!(!compiled.is_complex());
}

#[test]
fn math_constant_pi_eval_value() {
    let ast = mathlex::Expression::Constant(MathConstant::Pi);
    let compiled = compile(&ast, &no_constants()).unwrap();
    let result = eval_scalar_no_args(&compiled);
    assert_abs_diff_eq!(
        result.to_f64().unwrap(),
        std::f64::consts::PI,
        epsilon = 1e-15
    );
}

#[test]
fn math_constant_e_eval_value() {
    let ast = mathlex::Expression::Constant(MathConstant::E);
    let compiled = compile(&ast, &no_constants()).unwrap();
    let result = eval_scalar_no_args(&compiled);
    assert_abs_diff_eq!(
        result.to_f64().unwrap(),
        std::f64::consts::E,
        epsilon = 1e-15
    );
}

#[test]
fn math_constant_i_sets_complex_flag() {
    let ast = mathlex::Expression::Constant(MathConstant::I);
    let compiled = compile(&ast, &no_constants()).unwrap();
    assert!(compiled.is_complex());
    assert!(compiled.argument_names().is_empty());
}

#[test]
fn math_constant_i_eval_is_imaginary_unit() {
    let ast = mathlex::Expression::Constant(MathConstant::I);
    let compiled = compile(&ast, &no_constants()).unwrap();
    let result = eval_scalar_no_args(&compiled);
    match result {
        NumericResult::Complex(c) => {
            assert_abs_diff_eq!(c.re, 0.0, epsilon = 1e-15);
            assert_abs_diff_eq!(c.im, 1.0, epsilon = 1e-15);
        }
        NumericResult::Real(_) => {
            panic!("imaginary unit should not evaluate to a real number");
        }
    }
}

// ---------------------------------------------------------------------------
// 5. Constant folding
// ---------------------------------------------------------------------------

#[test]
fn constant_folding_two_times_pi() {
    let ast = mathlex::Expression::Binary {
        op: BinaryOp::Mul,
        left: Box::new(int(2)),
        right: Box::new(mathlex::Expression::Constant(MathConstant::Pi)),
    };
    let compiled = compile(&ast, &no_constants()).unwrap();
    assert!(compiled.argument_names().is_empty());
    let result = eval_scalar_no_args(&compiled);
    assert_abs_diff_eq!(
        result.to_f64().unwrap(),
        2.0 * std::f64::consts::PI,
        epsilon = 1e-14
    );
}

#[test]
fn constant_folding_sin_of_zero() {
    let ast = mathlex::Expression::Function {
        name: "sin".into(),
        args: vec![int(0)],
    };
    let compiled = compile(&ast, &no_constants()).unwrap();
    assert!(compiled.argument_names().is_empty());
    let result = eval_scalar_no_args(&compiled);
    assert_abs_diff_eq!(result.to_f64().unwrap(), 0.0, epsilon = 1e-15);
}

#[test]
fn constant_folding_rational_three_quarters() {
    let ast = mathlex::Expression::Rational {
        numerator: Box::new(int(3)),
        denominator: Box::new(int(4)),
    };
    let compiled = compile(&ast, &no_constants()).unwrap();
    assert!(compiled.argument_names().is_empty());
    let result = eval_scalar_no_args(&compiled);
    assert_abs_diff_eq!(result.to_f64().unwrap(), 0.75, epsilon = 1e-15);
}
