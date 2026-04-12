// swift-tools-version:5.9

import PackageDescription

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
        .target(
            name: "MathLexEval",
            path: "swift/Sources/MathLexEval"
        ),
    ]
)
