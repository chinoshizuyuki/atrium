// SPDX-License-Identifier: MIT
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);

    // 从 crates/atrium-bridge/ 往上级走两级到 workspace root
    let workspace_root = manifest_dir
        .parent()
        .expect("manifest parent")
        .parent()
        .expect("workspace root");
    let proto_dir = workspace_root.join("proto");
    let proto_file = proto_dir.join("atrium.proto");

    println!("cargo:rerun-if-changed={}", proto_file.display());
    println!("cargo:rerun-if-changed={}", proto_dir.display());

    tonic_build::compile_protos(&proto_file)?;
    Ok(())
}
