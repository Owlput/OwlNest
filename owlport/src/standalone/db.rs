use std::collections::HashMap;
use std::sync::RwLock;

use tokio::sync::oneshot;

pub struct SimpleDB<T> {
    name: String,
    locks: Vec<RwLock<HashMap<String, T>>>,
    receiver: oneshot::Receiver<T>,
}

pub enum DBOps<T> {
    GetKV {
        key: String,
        respond_to: oneshot::Sender<T>,
    },
    InsertKV {
        key: String,
        value: T,
    },
}

impl<T> SimpleDB<T> {
    pub fn new(name: String, shard_count: u32,receiver: oneshot::Receiver<T>) -> Result<Self, String> {
        let mut locks = vec![];
        for i in 0..shard_count {
            locks.push(RwLock::new(HashMap::new()))
        }
        Ok(SimpleDB { name, locks,receiver })
    }
    pub fn insert(&self, entry: String) -> Result<(), ()> {
        Err(())
    }
}
