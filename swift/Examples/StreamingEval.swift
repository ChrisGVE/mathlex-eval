/// Streaming evaluation example.
///
/// Demonstrates iterator input and lazy consumption via for-in.
///
/// Note: Requires swift-bridge FFI integration to run.

import MathLexEval

// In full integration, iterator input from Swift crosses FFI via callbacks:
//
// var index = 0
// let values = [1.0, 2.0, 3.0, 4.0, 5.0]
// let iter = MathEvaluator.makeIteratorInput {
//     guard index < values.count else { return nil }
//     defer { index += 1 }
//     return values[index]
// }
//
// The Rust side wraps the callback in SwiftIterAdapter and caches
// values incrementally during evaluation.
