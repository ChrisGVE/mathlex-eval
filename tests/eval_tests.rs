use std::collections::HashMap;

use approx::assert_abs_diff_eq;
use mathlex::{BinaryOp, Expression, MathConstant};

use mathlex_eval::{EvalError, EvalInput, NumericResult, compile, eval};

// ---------------------------------------------------------------------------
// AST builder helpers
// ---------------------------------------------------------------------------

fn int(v: i64) -> Expression {
    Expression::Integer(v)
}

fn var(name: &str) -> Expression {
    Expression::Variable(name.into())
}

fn binop(op: BinaryOp, left: Expression, right: Expression) -> Expression {
    Expression::Binary {
        op,
        left: Box::new(left),
        right: Box::new(right),
    }
}

fn func(name: &str, args: Vec<Expression>) -> Expression {
    Expression::Function {
        name: name.into(),
        args,
    }
}

fn no_constants() -> HashMap<&'static str, NumericResult> {
    HashMap::new()
}

/// Compile `ast`, evaluate with `args`, and return the scalar result.
fn eval_scalar(
    ast: &Expression,
    args: HashMap<&str, EvalInput>,
) -> Result<NumericResult, EvalError> {
    let compiled = compile(ast, &no_constants()).expect("compile failed");
    let handle = eval(&compiled, args)?;
    handle.scalar()
}

/// Evaluate a no-argument AST directly (compile-time constants only).
fn eval_const(ast: &Expression) -> f64 {
    let compiled = compile(ast, &no_constants()).expect("compile failed");
    let handle = eval(&compiled, HashMap::new()).expect("eval failed");
    handle
        .scalar()
        .expect("scalar failed")
        .to_f64()
        .expect("expected real result")
}

/// Build `args` for a single variable.
fn args1(name: &str, v: f64) -> HashMap<&str, EvalInput> {
    let mut m = HashMap::new();
    m.insert(name, EvalInput::Scalar(v));
    m
}

/// Build `args` for two variables.
fn args2<'a>(n1: &'a str, v1: f64, n2: &'a str, v2: f64) -> HashMap<&'a str, EvalInput> {
    let mut m = HashMap::new();
    m.insert(n1, EvalInput::Scalar(v1));
    m.insert(n2, EvalInput::Scalar(v2));
    m
}

// ---------------------------------------------------------------------------
// 1. Arithmetic: add, sub, mul, div, pow, mod
// ---------------------------------------------------------------------------

#[test]
fn arithmetic_add() {
    // x + y with x=3, y=4 → 7
    let ast = binop(BinaryOp::Add, var("x"), var("y"));
    let result = eval_scalar(&ast, args2("x", 3.0, "y", 4.0)).unwrap();
    assert_abs_diff_eq!(result.to_f64().unwrap(), 7.0, epsilon = 1e-10);
}

#[test]
fn arithmetic_sub() {
    // x - y with x=10, y=3 → 7
    let ast = binop(BinaryOp::Sub, var("x"), var("y"));
    let result = eval_scalar(&ast, args2("x", 10.0, "y", 3.0)).unwrap();
    assert_abs_diff_eq!(result.to_f64().unwrap(), 7.0, epsilon = 1e-10);
}

#[test]
fn arithmetic_mul() {
    // x * y with x=6, y=7 → 42
    let ast = binop(BinaryOp::Mul, var("x"), var("y"));
    let result = eval_scalar(&ast, args2("x", 6.0, "y", 7.0)).unwrap();
    assert_abs_diff_eq!(result.to_f64().unwrap(), 42.0, epsilon = 1e-10);
}

#[test]
fn arithmetic_div() {
    // x / y with x=15, y=4 → 3.75
    let ast = binop(BinaryOp::Div, var("x"), var("y"));
    let result = eval_scalar(&ast, args2("x", 15.0, "y", 4.0)).unwrap();
    assert_abs_diff_eq!(result.to_f64().unwrap(), 3.75, epsilon = 1e-10);
}

