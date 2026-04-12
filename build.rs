#[cfg(feature = "ffi")]
fn main() {
    swift_bridge_build::parse_bridges(vec!["src/ffi/bridge.rs"]);
}

#[cfg(not(feature = "ffi"))]
fn main() {}
