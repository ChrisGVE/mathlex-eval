//! Basic example: compile and evaluate `2*x + 3` with a single value.

use std::collections::HashMap;

use mathlex::{BinaryOp, Expression};

use mathlex_eval::{EvalInput, NumericResult, compile, eval};

fn main() {
    // Build AST for: 2*x + 3
    let ast = Expression::Binary {
        op: BinaryOp::Add,
        left: Box::new(Expression::Binary {
            op: BinaryOp::Mul,
            left: Box::new(Expression::Integer(2)),
            right: Box::new(Expression::Variable("x".into())),
        }),
        right: Box::new(Expression::Integer(3)),
    };

    // Compile with no constants
    let compiled = compile(&ast, &HashMap::new()).expect("compilation failed");
    println!("Arguments: {:?}", compiled.argument_names());
    println!("Complex: {}", compiled.is_complex());

    // Evaluate with x = 5.0
    let mut args = HashMap::new();
    args.insert("x", EvalInput::Scalar(5.0));
    let handle = eval(&compiled, args).expect("eval failed");
    let result = handle.scalar().expect("scalar eval failed");

    match result {
        NumericResult::Real(v) => println!("2*5 + 3 = {v}"),
        NumericResult::Complex(c) => println!("2*5 + 3 = {c}"),
    }
}