#[test]
fn arithmetic_pow() {
    // x ^ y with x=2, y=10 → 1024
    let ast = binop(BinaryOp::Pow, var("x"), var("y"));
    let result = eval_scalar(&ast, args2("x", 2.0, "y", 10.0)).unwrap();
    assert_abs_diff_eq!(result.to_f64().unwrap(), 1024.0, epsilon = 1e-10);
}

#[test]
fn arithmetic_mod() {
    // x % y with x=17, y=5 → 2
    let ast = binop(BinaryOp::Mod, var("x"), var("y"));
    let result = eval_scalar(&ast, args2("x", 17.0, "y", 5.0)).unwrap();
    assert_abs_diff_eq!(result.to_f64().unwrap(), 2.0, epsilon = 1e-10);
}

// ---------------------------------------------------------------------------
// 2. Built-in functions against known values
// ---------------------------------------------------------------------------

#[test]
fn builtin_sin_pi_over_2() {
    // sin(π/2) = 1
    let ast = func(
        "sin",
        vec![binop(
            BinaryOp::Div,
            Expression::Constant(MathConstant::Pi),
            int(2),
        )],
    );
    assert_abs_diff_eq!(eval_const(&ast), 1.0, epsilon = 1e-10);
}

#[test]
fn builtin_cos_zero() {
    // cos(0) = 1
    let ast = func("cos", vec![int(0)]);
    assert_abs_diff_eq!(eval_const(&ast), 1.0, epsilon = 1e-10);
}

#[test]
fn builtin_exp_zero() {
    // exp(0) = 1
    let ast = func("exp", vec![int(0)]);
    assert_abs_diff_eq!(eval_const(&ast), 1.0, epsilon = 1e-10);
}

#[test]
fn builtin_exp_one() {
    // exp(1) = e
    let ast = func("exp", vec![int(1)]);
    assert_abs_diff_eq!(eval_const(&ast), std::f64::consts::E, epsilon = 1e-10);
}

#[test]
fn builtin_ln_e() {
    // ln(e) = 1
    let ast = func("ln", vec![Expression::Constant(MathConstant::E)]);
    assert_abs_diff_eq!(eval_const(&ast), 1.0, epsilon = 1e-10);
}

#[test]
fn builtin_sqrt_four() {
    // sqrt(4) = 2
    let ast = func("sqrt", vec![int(4)]);
    assert_abs_diff_eq!(eval_const(&ast), 2.0, epsilon = 1e-10);
}

#[test]
fn builtin_abs_negative() {
    // abs(-5) = 5
    let ast = func(
        "abs",
        vec![Expression::Unary {
            op: mathlex::UnaryOp::Neg,
            operand: Box::new(int(5)),
        }],
    );
    assert_abs_diff_eq!(eval_const(&ast), 5.0, epsilon = 1e-10);
}

#[test]
fn builtin_floor() {
    // floor(3.7): build as floor(x) with x = 3.7
    let ast = func("floor", vec![var("x")]);
    let compiled = compile(&ast, &no_constants()).unwrap();
    let handle = eval(&compiled, args1("x", 3.7)).unwrap();
    let result = handle.scalar().unwrap().to_f64().unwrap();
    assert_abs_diff_eq!(result, 3.0, epsilon = 1e-10);
}

#[test]
fn builtin_ceil() {
    // ceil(3.2): build as ceil(x) with x = 3.2
    let ast = func("ceil", vec![var("x")]);
    let compiled = compile(&ast, &no_constants()).unwrap();
    let handle = eval(&compiled, args1("x", 3.2)).unwrap();
    let result = handle.scalar().unwrap().to_f64().unwrap();
    assert_abs_diff_eq!(result, 4.0, epsilon = 1e-10);
}

#[test]
fn builtin_log2_eight() {
    // log2(8) = 3
    let ast = func("log2", vec![int(8)]);
    assert_abs_diff_eq!(eval_const(&ast), 3.0, epsilon = 1e-10);
}

#[test]
fn builtin_log10_thousand() {
    // log10(1000) = 3
    let ast = func("log10", vec![int(1000)]);
    assert_abs_diff_eq!(eval_const(&ast), 3.0, epsilon = 1e-10);
}

