use std::collections::HashMap;
use std::hash::Hash;

use tokio::sync::{mpsc, oneshot};

use crate::store::MapStore;

/// All supported operations for ``MapStore``.
pub enum MapOps<V> {
    GetSingle(String, oneshot::Sender<MapResult<V>>),
    GetAll(oneshot::Sender<MapResult<V>>),
    Lookup(regex::Regex, oneshot::Sender<MapResult<V>>),
    Set(String, V, oneshot::Sender<MapResult<V>>),
    TryInsert(String, V, oneshot::Sender<MapResult<V>>),
    Delete(String, oneshot::Sender<MapResult<V>>),
}
#[derive(Debug, PartialEq, Eq, Clone)]
/// Execution result of a ``MapOps``
pub enum MapResult<V> {
    Inserted,
    FoundValue(Option<V>),
    FoundKV(Option<HashMap<String, V>>),
    Modified(V),
    Deleted(V),
    Failed,
}

#[derive(Clone)]
/// A cloneable handle for a ``MapStore``. All clones share the same underlying ``MapStore``.
pub struct MapManager<V> {
    sender: mpsc::Sender<MapOps<V>>,
}
impl<V> MapManager<V>
where
    V: Hash + Clone + Send + 'static,
{
    /// Create a new ``MapManager`` and spawn a task for managing a ``MapStore``.
    pub fn new(buffer: usize) -> Self {
        let (sender, receiver) = mpsc::channel(buffer);
        let mut store = MapStore::new(receiver);
        tokio::spawn(async move {
            while let Some(op) = store.receiver.recv().await {
                store.handle_op(op);
            }
        });
        Self { sender }
    }
    pub async fn map_get_single(&self, key: String) -> MapResult<V> {
        let (tx, rx) = oneshot::channel();
        let op = MapOps::GetSingle(key, tx);
        let _ = self.sender.send(op).await;
        rx.await.expect("Actor has been killed")
    }
    pub async fn map_get_all(&self) -> MapResult<V> {
        let (tx, rx) = oneshot::channel();
        let op = MapOps::GetAll(tx);
        let _ = self.sender.send(op).await;
        rx.await.expect("Actor has been killed")
    }
    pub async fn map_set(&self, key: String, value: V) -> MapResult<V> {
        let (tx, rx) = oneshot::channel();
        let op = MapOps::Set(key, value, tx);
        let _ = self.sender.send(op).await;
        rx.await.expect("Actor has been killed")
    }
    pub async fn map_try_insert(&self, key: String, value: V) -> MapResult<V> {
        let (tx, rx) = oneshot::channel();
        let op = MapOps::TryInsert(key, value, tx);
        let _ = self.sender.send(op).await;
        rx.await.expect("Actor has been killed")
    }
    pub async fn map_delete(&self, key: String) -> MapResult<V> {
        let (tx, rx) = oneshot::channel();
        let op = MapOps::Delete(key, tx);
        let _ = self.sender.send(op).await;
        rx.await.expect("Actor has been killed")
    }
    pub async fn map_lookup(&self, pattern: regex::Regex) -> MapResult<V> {
        let (tx, rx) = oneshot::channel();
        let op = MapOps::Lookup(pattern, tx);
        let _ = self.sender.send(op).await;
        rx.await.expect("Actor has been killed")
    }
}
impl<V> From<HashMap<String, V>> for MapManager<V>
where
    V: Hash + Send + Clone + 'static,
{
    fn from(map: HashMap<String, V>) -> Self {
        let (sender, receiver) = mpsc::channel(8);
        let mut store = MapStore::with_map(map, receiver);
        tokio::spawn(async move {
            while let Some(op) = store.receiver.recv().await {
                store.handle_op(op);
            }
        });
        Self { sender }
    }
}

pub enum SetOps<T> {
    Find(T, oneshot::Sender<SetResult<T>>),
    TryInsert(T, oneshot::Sender<SetResult<T>>),
    Delete(T, oneshot::Sender<SetResult<T>>),
    Dump(oneshot::Sender<SetResult<T>>),
}

pub enum SetResult<T> {
    Inserted,
    Failed,
    Deleted,
    Found,
    NotFound,
    Dumped(Vec<T>),
}

#[derive(Clone)]
pub struct SetManager<T> {
    sender: mpsc::Sender<SetOps<T>>,
}

impl<T> SetManager<T>
where
    T: Eq + Hash + Clone + Send,
{
    pub fn new(sender: mpsc::Sender<SetOps<T>>) -> Self {
        Self { sender }
    }
    pub async fn find(&self, v: T) -> SetResult<T> {
        let (tx, rx) = oneshot::channel();
        let op = SetOps::Find(v, tx);
        let _ = self.sender.send(op).await;
        rx.await.expect("Actor has been killed")
    }
    pub async fn try_insert(&self, v: T) -> SetResult<T> {
        let (tx, rx) = oneshot::channel();
        let op = SetOps::TryInsert(v, tx);
        let _ = self.sender.send(op).await;
        rx.await.expect("Actor has been killed")
    }
    pub async fn dump(&self) -> SetResult<T> {
        let (tx, rx) = oneshot::channel();
        let op = SetOps::Dump(tx);
        let _ = self.sender.send(op).await;
        rx.await.expect("Actor has been killed")
    }
    pub async fn delete(&self, v: T) -> SetResult<T> {
        let (tx, rx) = oneshot::channel();
        let op = SetOps::Delete(v, tx);
        let _ = self.sender.send(op).await;
        rx.await.expect("Actor has been killed")
    }
}
