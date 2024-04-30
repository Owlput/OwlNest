use std::io::Result;
fn main() -> Result<()> {
    prost_build::compile_protos(&["src/protocol/messages.proto"], &["src/protocol"])?;
    Ok(())
}
