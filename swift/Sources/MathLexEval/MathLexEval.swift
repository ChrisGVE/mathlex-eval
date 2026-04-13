import Foundation

// MARK: - Result types

/// Result of evaluating a compiled mathematical expression.
public enum MathEvalResult: Sendable, Codable {
    case real(Double)
    case complex(re: Double, im: Double)

    /// The real component, or the real part of a complex number.
    public var realPart: Double {
        switch self {
        case .real(let v): return v
        case .complex(let re, _): return re
        }
    }

    /// The imaginary component, or zero for real numbers.
    public var imaginaryPart: Double {
        switch self {
        case .real: return 0
        case .complex(_, let im): return im
        }
    }

    /// Whether this result is complex.
    public var isComplex: Bool {
        switch self {
        case .real: return false
        case .complex: return true
        }
    }

    // Codable conformance for JSON deserialization from Rust
    private enum CodingKeys: String, CodingKey {
        case Real, Complex
    }

    private struct ComplexValue: Codable {
        let re: Double
        let im: Double
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        if let value = try container.decodeIfPresent(Double.self, forKey: .Real) {
            self = .real(value)
        } else if let value = try container.decodeIfPresent(ComplexValue.self, forKey: .Complex) {
            self = .complex(re: value.re, im: value.im)
        } else {
            throw DecodingError.dataCorrupted(
                .init(codingPath: decoder.codingPath,
                      debugDescription: "Expected Real or Complex"))
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case .real(let v):
            try container.encode(v, forKey: .Real)
        case .complex(let re, let im):
            try container.encode(ComplexValue(re: re, im: im), forKey: .Complex)
        }
    }
}

// MARK: - Error

/// Errors from mathlex-eval operations.
public struct MathEvalError: Error, LocalizedError, Sendable {
    public let message: String

    public var errorDescription: String? { message }

    init(_ message: String) {
        self.message = message
    }
}

// MARK: - Compiled expression

/// A compiled mathematical expression ready for repeated evaluation.
///
/// Wraps the Rust `CompiledExpr` via swift-bridge FFI. Create via
/// ``MathEvaluator/compile(json:constants:)``, then evaluate with
/// different argument values.
public final class CompiledExpression: @unchecked Sendable {
    let inner: RustCompiledExpr

    init(_ inner: RustCompiledExpr) {
        self.inner = inner
    }

    /// Names of free variables that must be provided at evaluation time.
    public var argumentNames: [String] {
        let rustVec = ffi_argument_names(inner)
        var result: [String] = []
        for i in 0..<rustVec.len() {
            if let s = rustVec.get(index: i) {
                result.append(String(s))
            }
        }
        return result
    }

    /// Whether the expression involves complex numbers.
    public var isComplex: Bool {
        ffi_is_complex(inner)
    }
}

// MARK: - Eval handle

/// Lazy evaluation handle. Defers computation until consumed.
///
/// Consume via ``scalar()``, ``toArray()``, or by iterating with
/// ``makeIterator()``.
public final class EvalHandle: @unchecked Sendable {
    let inner: RustEvalHandle

    init(_ inner: RustEvalHandle) {
        self.inner = inner
    }

    /// Output shape. Empty for scalar, `[n]` for 1-D, `[n, m]` for 2-D.
    public var shape: [Int] {
        let rustVec = ffi_shape(inner)
        var result: [Int] = []
        for i in 0..<rustVec.len() {
            if let v = rustVec.get(index: i) {
                result.append(Int(v))
            }
        }
        return result
    }

    /// Total number of output elements.
    public var count: Int {
        Int(ffi_len(inner))
    }

    /// Whether the output is empty.
    public var isEmpty: Bool {
        count == 0
    }

    /// Consume as scalar result. Throws if output is not 0-d.
    public func scalar() throws -> MathEvalResult {
        let json = try ffi_scalar_json(inner)
        return try JSONDecoder().decode(MathEvalResult.self, from: Data(String(json).utf8))
    }

    /// Consume eagerly into array of results in row-major order.
    public func toArray() throws -> [MathEvalResult] {
        let json = try ffi_to_array_json(inner)
        return try JSONDecoder().decode([MathEvalResult].self, from: Data(String(json).utf8))
    }

    /// Consume lazily as a streaming iterator.
    public func makeIterator() -> EvalIterator {
        EvalIterator(ffi_into_iter(inner))
    }
}

// MARK: - Streaming iterator

/// Streaming iterator over broadcast evaluation results.
///
/// Yields one ``MathEvalResult`` per output element. Conforms to
/// `Sequence` and `IteratorProtocol` for use in `for-in` loops.
public final class EvalIterator: Sequence, IteratorProtocol, @unchecked Sendable {
    public typealias Element = Result<MathEvalResult, MathEvalError>

