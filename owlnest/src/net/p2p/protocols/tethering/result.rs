use super::subprotocols::{exec, push};

#[derive(Debug)]
pub enum OpResult{
    Exec(exec::result::OpResult),
    Push(push::result::OpResult)
}