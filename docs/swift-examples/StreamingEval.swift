/// Streaming evaluation example.
///
/// Demonstrates lazy consumption via the EvalIterator interface.
///
/// Prerequisites:
///   cargo build --release --features ffi

import MathLexEval

func streamingEvalExample() throws {
    // AST JSON for: x^2 + 1
    let astJson = """
    {"Binary":{"op":"Add","left":{"Binary":{"op":"Pow","left":{"Variable":"x"},"right":{"Integer":2}}},"right":{"Integer":1}}}
    """

    let expr = try MathEvaluator.compile(json: astJson)

    // Create handle with array input
    let handle = try MathEvaluator.createHandle(
        expr,
        args: ["x": .array([1, 2, 3, 4, 5])]
    )

    print("x^2 + 1 for x = 1..5:")

    // Consume lazily via iterator — conforms to Sequence
    for (i, result) in handle.makeIterator().enumerated() {
        let x = i + 1
        switch result {
        case .success(let val):
            print("  f(\(x)) = \(val)")
        case .failure(let err):
            print("  f(\(x)) = ERROR: \(err)")
        }
    }
}