    private let inner: RustEvalIter

    init(_ inner: RustEvalIter) {
        self.inner = inner
    }

    public func next() -> Element? {
        guard let json = ffi_iter_next(inner) else {
            return nil
        }
        let str = String(json)
        // Error results come as {"error":"..."} from Rust
        if str.hasPrefix("{\"error\":") {
            let msg = str
                .dropFirst(10) // {"error":"
                .dropLast(2)   // "}
            return .failure(MathEvalError(String(msg)))
        }
        do {
            let result = try JSONDecoder().decode(
                MathEvalResult.self, from: Data(str.utf8))
            return .success(result)
        } catch {
            return .failure(MathEvalError(error.localizedDescription))
        }
    }
}

// MARK: - Main API

/// Entry point for compiling and evaluating mathlex ASTs.
///
/// Two-phase usage:
/// 1. **Compile** an AST (as JSON) with optional constants
/// 2. **Evaluate** the compiled expression with arguments
///
/// ```swift
/// let expr = try MathEvaluator.compile(json: astJson)
/// let result = try MathEvaluator.evaluate(expr, args: ["x": .scalar(5.0)])
/// ```
public enum MathEvaluator {

    /// Argument value for evaluation.
    public enum Argument: Sendable {
        /// Single scalar value — broadcasts to all positions.
        case scalar(Double)
        /// Array of values — contributes one axis to output shape.
        case array([Double])
    }

    /// Compile a mathlex AST from JSON with optional constants.
    ///
    /// - Parameters:
    ///   - json: JSON string representing a mathlex `Expression` AST
    ///   - constants: Map of constant names to numeric values
    /// - Returns: A compiled expression ready for evaluation
    /// - Throws: ``MathEvalError`` on unsupported expressions, unknown functions, etc.
    public static func compile(
        json: String,
        constants: [String: Double] = [:]
    ) throws -> CompiledExpression {
        let constantsJson: String
        if constants.isEmpty {
            constantsJson = "{}"
        } else {
            // Encode constants as {"name": {"Real": value}}
            var dict: [String: [String: Double]] = [:]
            for (key, value) in constants {
                dict[key] = ["Real": value]
            }
            let data = try JSONEncoder().encode(dict)
            constantsJson = String(data: data, encoding: .utf8) ?? "{}"
        }

        do {
            let inner = try ffi_compile(json, constantsJson)
            return CompiledExpression(inner)
        } catch let error as RustString {
            throw MathEvalError(String(error))
        }
    }

    /// Evaluate a compiled expression with the given arguments as a scalar.
    ///
    /// - Parameters:
    ///   - expr: The compiled expression
    ///   - args: Map of argument names to values
    /// - Returns: The scalar evaluation result
    /// - Throws: ``MathEvalError`` on missing arguments, division by zero, or non-scalar output
    public static func evaluate(
        _ expr: CompiledExpression,
        args: [String: Argument] = [:]
    ) throws -> MathEvalResult {
        let handle = try createHandle(expr, args: args)
        return try handle.scalar()
    }

    /// Evaluate a compiled expression over arrays, producing results in row-major order.
    ///
    /// - Parameters:
    ///   - expr: The compiled expression
    ///   - args: Map of argument names to values (scalars or arrays)
    /// - Returns: Array of results with shape determined by Cartesian product of array args
    /// - Throws: ``MathEvalError`` on missing arguments, division by zero, or shape issues
    public static func evaluateArray(
        _ expr: CompiledExpression,
        args: [String: Argument] = [:]
    ) throws -> [MathEvalResult] {
        let handle = try createHandle(expr, args: args)
        return try handle.toArray()
    }

    /// Create a lazy evaluation handle for advanced consumption patterns.
    ///
    /// - Parameters:
    ///   - expr: The compiled expression
    ///   - args: Map of argument names to values
    /// - Returns: An ``EvalHandle`` that can be consumed via `scalar()`, `toArray()`, or `makeIterator()`
    public static func createHandle(
        _ expr: CompiledExpression,
        args: [String: Argument] = [:]
    ) throws -> EvalHandle {
        let argsJson = try encodeArgs(args)

        do {
            let inner = try ffi_eval_json(expr.inner, argsJson)
            return EvalHandle(inner)
        } catch let error as RustString {
            throw MathEvalError(String(error))
        }
    }

    // MARK: - Private

    private static func encodeArgs(_ args: [String: Argument]) throws -> String {
        if args.isEmpty { return "{}" }

        var dict: [String: Any] = [:]
        for (key, value) in args {
            switch value {
            case .scalar(let v):
                dict[key] = v
            case .array(let arr):
                dict[key] = arr
            }
        }
        let data = try JSONSerialization.data(withJSONObject: dict)
        return String(data: data, encoding: .utf8) ?? "{}"
    }
}
