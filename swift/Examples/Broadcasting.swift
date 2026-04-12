/// Broadcasting example.
///
/// Demonstrates array evaluation with Cartesian product broadcasting.
///
/// Note: Requires swift-bridge FFI integration to run.

import MathLexEval

// Compile x^2 + y
// let expr = try MathEvaluator.compile(json: astJson)
//
// Evaluate over x = [1, 2, 3] and y = [10, 20]
// let results = try MathEvaluator.evaluateArray(
//     expr,
//     args: [
//         "x": .array([1, 2, 3]),
//         "y": .array([10, 20])
//     ]
// )
// Output shape: [3, 2]
// results = [11, 21, 14, 24, 19, 29]
