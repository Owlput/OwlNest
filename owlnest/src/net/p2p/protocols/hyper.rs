
pub struct Handle {
    sender: mpsc::Sender<InEvent>,
    swarm_event_source: EventSender,
    counter: Arc<AtomicU64>,
}
#[allow(unused)]
impl Handle {
    pub fn new(buffer: usize, swarm_event_source: &EventSender) -> (Self, mpsc::Receiver<InEvent>) {
        let (tx, rx) = mpsc::channel(buffer);
        (
            Self {
                sender: tx,
                swarm_event_source: swarm_event_source.clone(),
                counter: Arc::new(AtomicU64::new(0)),
            },
            rx,
        )
    }
    generate_handler_method!(SendRequest:send_request(peer:PeerId,request:Request<String>)->Response<Bytes>;);
    fn next_id(&self) -> u64 {
        self.counter.fetch_add(1, Ordering::SeqCst)
    }
}