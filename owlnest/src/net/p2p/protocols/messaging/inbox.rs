use std::collections::{VecDeque,HashMap};

use super::Message;

pub(crate) struct Inbox {
    queue: VecDeque<u128>,
    map: HashMap<u128, Message>,
}

pub(crate) enum InboxActionError {
    EmptyQueue,
    NotFound(u128),
}

impl Inbox {
    pub fn new()->Self{
        Self { queue: VecDeque::new(), map: HashMap::new() }
    }
    pub fn push(&mut self, message: Message) {
        self.queue.push_front(message.time);
        self.map.insert(message.time, message);
    }
    /// Borrows the next message for serialization.
    /// The message is removed from the queue but still available in the map.
    pub fn take_next(&mut self) -> Result<&Message, InboxActionError> {
        let key = match self.queue.pop_back() {
            Some(v) => v,
            None => return Err(InboxActionError::EmptyQueue),
        };
        match self.map.get(&key) {
            Some(v) => Ok(v),
            None => Err(InboxActionError::NotFound(key)),
        }
    }
    /// Get the message with the given stamp from map
    #[inline]
    pub fn get(&self,k: &u128)->Option<&Message>{
        self.map.get(k)
    }
    /// This function won't check whether the message has been processed
    /// and simply removes the message from the map.
    /// It runs faster but will cause a dropped message when not used properly.
    #[inline]
    pub fn delete_unchecked(&mut self, index: u128) -> Option<Message> {
        self.map.remove(&index)
    }
    /// This function makes sure that the message is deleted both from the queue and the map.
    pub fn delete_forced(&mut self, index: u128) -> Option<Message> {
        let queue_index = match self.queue.binary_search(&index) {
            Ok(v) => v,
            Err(_) => match self.map.remove(&index) {
                Some(v) => return Some(v),
                None => return None,
            },
        };
        self.queue.remove(queue_index);
        self.map.remove(&index)
    }
}
