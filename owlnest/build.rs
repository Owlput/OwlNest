pub fn main()->Result<(),Box<dyn std::error::Error>>{
    tonic_build::compile_protos("src/protos/nest_rpc/nest_rpc.proto")?;
    Ok(())
}