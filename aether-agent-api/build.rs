//! Build script for aether-agent-api.
//! Compiles the Protobuf definitions.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    std::env::set_var("PROTOC", protoc_bin_vendored::protoc_bin_path().unwrap());
    tonic_build::compile_protos("proto/aether.proto")?;
    Ok(())
}
