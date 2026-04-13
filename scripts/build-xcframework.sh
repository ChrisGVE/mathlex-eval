#!/bin/bash
set -euo pipefail

PROJECT_DIR=$(pwd)
BUILD_DIR="$PROJECT_DIR/target/xcframework"
FRAMEWORK_NAME="MathLexEval"
SWIFT_TARGET_DIR="$PROJECT_DIR/Sources/MathLexEvalRust"
LIB_NAME="libmathlex_eval"

# Clean
rm -rf "$BUILD_DIR"
mkdir -p "$BUILD_DIR"
mkdir -p "$SWIFT_TARGET_DIR"

echo "Building Rust library for all Apple targets..."

# Build for all targets with FFI and serde
cargo build --release --features "ffi,serde" --target aarch64-apple-ios
cargo build --release --features "ffi,serde" --target aarch64-apple-ios-sim
cargo build --release --features "ffi,serde" --target x86_64-apple-ios
cargo build --release --features "ffi,serde" --target aarch64-apple-darwin
cargo build --release --features "ffi,serde" --target x86_64-apple-darwin

# Copy generated Swift bindings to Swift package target
echo "Copying generated Swift bindings to $SWIFT_TARGET_DIR"
for file in "generated/mathlex-eval/mathlex-eval.swift" "generated/SwiftBridgeCore.swift"; do
	if [ -f "$file" ]; then
		cp "$file" "$SWIFT_TARGET_DIR/"
		echo "  copied $(basename "$file")"
	else
		echo "ERROR: $file not found"
		exit 1
	fi
done

# Copy headers to bridge target
echo "Copying headers to Sources/MathLexEvalBridge/include/"
cp generated/SwiftBridgeCore.h Sources/MathLexEvalBridge/include/
cp generated/mathlex-eval/mathlex-eval.h Sources/MathLexEvalBridge/include/

# Create fat library for iOS Simulator (arm64 + x86_64)
echo "Creating fat library for iOS Simulator..."
lipo -create \
	"target/aarch64-apple-ios-sim/release/${LIB_NAME}.a" \
	"target/x86_64-apple-ios/release/${LIB_NAME}.a" \
	-output "$BUILD_DIR/${LIB_NAME}-ios-sim.a"

# Create fat library for macOS (arm64 + x86_64)
echo "Creating fat library for macOS..."
lipo -create \
	"target/aarch64-apple-darwin/release/${LIB_NAME}.a" \
	"target/x86_64-apple-darwin/release/${LIB_NAME}.a" \
	-output "$BUILD_DIR/${LIB_NAME}-macos.a"

# Create XCFramework
echo "Creating XCFramework..."
xcodebuild -create-xcframework \
	-library "target/aarch64-apple-ios/release/${LIB_NAME}.a" \
	-headers generated/mathlex-eval \
	-library "$BUILD_DIR/${LIB_NAME}-ios-sim.a" \
	-headers generated/mathlex-eval \
	-library "$BUILD_DIR/${LIB_NAME}-macos.a" \
	-headers generated/mathlex-eval \
	-output "$BUILD_DIR/$FRAMEWORK_NAME.xcframework"

echo "XCFramework created at $BUILD_DIR/$FRAMEWORK_NAME.xcframework"
