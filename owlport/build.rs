pub fn main()->Result<(),Box<dyn std::error::Error>>{
    tonic_build::compile_protos("./src/net/grpc/protos/helloworld.proto")?;
    Ok(())
}