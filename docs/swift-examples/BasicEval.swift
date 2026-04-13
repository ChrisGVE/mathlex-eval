/// Basic evaluation example.
///
/// Demonstrates compiling and evaluating a simple expression.
///
/// Prerequisites:
///   cargo build --release --features ffi

import MathLexEval

func basicEvalExample() throws {
    // AST JSON for: 2*x + 3
    // (In practice, mathlex generates this from parsing "2*x + 3")
    let astJson = """
    {"Binary":{"op":"Add","left":{"Binary":{"op":"Mul","left":{"Integer":2},"right":{"Variable":"x"}}},"right":{"Integer":3}}}
    """

    // Compile with no constants
    let expr = try MathEvaluator.compile(json: astJson)
    print("Arguments: \(expr.argumentNames)")
    print("Complex: \(expr.isComplex)")

    // Evaluate with x = 5.0
    let result = try MathEvaluator.evaluate(expr, args: ["x": .scalar(5.0)])

    switch result {
    case .real(let v):
        print("2*5 + 3 = \(v)")
    case .complex(let re, let im):
        print("2*5 + 3 = \(re) + \(im)i")
    }
}
