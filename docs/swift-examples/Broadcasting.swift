/// Broadcasting example.
///
/// Demonstrates array evaluation with Cartesian product broadcasting.
///
/// Prerequisites:
///   cargo build --release --features ffi

import MathLexEval

func broadcastingExample() throws {
    // AST JSON for: x^2 + y
    let astJson = """
    {"Binary":{"op":"Add","left":{"Binary":{"op":"Pow","left":{"Variable":"x"},"right":{"Integer":2}}},"right":{"Variable":"y"}}}
    """

    let expr = try MathEvaluator.compile(json: astJson)

    // Evaluate over x = [1, 2, 3] and y = [10, 20]
    // Output shape: [3, 2] (Cartesian product)
    let handle = try MathEvaluator.createHandle(
        expr,
        args: [
            "x": .array([1, 2, 3]),
            "y": .array([10, 20]),
        ]
    )

    print("Shape: \(handle.shape)")
    print("Count: \(handle.count)")

    // Consume as array
    let results = try handle.toArray()
    print("Results: \(results)")

    // Scalar + array broadcasting
    let handle2 = try MathEvaluator.createHandle(
        expr,
        args: [
            "x": .scalar(2),
            "y": .array([10, 20, 30]),
        ]
    )
    print("\nScalar x=2, y=[10,20,30]:")
    print("Shape: \(handle2.shape)")
    let results2 = try handle2.toArray()
    print("Results: \(results2)")
}
