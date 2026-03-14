fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_dir = std::path::Path::new("proto");
    prost_build::Config::new().compile_protos(
        &[proto_dir.join("pwright/script/v1/script.proto")],
        &[proto_dir],
    )?;
    Ok(())
}
