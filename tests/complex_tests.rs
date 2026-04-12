use std::collections::HashMap;

use approx::assert_abs_diff_eq;
use mathlex::{BinaryOp, Expression, MathConstant};
use num_complex::Complex;

use mathlex_eval::{EvalInput, NumericResult, compile, eval};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn int(v: i64) -> Expression {
    Expression::Integer(v)
}

fn var(name: &str) -> Expression {
    Expression::Variable(name.into())
}

fn real_const(re: f64) -> Expression {
    Expression::Float(re.into())
}

fn no_constants() -> HashMap<&'static str, NumericResult> {
    HashMap::new()
}

fn assert_real(result: NumericResult, expected: f64, epsilon: f64) {
    assert!(
        !result.is_complex(),
        "expected Real but got Complex: {:?}",
        result
    );
    assert_abs_diff_eq!(result.to_f64().unwrap(), expected, epsilon = epsilon);
}

fn assert_complex_parts(result: NumericResult, re: f64, im: f64, epsilon: f64) {
    assert!(
        result.is_complex(),
        "expected Complex but got Real: {:?}",
        result
    );
    let c = result.to_complex();
    assert_abs_diff_eq!(c.re, re, epsilon = epsilon);
    assert_abs_diff_eq!(c.im, im, epsilon = epsilon);
}

// ---------------------------------------------------------------------------
// 1. Real inputs with real operations stay Real
// ---------------------------------------------------------------------------

#[test]
fn real_add_stays_real() {
    // x + y with real scalars → NumericResult::Real
    let ast = Expression::Binary {
        op: BinaryOp::Add,
        left: Box::new(var("x")),
        right: Box::new(var("y")),
    };
    let compiled = compile(&ast, &no_constants()).unwrap();
    assert!(!compiled.is_complex());

    let mut args = HashMap::new();
    args.insert("x", EvalInput::Scalar(3.0));
    args.insert("y", EvalInput::Scalar(4.0));
    let result = eval(&compiled, args).unwrap().scalar().unwrap();

    assert_real(result, 7.0, 1e-15);
}

#[test]
fn real_mul_stays_real() {
    // x * y with real scalars
    let ast = Expression::Binary {
        op: BinaryOp::Mul,
        left: Box::new(var("x")),
        right: Box::new(var("y")),
    };
    let compiled = compile(&ast, &no_constants()).unwrap();

    let mut args = HashMap::new();
    args.insert("x", EvalInput::Scalar(6.0));
    args.insert("y", EvalInput::Scalar(7.0));
    let result = eval(&compiled, args).unwrap().scalar().unwrap();

    assert_real(result, 42.0, 1e-15);
}

// ---------------------------------------------------------------------------
// 2. Imaginary unit constant → result promotes to Complex
// ---------------------------------------------------------------------------

#[test]
fn imaginary_unit_constant_is_complex() {
    // Expression: i  (the imaginary unit)
    let ast = Expression::Constant(MathConstant::I);
    let compiled = compile(&ast, &no_constants()).unwrap();

    assert!(
        compiled.is_complex(),
        "compiled expr should be flagged complex"
    );

    let result = eval(&compiled, HashMap::new()).unwrap().scalar().unwrap();

    assert_complex_parts(result, 0.0, 1.0, 1e-15);
}

#[test]
fn imaginary_unit_added_to_real_promotes() {
    // 1 + i
    let ast = Expression::Binary {
        op: BinaryOp::Add,
        left: Box::new(int(1)),
        right: Box::new(Expression::Constant(MathConstant::I)),
    };
    let compiled = compile(&ast, &no_constants()).unwrap();

    let result = eval(&compiled, HashMap::new()).unwrap().scalar().unwrap();

    assert_complex_parts(result, 1.0, 1.0, 1e-15);
}

// ---------------------------------------------------------------------------
// 3. sqrt(-1) → complex promotion
// ---------------------------------------------------------------------------

#[test]
fn sqrt_neg_one_produces_complex() {
    // sqrt(-1) via Function AST node
    let ast = Expression::Function {
        name: "sqrt".into(),
        args: vec![Expression::Unary {
            op: mathlex::UnaryOp::Neg,
            operand: Box::new(int(1)),
        }],
    };
    let compiled = compile(&ast, &no_constants()).unwrap();

    let result = eval(&compiled, HashMap::new()).unwrap().scalar().unwrap();

    assert!(result.is_complex(), "sqrt(-1) must produce Complex");
    let c = result.to_complex();
    assert_abs_diff_eq!(c.re, 0.0, epsilon = 1e-14);
    assert_abs_diff_eq!(c.im, 1.0, epsilon = 1e-14);
}

