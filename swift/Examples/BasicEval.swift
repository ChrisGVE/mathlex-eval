/// Basic evaluation example.
///
/// Demonstrates compiling and evaluating a simple expression.
///
/// Note: Requires swift-bridge FFI integration to run.
/// This file serves as API documentation for the Swift wrapper.

import MathLexEval

// Compile an AST (provided as JSON from mathlex)
// let expr = try MathEvaluator.compile(json: astJson)
//
// Evaluate with x = 5.0
// let result = try MathEvaluator.evaluate(expr, args: ["x": .scalar(5.0)])
//
// switch result {
// case .real(let v):
//     print("Result: \(v)")
// case .complex(let re, let im):
//     print("Result: \(re) + \(im)i")
// }
