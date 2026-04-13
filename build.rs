#[cfg(feature = "ffi")]
fn main() {
    use std::path::PathBuf;

    let out_dir = PathBuf::from("./generated");

    let bridges = vec!["src/ffi/bridge.rs"];
    for path in &bridges {
        println!("cargo:rerun-if-changed={}", path);
    }

    swift_bridge_build::parse_bridges(bridges)
        .write_all_concatenated(out_dir, env!("CARGO_PKG_NAME"));
}

#[cfg(not(feature = "ffi"))]
fn main() {}
