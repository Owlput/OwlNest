use super::*;
#[derive(Debug)]
pub enum CallbackResult{
    SuccessfulPost(Duration),
    Error(Error)
}