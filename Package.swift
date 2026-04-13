// swift-tools-version: 5.9

import PackageDescription

let package = Package(
    name: "MathLexEval",
    platforms: [
        .iOS(.v16),
        .macOS(.v13),
        .watchOS(.v9),
        .tvOS(.v16),
        .visionOS(.v1),
    ],
    products: [
        .library(
            name: "MathLexEval",
            targets: ["MathLexEval"]
        ),
    ],
    targets: [
        // C headers for the swift-bridge FFI symbols
        .target(
            name: "MathLexEvalBridge",
            dependencies: [],
            path: "Sources/MathLexEvalBridge",
            publicHeadersPath: "include"
        ),

        // Generated swift-bridge Swift bindings + Rust static library linkage
        .target(
            name: "MathLexEvalRust",
            dependencies: ["MathLexEvalBridge"],
            path: "Sources/MathLexEvalRust",
            linkerSettings: [
                .unsafeFlags([
                    "-L", "target/release",
                    "-lmathlex_eval",
                ]),
            ]
        ),

        // Swift wrapper providing idiomatic Swift API
        .target(
            name: "MathLexEval",
            dependencies: ["MathLexEvalRust"],
            path: "Sources/MathLexEval"
        ),
    ]
)