#[test]
fn sqrt_neg_four_produces_two_i() {
    // sqrt(-4) = 2i
    let ast = Expression::Function {
        name: "sqrt".into(),
        args: vec![Expression::Unary {
            op: mathlex::UnaryOp::Neg,
            operand: Box::new(int(4)),
        }],
    };
    let compiled = compile(&ast, &no_constants()).unwrap();
    let result = eval(&compiled, HashMap::new()).unwrap().scalar().unwrap();

    assert!(result.is_complex());
    let c = result.to_complex();
    assert_abs_diff_eq!(c.re, 0.0, epsilon = 1e-14);
    assert_abs_diff_eq!(c.im, 2.0, epsilon = 1e-14);
}

// ---------------------------------------------------------------------------
// 4. Complex arithmetic: (a+bi) * (c+di) — compile with complex constants
// ---------------------------------------------------------------------------

#[test]
fn complex_constant_multiplication() {
    // Expression: z * w, where z and w are complex constants
    // z = 1 + 2i, w = 3 + 4i → (1+2i)(3+4i) = 3+4i+6i+8i² = -5+10i
    let ast = Expression::Binary {
        op: BinaryOp::Mul,
        left: Box::new(var("z")),
        right: Box::new(var("w")),
    };
    let mut constants = HashMap::new();
    constants.insert("z", NumericResult::Complex(Complex::new(1.0, 2.0)));
    constants.insert("w", NumericResult::Complex(Complex::new(3.0, 4.0)));
    let compiled = compile(&ast, &constants).unwrap();

    let result = eval(&compiled, HashMap::new()).unwrap().scalar().unwrap();

    assert_complex_parts(result, -5.0, 10.0, 1e-13);
}

#[test]
fn complex_constant_addition() {
    // (2+3i) + (1-1i) = 3+2i
    let ast = Expression::Binary {
        op: BinaryOp::Add,
        left: Box::new(var("a")),
        right: Box::new(var("b")),
    };
    let mut constants = HashMap::new();
    constants.insert("a", NumericResult::Complex(Complex::new(2.0, 3.0)));
    constants.insert("b", NumericResult::Complex(Complex::new(1.0, -1.0)));
    let compiled = compile(&ast, &constants).unwrap();

    let result = eval(&compiled, HashMap::new()).unwrap().scalar().unwrap();

    assert_complex_parts(result, 3.0, 2.0, 1e-15);
}

#[test]
fn complex_ast_node_arithmetic() {
    // Build (2 + 3i) * (1 + 2i) via Expression::Complex AST nodes
    // (2+3i)(1+2i) = 2+4i+3i+6i² = 2+7i-6 = -4+7i
    let lhs = Expression::Complex {
        real: Box::new(real_const(2.0)),
        imaginary: Box::new(real_const(3.0)),
    };
    let rhs = Expression::Complex {
        real: Box::new(real_const(1.0)),
        imaginary: Box::new(real_const(2.0)),
    };
    let ast = Expression::Binary {
        op: BinaryOp::Mul,
        left: Box::new(lhs),
        right: Box::new(rhs),
    };
    let compiled = compile(&ast, &no_constants()).unwrap();
    assert!(compiled.is_complex());

    let result = eval(&compiled, HashMap::new()).unwrap().scalar().unwrap();

    assert_complex_parts(result, -4.0, 7.0, 1e-13);
}

// ---------------------------------------------------------------------------
// 5. Mixed real/complex arguments
// ---------------------------------------------------------------------------

#[test]
fn mixed_real_and_complex_inputs() {
    // Expression: x + y, where x is real scalar and y is complex scalar at eval time
    let ast = Expression::Binary {
        op: BinaryOp::Add,
        left: Box::new(var("x")),
        right: Box::new(var("y")),
    };
    let compiled = compile(&ast, &no_constants()).unwrap();

    let mut args = HashMap::new();
    args.insert("x", EvalInput::Scalar(5.0));
    args.insert("y", EvalInput::Complex(Complex::new(0.0, 3.0)));
    let result = eval(&compiled, args).unwrap().scalar().unwrap();

    // 5 + 3i
    assert_complex_parts(result, 5.0, 3.0, 1e-15);
}

#[test]
fn mixed_real_constant_and_complex_eval_input() {
    // Expression: k * x, where k is compile-time real constant and x is runtime complex
    let ast = Expression::Binary {
        op: BinaryOp::Mul,
        left: Box::new(var("k")),
        right: Box::new(var("x")),
    };
    let mut constants = HashMap::new();
    constants.insert("k", NumericResult::Real(2.0));
    let compiled = compile(&ast, &constants).unwrap();
    // Only x remains as argument
    assert_eq!(compiled.argument_names(), &["x"]);

    let mut args = HashMap::new();
    args.insert("x", EvalInput::Complex(Complex::new(3.0, 4.0)));
    let result = eval(&compiled, args).unwrap().scalar().unwrap();

    // 2 * (3+4i) = 6+8i
    assert_complex_parts(result, 6.0, 8.0, 1e-15);
}

// ---------------------------------------------------------------------------
// 6. Complex result that simplifies to real (i * i = -1)
// ---------------------------------------------------------------------------

