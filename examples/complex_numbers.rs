//! Complex number example: real/complex promotion and complex constants.

use std::collections::HashMap;

use mathlex::{BinaryOp, Expression, MathConstant};
use num_complex::Complex;

use mathlex_eval::{EvalInput, NumericResult, compile, eval};

fn main() {
    // Example 1: sqrt(-1) → complex
    println!("=== sqrt(-1) ===");
    let ast = Expression::Function {
        name: "sqrt".into(),
        args: vec![Expression::Unary {
            op: mathlex::UnaryOp::Neg,
            operand: Box::new(Expression::Integer(1)),
        }],
    };
    let compiled = compile(&ast, &HashMap::new()).expect("compile failed");
    let result = eval(&compiled, HashMap::new())
        .expect("eval failed")
        .scalar()
        .expect("scalar failed");
    println!("sqrt(-1) = {:?}", result);

    // Example 2: Expression using imaginary unit i
    println!("\n=== 1 + 2i ===");
    let ast = Expression::Binary {
        op: BinaryOp::Add,
        left: Box::new(Expression::Integer(1)),
        right: Box::new(Expression::Binary {
            op: BinaryOp::Mul,
            left: Box::new(Expression::Integer(2)),
            right: Box::new(Expression::Constant(MathConstant::I)),
        }),
    };
    let compiled = compile(&ast, &HashMap::new()).expect("compile failed");
    println!("is_complex: {}", compiled.is_complex());
    let result = eval(&compiled, HashMap::new())
        .expect("eval failed")
        .scalar()
        .expect("scalar failed");
    println!("1 + 2i = {:?}", result);

    // Example 3: Evaluate with complex argument
    println!("\n=== x^2 with x = 1+i ===");
    let ast = Expression::Binary {
        op: BinaryOp::Pow,
        left: Box::new(Expression::Variable("x".into())),
        right: Box::new(Expression::Integer(2)),
    };
    let compiled = compile(&ast, &HashMap::new()).expect("compile failed");
    let mut args = HashMap::new();
    args.insert("x", EvalInput::Complex(Complex::new(1.0, 1.0)));
    let result = eval(&compiled, args)
        .expect("eval failed")
        .scalar()
        .expect("scalar failed");
    // (1+i)^2 = 1 + 2i + i^2 = 1 + 2i - 1 = 2i
    println!("(1+i)^2 = {:?}", result);

    // Example 4: ln(-1) = iπ
    println!("\n=== ln(-1) ===");
    let ast = Expression::Function {
        name: "ln".into(),
        args: vec![Expression::Unary {
            op: mathlex::UnaryOp::Neg,
            operand: Box::new(Expression::Integer(1)),
        }],
    };
    let compiled = compile(&ast, &HashMap::new()).expect("compile failed");
    let result = eval(&compiled, HashMap::new())
        .expect("eval failed")
        .scalar()
        .expect("scalar failed");
    println!("ln(-1) = {:?}", result);
    if let NumericResult::Complex(c) = result {
        println!(
            "  (re ≈ 0: {}, im ≈ π: {})",
            c.re.abs() < 1e-10,
            (c.im - std::f64::consts::PI).abs() < 1e-10
        );
    }
}