// ---------------------------------------------------------------------------
// 3. Rational: 3/4 → 0.75
// ---------------------------------------------------------------------------

#[test]
fn rational_three_quarters() {
    let ast = Expression::Rational {
        numerator: Box::new(int(3)),
        denominator: Box::new(int(4)),
    };
    assert_abs_diff_eq!(eval_const(&ast), 0.75, epsilon = 1e-15);
}

// ---------------------------------------------------------------------------
// 4. Nested: sin(x^2 + 1) with x=0 → sin(1)
// ---------------------------------------------------------------------------

#[test]
fn nested_sin_x_squared_plus_one() {
    let x_sq = binop(BinaryOp::Pow, var("x"), int(2));
    let inner = binop(BinaryOp::Add, x_sq, int(1));
    let ast = func("sin", vec![inner]);
    let result = eval_scalar(&ast, args1("x", 0.0)).unwrap();
    assert_abs_diff_eq!(result.to_f64().unwrap(), 1.0_f64.sin(), epsilon = 1e-10);
}

// ---------------------------------------------------------------------------
// 5. Sum: Σ_{k=1}^{5} k → 15
// ---------------------------------------------------------------------------

#[test]
fn sum_one_to_five() {
    let ast = Expression::Sum {
        index: "k".into(),
        lower: Box::new(int(1)),
        upper: Box::new(int(5)),
        body: Box::new(var("k")),
    };
    assert_abs_diff_eq!(eval_const(&ast), 15.0, epsilon = 1e-10);
}

// ---------------------------------------------------------------------------
// 6. Product: Π_{k=1}^{4} k → 24
// ---------------------------------------------------------------------------

#[test]
fn product_one_to_four() {
    let ast = Expression::Product {
        index: "k".into(),
        lower: Box::new(int(1)),
        upper: Box::new(int(4)),
        body: Box::new(var("k")),
    };
    assert_abs_diff_eq!(eval_const(&ast), 24.0, epsilon = 1e-10);
}

// ---------------------------------------------------------------------------
// 7. Division by zero → EvalError::DivisionByZero
// ---------------------------------------------------------------------------

#[test]
fn division_by_zero_at_eval_time() {
    // x / y compiled with y free; passing y=0 at eval time triggers EvalError
    let ast = binop(BinaryOp::Div, var("x"), var("y"));
    let compiled = compile(&ast, &no_constants()).unwrap();
    let handle = eval(&compiled, args2("x", 5.0, "y", 0.0)).unwrap();
    let err = handle.scalar().unwrap_err();
    assert!(
        matches!(err, EvalError::DivisionByZero),
        "expected DivisionByZero, got {err:?}"
    );
}

// ---------------------------------------------------------------------------
// 8. Unknown argument → EvalError::UnknownArgument
// ---------------------------------------------------------------------------

#[test]
fn unknown_argument_error() {
    // Compile x+1; pass z which is not expected
    let ast = binop(BinaryOp::Add, var("x"), int(1));
    let compiled = compile(&ast, &no_constants()).unwrap();
    let mut extra_args = HashMap::new();
    extra_args.insert("x", EvalInput::Scalar(1.0));
    extra_args.insert("z", EvalInput::Scalar(2.0));
    let err = eval(&compiled, extra_args).unwrap_err();
    assert!(
        matches!(err, EvalError::UnknownArgument { .. }),
        "expected UnknownArgument, got {err:?}"
    );
}

// ---------------------------------------------------------------------------
// 9. Missing argument → EvalError::MissingArgument
// ---------------------------------------------------------------------------

#[test]
fn missing_argument_error() {
    // Compile x + y; provide only x
    let ast = binop(BinaryOp::Add, var("x"), var("y"));
    let compiled = compile(&ast, &no_constants()).unwrap();
    let err = eval(&compiled, args1("x", 1.0)).unwrap_err();
    assert!(
        matches!(err, EvalError::MissingArgument { .. }),
        "expected MissingArgument, got {err:?}"
    );
}
