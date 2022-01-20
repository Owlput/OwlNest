pub fn compile_protos()->Result<(),Box<dyn std::error::Error>>{
    tonic_build::compile_protos("/net/grpc/protos/service.proto")?;
    Ok(())
}