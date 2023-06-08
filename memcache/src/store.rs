use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use tokio::sync::mpsc;

use crate::manager::*;

/// A wrapper around ``std::collections::HashMap<K,V>`` for asynchronous operations.
pub struct MapStore<V> {
    map: HashMap<String, V>,
    pub receiver: mpsc::Receiver<MapOps<V>>,
}

impl<V> MapStore<V>
where
    V: Hash + Clone + Send,
{
    /// Factory function for creating a new instance of ``MapStore``
    pub fn new(receiver: mpsc::Receiver<MapOps<V>>) -> Self {
        Self {
            map: HashMap::new(),
            receiver,
        }
    }
    /// Creates a ``MapStore`` with the given ``HashMap<String,V>``.
    pub fn with_map(map: HashMap<String, V>, receiver: mpsc::Receiver<MapOps<V>>) -> Self {
        Self { map, receiver }
    }
    pub fn handle_op(&mut self, op: MapOps<V>) {
        let _ = match op {
            MapOps::Set(k, v, tx) => match self.map.insert(k, v) {
                Some(v) => tx.send(MapResult::Modified(v)),
                None => tx.send(MapResult::Inserted),
            },
            MapOps::TryInsert(k, v, tx) => match self.map.get(&k) {
                Some(_) => tx.send(MapResult::Failed),
                None => {
                    self.map.insert(k, v);
                    tx.send(MapResult::Inserted)
                }
            },
            MapOps::Delete(k, tx) => match self.map.remove(&k) {
                Some(v) => tx.send(MapResult::Deleted(v)),
                None => tx.send(MapResult::Failed),
            },
            MapOps::GetSingle(k, tx) => match self.map.get(&k) {
                Some(v) => tx.send(MapResult::FoundValue(Some(v.clone()))),
                None => tx.send(MapResult::FoundValue(None)),
            },
            MapOps::Lookup(pattern, tx) => {
                let mut map = self.map.clone();
                map.retain(|k, _| pattern.is_match(k));
                if map.is_empty() {
                    tx.send(MapResult::FoundKV(None))
                } else {
                    tx.send(MapResult::FoundKV(Some(map)))
                }
            }
            MapOps::GetAll(tx) => tx.send(MapResult::FoundKV(Some(self.map.clone()))),
        };
    }
}

/// A wrapper around ``std::collections::HashSet<T>`` for asynchronous operations.
pub struct SetStore<T> {
    set: HashSet<T>,
    pub receiver: mpsc::Receiver<SetOps<T>>,
}

impl<T> SetStore<T>
where
    T: Hash + Clone + Send + Eq,
{
    /// Factory function for creating a new instance of ``SetStore``.
    pub fn new(receiver: mpsc::Receiver<SetOps<T>>) -> Self {
        Self {
            set: HashSet::new(),
            receiver,
        }
    }
    /// Creates a ``SetStore`` with the given ``HashSet<T>``.
    pub fn with_set(set: HashSet<T>, receiver: mpsc::Receiver<SetOps<T>>) -> Self {
        Self { set, receiver }
    }
    pub fn handle_ops(&mut self, op: SetOps<T>) {
        let _ = match op {
            SetOps::Find(v, tx) => tx.send(if self.set.contains(&v) {
                SetResult::Found
            } else {
                SetResult::NotFound
            }),
            SetOps::Dump(tx) => tx.send(SetResult::Dumped(
                self.set.clone().into_iter().collect::<Vec<T>>(),
            )),
            SetOps::TryInsert(v, tx) => match self.set.insert(v) {
                false => tx.send(SetResult::Failed),
                true => tx.send(SetResult::Inserted),
            },
            SetOps::Delete(v, tx) => match self.set.remove(&v) {
                true => tx.send(SetResult::Deleted),
                false => tx.send(SetResult::Failed),
            },
        };
    }
}
