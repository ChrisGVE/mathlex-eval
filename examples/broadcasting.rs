//! Broadcasting example: evaluate `x^2 + y` over arrays of x and y values.
//!
//! Demonstrates Cartesian product broadcasting — each array argument
//! contributes one axis to the output shape.

use std::collections::HashMap;

use mathlex::{BinaryOp, Expression};

use mathlex_eval::{EvalInput, NumericResult, compile, eval};

fn main() {
    // Build AST for: x^2 + y
    let x_squared = Expression::Binary {
        op: BinaryOp::Pow,
        left: Box::new(Expression::Variable("x".into())),
        right: Box::new(Expression::Integer(2)),
    };
    let ast = Expression::Binary {
        op: BinaryOp::Add,
        left: Box::new(x_squared),
        right: Box::new(Expression::Variable("y".into())),
    };

    let compiled = compile(&ast, &HashMap::new()).expect("compilation failed");

    // Evaluate over x = [1, 2, 3] and y = [10, 20]
    // Output shape: [3, 2] (Cartesian product)
    let mut args = HashMap::new();
    args.insert("x", EvalInput::from(vec![1.0, 2.0, 3.0]));
    args.insert("y", EvalInput::from(vec![10.0, 20.0]));

    let handle = eval(&compiled, args).expect("eval failed");
    println!("Output shape: {:?}", handle.shape());
    println!("Total elements: {}", handle.len());

    // Consume as array
    let array = handle.to_array().expect("array eval failed");
    println!("\n         y=10    y=20");
    for (xi, x_val) in [1.0, 2.0, 3.0].iter().enumerate() {
        let v0 = array.get([xi, 0]).unwrap();
        let v1 = array.get([xi, 1]).unwrap();
        println!("x={x_val}  [ {:>5}, {:>5} ]", fmt(v0), fmt(v1));
    }

    // Also demonstrate scalar + array broadcasting
    let mut args2 = HashMap::new();
    args2.insert("x", EvalInput::Scalar(2.0)); // scalar broadcasts
    args2.insert("y", EvalInput::from(vec![10.0, 20.0, 30.0]));

    let handle2 = eval(&compiled, args2).expect("eval failed");
    println!("\nScalar x=2, y=[10,20,30]:");
    println!("Shape: {:?}", handle2.shape());
    let results: Vec<String> = handle2
        .iter()
        .map(|r| fmt(&r.expect("eval error")))
        .collect();
    println!("Results: [{}]", results.join(", "));
}

fn fmt(r: &NumericResult) -> String {
    match r {
        NumericResult::Real(v) => format!("{v}"),
        NumericResult::Complex(c) => format!("{c}"),
    }
}
