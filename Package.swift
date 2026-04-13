// swift-tools-version:5.9

import PackageDescription

// NOTE: Before building, run:
//   cargo build --release --features ffi
// This generates the Swift bridge files in ./generated and the static
// library in ./target/release/libmathlex_eval.a

let package = Package(
    name: "MathLexEval",
    platforms: [
        .macOS(.v13),
        .iOS(.v16),
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
        // Swift convenience wrapper
        .target(
            name: "MathLexEval",
            dependencies: ["MathLexEvalBridge"],
            path: "swift/Sources/MathLexEval"
        ),
        // Generated swift-bridge code + Rust static library linkage
        .target(
            name: "MathLexEvalBridge",
            path: "generated",
            publicHeadersPath: ".",
            cSettings: [
                .headerSearchPath("."),
                .headerSearchPath("mathlex-eval"),
            ],
            linkerSettings: [
                .unsafeFlags([
                    "-L", "target/release",
                    "-lmathlex_eval",
                ]),
            ]
        ),
    ]
)
