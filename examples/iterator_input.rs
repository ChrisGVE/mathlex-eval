//! Iterator input example: streaming evaluation with iterator arguments.
//!
//! Demonstrates feeding an iterator as input and consuming results
//! lazily via the EvalIter interface.

use std::collections::HashMap;

use mathlex::{BinaryOp, Expression};

use mathlex_eval::{EvalInput, compile, eval};

fn main() {
    // Build AST for: x^2 + 1
    let ast = Expression::Binary {
        op: BinaryOp::Add,
        left: Box::new(Expression::Binary {
            op: BinaryOp::Pow,
            left: Box::new(Expression::Variable("x".into())),
            right: Box::new(Expression::Integer(2)),
        }),
        right: Box::new(Expression::Integer(1)),
    };

    let compiled = compile(&ast, &HashMap::new()).expect("compilation failed");

    // Feed an iterator as input — values are materialized and cached internally
    let values = (1..=10).map(|i| i as f64);
    let mut args = HashMap::new();
    args.insert("x", EvalInput::Iter(Box::new(values)));

    let handle = eval(&compiled, args).expect("eval failed");
    println!("Shape: {:?}", handle.shape());

    // Consume lazily via iterator
    println!("\nx^2 + 1 for x = 1..10:");
    for (i, result) in handle.iter().enumerate() {
        let x = (i + 1) as f64;
        match result {
            Ok(v) => println!("  f({x}) = {v:?}"),
            Err(e) => println!("  f({x}) = ERROR: {e}"),
        }
    }

    // Demonstrate mixed iterator + scalar broadcasting
    println!("\n--- Mixed iterator + scalar ---");
    let ast2 = Expression::Binary {
        op: BinaryOp::Mul,
        left: Box::new(Expression::Variable("scale".into())),
        right: Box::new(Expression::Variable("x".into())),
    };
    let compiled2 = compile(&ast2, &HashMap::new()).expect("compilation failed");

    let stream = (1..=5).map(|i| i as f64);
    let mut args2 = HashMap::new();
    args2.insert("scale", EvalInput::Scalar(3.0));
    args2.insert("x", EvalInput::Iter(Box::new(stream)));

    let handle2 = eval(&compiled2, args2).expect("eval failed");
    println!("Shape: {:?}", handle2.shape());
    let results: Vec<String> = handle2
        .iter()
        .map(|r| format!("{:?}", r.expect("eval error")))
        .collect();
    println!("3 * [1,2,3,4,5] = [{}]", results.join(", "));
}