#[test]
fn i_squared_simplifies_to_real() {
    // i² = -1 → the evaluator should simplify to NumericResult::Real(-1.0)
    let ast = Expression::Binary {
        op: BinaryOp::Mul,
        left: Box::new(Expression::Constant(MathConstant::I)),
        right: Box::new(Expression::Constant(MathConstant::I)),
    };
    let compiled = compile(&ast, &no_constants()).unwrap();

    let result = eval(&compiled, HashMap::new()).unwrap().scalar().unwrap();

    // Imaginary part is exactly 0; simplify() converts to Real
    assert!(
        !result.is_complex(),
        "i*i = -1 should simplify to Real, got {:?}",
        result
    );
    assert_real(result, -1.0, 1e-14);
}

#[test]
fn complex_conj_product_simplifies_to_real() {
    // (1+2i) * (1-2i) = 1 - 4i² = 1 + 4 = 5  (real)
    let ast = Expression::Binary {
        op: BinaryOp::Mul,
        left: Box::new(var("z")),
        right: Box::new(var("w")),
    };
    let mut constants = HashMap::new();
    constants.insert("z", NumericResult::Complex(Complex::new(1.0, 2.0)));
    constants.insert("w", NumericResult::Complex(Complex::new(1.0, -2.0)));
    let compiled = compile(&ast, &constants).unwrap();

    let result = eval(&compiled, HashMap::new()).unwrap().scalar().unwrap();

    // Imaginary part cancels → should simplify to Real(5)
    assert!(
        !result.is_complex(),
        "(1+2i)(1-2i) should simplify to Real, got {:?}",
        result
    );
    assert_real(result, 5.0, 1e-13);
}

// ---------------------------------------------------------------------------
// 7. is_complex flag on CompiledExpr when expression uses imaginary unit
// ---------------------------------------------------------------------------

#[test]
fn is_complex_flag_set_for_imaginary_constant() {
    let ast = Expression::Constant(MathConstant::I);
    let compiled = compile(&ast, &no_constants()).unwrap();
    assert!(compiled.is_complex());
}

#[test]
fn is_complex_flag_set_for_complex_ast_node() {
    let ast = Expression::Complex {
        real: Box::new(int(1)),
        imaginary: Box::new(int(2)),
    };
    let compiled = compile(&ast, &no_constants()).unwrap();
    assert!(compiled.is_complex());
}

#[test]
fn is_complex_flag_set_for_complex_numeric_constant() {
    // Compile x + k where k is a complex NumericResult constant
    let ast = Expression::Binary {
        op: BinaryOp::Add,
        left: Box::new(var("x")),
        right: Box::new(var("k")),
    };
    let mut constants = HashMap::new();
    constants.insert("k", NumericResult::Complex(Complex::new(0.0, 1.0)));
    let compiled = compile(&ast, &constants).unwrap();
    assert!(compiled.is_complex());
}

#[test]
fn is_complex_flag_not_set_for_pure_real_expression() {
    // x + 1 has no imaginary component
    let ast = Expression::Binary {
        op: BinaryOp::Add,
        left: Box::new(var("x")),
        right: Box::new(int(1)),
    };
    let compiled = compile(&ast, &no_constants()).unwrap();
    assert!(!compiled.is_complex());
}

// ---------------------------------------------------------------------------
// 8. ln(-1) → complex result (iπ)
// ---------------------------------------------------------------------------

#[test]
fn ln_negative_one_is_i_pi() {
    // ln(-1) = iπ  (principal branch)
    let ast = Expression::Function {
        name: "ln".into(),
        args: vec![Expression::Unary {
            op: mathlex::UnaryOp::Neg,
            operand: Box::new(int(1)),
        }],
    };
    let compiled = compile(&ast, &no_constants()).unwrap();
    let result = eval(&compiled, HashMap::new()).unwrap().scalar().unwrap();

    assert!(result.is_complex(), "ln(-1) must be Complex");
    let c = result.to_complex();
    assert_abs_diff_eq!(c.re, 0.0, epsilon = 1e-14);
    assert_abs_diff_eq!(c.im, std::f64::consts::PI, epsilon = 1e-14);
}

#[test]
fn ln_negative_e_is_one_plus_i_pi() {
    // ln(-e) = 1 + iπ
    let ast = Expression::Function {
        name: "ln".into(),
        args: vec![Expression::Unary {
            op: mathlex::UnaryOp::Neg,
            operand: Box::new(Expression::Constant(MathConstant::E)),
        }],
    };
    let compiled = compile(&ast, &no_constants()).unwrap();
    let result = eval(&compiled, HashMap::new()).unwrap().scalar().unwrap();

    assert!(result.is_complex(), "ln(-e) must be Complex");
    let c = result.to_complex();
    assert_abs_diff_eq!(c.re, 1.0, epsilon = 1e-14);
    assert_abs_diff_eq!(c.im, std::f64::consts::PI, epsilon = 1e-14);
}
