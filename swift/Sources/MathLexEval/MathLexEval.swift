import Foundation

/// Result of evaluating a compiled mathematical expression.
public enum MathEvalResult: Sendable {
    case real(Double)
    case complex(re: Double, im: Double)

    /// The real component, or the real part of a complex number.
    public var realPart: Double {
        switch self {
        case .real(let v): return v
        case .complex(let re, _): return re
        }
    }

    /// Whether this result is complex.
    public var isComplex: Bool {
        switch self {
        case .real: return false
        case .complex: return true
        }
    }
}

/// A compiled mathematical expression ready for repeated evaluation.
///
/// Create via `MathEvaluator.compile(json:constants:)`, then evaluate
/// with different argument values using `evaluate(args:)`.
public final class CompiledExpression: @unchecked Sendable {
    // Placeholder for swift-bridge opaque pointer
    // In full integration, this wraps the Rust CompiledExpr via FFI

    /// Names of free variables that must be provided at evaluation time.
    public let argumentNames: [String]

    /// Whether the expression involves complex numbers.
    public let isComplex: Bool

    init(argumentNames: [String], isComplex: Bool) {
        self.argumentNames = argumentNames
        self.isComplex = isComplex
    }
}

/// Entry point for compiling and evaluating mathlex ASTs.
///
/// Usage:
/// ```swift
/// let expr = try MathEvaluator.compile(
///     json: astJsonString,
///     constants: ["pi": 3.14159]
/// )
/// let result = try MathEvaluator.evaluate(
///     expr,
///     args: ["x": .scalar(5.0)]
/// )
/// ```
public enum MathEvaluator {

    /// Argument value for evaluation.
    public enum Argument: Sendable {
        case scalar(Double)
        case array([Double])
    }

    /// Compile a mathlex AST from JSON with optional constants.
    ///
    /// - Parameters:
    ///   - json: JSON string representing a mathlex Expression AST
    ///   - constants: Map of constant names to values
    /// - Returns: A compiled expression ready for evaluation
    /// - Throws: If the AST contains unsupported expressions or invalid functions
    public static func compile(
        json: String,
        constants: [String: Double] = [:]
    ) throws -> CompiledExpression {
        // Placeholder: in full swift-bridge integration, this calls
        // compile_from_json() via FFI
        fatalError("swift-bridge FFI integration required")
    }

    /// Evaluate a compiled expression with the given arguments.
    ///
    /// - Parameters:
    ///   - expr: The compiled expression
    ///   - args: Map of argument names to values
    /// - Returns: The evaluation result
    /// - Throws: On missing arguments, division by zero, or shape mismatch
    public static func evaluate(
        _ expr: CompiledExpression,
        args: [String: Argument]
    ) throws -> MathEvalResult {
        // Placeholder: in full swift-bridge integration, this calls
        // eval_with_json() via FFI
        fatalError("swift-bridge FFI integration required")
    }

    /// Evaluate a compiled expression over arrays, producing a grid result.
    ///
    /// - Parameters:
    ///   - expr: The compiled expression
    ///   - args: Map of argument names to values (scalars or arrays)
    /// - Returns: Array of results in row-major order
    public static func evaluateArray(
        _ expr: CompiledExpression,
        args: [String: Argument]
    ) throws -> [MathEvalResult] {
        // Placeholder: in full swift-bridge integration, this calls
        // eval_with_json() + to_array_json() via FFI
        fatalError("swift-bridge FFI integration required")
    }
}
