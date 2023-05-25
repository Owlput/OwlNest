#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    OnConnectionEstablished = 0,
    OnIncomingConnection = 1,
    OnIncomingConnectionError = 2,
    OnOutgoingConnectionError = 3,
    OnNewListenAddr = 5,
    OnExpiredListenAddr = 6,
    OnListenerClosed = 7,
    OnListenerError = 8,
    OnDialing = 9,
}
impl std::hash::Hash for Kind{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        format!("swarm:{:?}",self).hash(state);
    }
}