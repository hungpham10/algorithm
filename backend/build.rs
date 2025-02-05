use std::io::Result;

fn main() -> Result<()> {
    prost_build::compile_protos(
        &["proto/internal/simulator/v1/simulator.proto"], 
        &["proto/internal"],
    )
    .unwrap();


    println!("cargo:rerun-if-changed=build.rs");
    Ok(())
}
