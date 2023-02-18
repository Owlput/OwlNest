use super::*;
#[derive(Debug)]
pub enum OpResult {
    SuccessfulPost(Duration),
    Local(LocalOpResult),
    Error(Error),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum LocalOpResult {
    Ok,
    AlreadyTrusted,
    NotFound,
}

