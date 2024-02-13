use super::{protocol, Config, Error, PROTOCOL_NAME};
use crate::net::p2p::handler_prelude::*;
use tokio::sync::oneshot;
use futures::Future;
use hyper::body::Incoming;
use hyper::client::conn::http1::SendRequest;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response};
use std::collections::VecDeque;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_util::bytes::Bytes;
use tokio_util::compat::FuturesAsyncReadCompatExt;
use tracing::{debug, info, trace, warn};

#[derive(Debug)]
pub enum FromBehaviourEvent {
    Request {
        request: Request<String>,
        callback: oneshot::Sender<Response<Bytes>>,
    },
}
#[derive(Debug)]
pub enum ToBehaviourEvent {
    Error(Error),
    InboundNegotiated,
    OutboundNegotiated,
    Unsupported,
}

enum State {
    Inactive { reported: bool },
    Active,
}

pub struct Handler {
    state: State,
    pending_in_events: VecDeque<FromBehaviourEvent>,
    pending_out_events: VecDeque<ToBehaviourEvent>,
    inbound_stream: Option<StreamState>,
    outbound_stream: Option<StreamState>,
    server_event_tap: Option<mpsc::Receiver<String>>,
    client_event_tap: Option<mpsc::Receiver<String>>,
    client: Option<ClientState>,
    ongoing_request: Option<(
        std::pin::Pin<Box<dyn Future<Output = Result<Response<Incoming>, hyper::Error>> + Send>>,
        oneshot::Sender<Response<Bytes>>,
    )>,
    pending_requests: VecDeque<(Request<String>, oneshot::Sender<Response<Bytes>>)>,
}

use libp2p::swarm::{handler::DialUpgradeError, StreamUpgradeError};
impl Handler {
    pub fn new(_config: Config) -> Self {
        Self {
            state: State::Active,
            pending_in_events: VecDeque::new(),
            pending_out_events: VecDeque::new(),
            inbound_stream: Default::default(),
            outbound_stream: Default::default(),
            pending_requests: Default::default(),
            server_event_tap: None,
            client_event_tap: None,
            client: None,
            ongoing_request: None,
        }
    }
    #[inline]
    fn on_dial_upgrade_error(
        &mut self,
        DialUpgradeError { error, .. }: DialUpgradeError<
            <Self as ConnectionHandler>::OutboundOpenInfo,
            <Self as ConnectionHandler>::OutboundProtocol,
        >,
    ) {
        self.inbound_stream = None;
        match error {
            StreamUpgradeError::NegotiationFailed => {
                self.state = State::Inactive { reported: false };
            }
            e => {
                let e = format!("{:?}", e);
                if !e.contains("Timeout") {
                    debug!(
                        "Error occurred when negotiating protocol {}: {:?}",
                        PROTOCOL_NAME, e
                    )
                }
            }
        }
    }
}

use libp2p::core::upgrade::ReadyUpgrade;
use libp2p::swarm::{ConnectionHandlerEvent, SubstreamProtocol};
impl ConnectionHandler for Handler {
    type FromBehaviour = FromBehaviourEvent;
    type ToBehaviour = ToBehaviourEvent;
    type InboundProtocol = ReadyUpgrade<&'static str>;
    type OutboundProtocol = ReadyUpgrade<&'static str>;
    type InboundOpenInfo = ();
    type OutboundOpenInfo = ();
    fn listen_protocol(
        &self,
    ) -> libp2p::swarm::SubstreamProtocol<Self::InboundProtocol, Self::InboundOpenInfo> {
        SubstreamProtocol::new(ReadyUpgrade::new(PROTOCOL_NAME), ())
    }
    fn on_behaviour_event(&mut self, event: Self::FromBehaviour) {
        trace!("Received event {:#?}", event);
        self.pending_in_events.push_back(event)
    }
    fn connection_keep_alive(&self) -> bool {
        true
    }
    fn poll(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<
        libp2p::swarm::ConnectionHandlerEvent<
            Self::OutboundProtocol,
            Self::OutboundOpenInfo,
            Self::ToBehaviour,
        >,
    > {
        match self.state {
            State::Inactive { reported: true } => return Poll::Pending,
            State::Inactive { reported: false } => {
                self.state = State::Inactive { reported: true };
                return Poll::Ready(ConnectionHandlerEvent::NotifyBehaviour(
                    ToBehaviourEvent::Unsupported,
                ));
            }
            State::Active => {}
        };

        loop {
            // Check the status of the inbound stream
            match self.inbound_stream.take() {
                // Stream present
                Some(state) => match state {
                    StreamState::Negotiating => {
                        self.inbound_stream = Some(StreamState::Negotiating);
                        break;
                    }
                    // ready to build a server
                    StreamState::Ready(stream) => {
                        let (tx, rx) = mpsc::channel(8);
                        tokio::spawn(async move {
                            let server = http1::Builder::new().serve_connection(
                                hyper_util::rt::TokioIo::new(stream.compat()),
                                service_fn(services::hello),
                            );
                            if let Err(e) = server.await {
                                tx.send(e.to_string()).await.unwrap()
                            }
                        });
                        let _ = self.server_event_tap.insert(rx);
                        self.inbound_stream = Some(StreamState::Consumed);
                        // the server is now listening for requests!
                        info!("server ready");
                    }
                    StreamState::Consumed => self.inbound_stream = Some(StreamState::Consumed),
                },
                // Stream not present
                // this is an inbound stream, we can't request for remote to establish a stream
                None => {
                    self.inbound_stream = None;
                }
            }
            // Check the outbound stream
            match self.outbound_stream.take() {
                // Stream present
                Some(state) => {
                    match state {
                        // Negotiating
                        StreamState::Negotiating => {
                            self.outbound_stream = Some(StreamState::Negotiating);
                            break; // proceed negotiation
                        }
                        // ready
                        StreamState::Ready(stream) => {
                            let (tx, rx) = mpsc::channel(8);
                            let handle = tokio::spawn(async move {
                                let (sender, conn) = hyper::client::conn::http1::Builder::new()
                                    .handshake::<_, String>(hyper_util::rt::TokioIo::new(
                                        stream.compat(),
                                    ))
                                    .await
                                    .expect("operation to succeed");
                                tokio::spawn(async move {
                                    if let Err(e) = conn.await {
                                        tx.send(e.to_string()).await.unwrap()
                                    }
                                });
                                sender
                            });
                            self.client = Some(ClientState::Opening(handle));
                            let _ = self.client_event_tap.insert(rx);
                        }
                        StreamState::Consumed => self.outbound_stream = Some(StreamState::Consumed),
                    }
                }
                //Stream not present
                None => {
                    // Negotiate one
                    info!("requesting new stream");
                    self.outbound_stream = Some(StreamState::Negotiating);
                    let protocol =
                        SubstreamProtocol::new(ReadyUpgrade::new(protocol::PROTOCOL_NAME), ());
                    let event = ConnectionHandlerEvent::OutboundSubstreamRequest { protocol };
                    return Poll::Ready(event); // stop checking and proceed negotiation
                }
            }
            match self.client.take() {
                Some(client) => match client {
                    ClientState::Ready(mut client) => {
                        if self.ongoing_request.is_none() {
                            if let Some((request, callback)) = self.pending_requests.pop_front() {
                                self.ongoing_request =
                                    Some((client.send_request(request).boxed(), callback));
                            }
                            break;
                        }
                    }
                    ClientState::Opening(mut client) => match client.poll_unpin(cx) {
                        Poll::Ready(client) => {
                            self.client = Some(ClientState::Ready(client.unwrap()))
                        }
                        Poll::Pending => {
                            self.client = Some(ClientState::Opening(client));
                            break;
                        }
                    },
                },
                None => {
                    self.outbound_stream = None;
                }
            };
            // Check server events
            if let Some(ev_tap) = &mut self.server_event_tap {
                match ev_tap.poll_recv(cx) {
                    Poll::Ready(maybe_message) => {
                        if let Some(msg) = maybe_message {
                            println!("{}", msg)
                        } else {
                            self.server_event_tap = None;
                            self.inbound_stream = None;
                            warn!("Server dropped!");
                            break;
                        }
                    }
                    Poll::Pending => {}
                }
            }
            // Check client events
            if let Some(ev_tap) = &mut self.client_event_tap {
                match ev_tap.poll_recv(cx) {
                    Poll::Ready(maybe_message) => {
                        if let Some(msg) = maybe_message {
                            println!("{}", msg)
                        } else {
                            self.server_event_tap = None;
                            self.outbound_stream = None;
                            warn!("Client dropped!");
                            break;
                        }
                    }
                    Poll::Pending => {}
                }
            }
        }
        // Check outevents
        if let Some(ev) = self.pending_out_events.pop_front() {
            return Poll::Ready(ConnectionHandlerEvent::NotifyBehaviour(ev));
        }
        Poll::Pending
    }
    fn on_connection_event(
        &mut self,
        event: ConnectionEvent<
            Self::InboundProtocol,
            Self::OutboundProtocol,
            Self::InboundOpenInfo,
            Self::OutboundOpenInfo,
        >,
    ) {
        match event {
            ConnectionEvent::FullyNegotiatedInbound(FullyNegotiatedInbound {
                protocol: stream,
                info: (),
            }) => {
                // got inbound
                self.inbound_stream = Some(StreamState::Ready(stream));
                self.pending_out_events
                    .push_back(ToBehaviourEvent::InboundNegotiated)
            }
            ConnectionEvent::FullyNegotiatedOutbound(FullyNegotiatedOutbound {
                protocol: stream,
                ..
            }) => {
                // got outbound
                self.outbound_stream = Some(StreamState::Ready(stream));
                self.pending_out_events
                    .push_back(ToBehaviourEvent::OutboundNegotiated)
            }
            ConnectionEvent::DialUpgradeError(e) => {
                self.on_dial_upgrade_error(e);
            }
            ConnectionEvent::AddressChange(_) | ConnectionEvent::ListenUpgradeError(_) => {}
            ConnectionEvent::LocalProtocolsChange(_) => {}
            ConnectionEvent::RemoteProtocolsChange(_) => {}
            _ => unimplemented!("New branch not handled!"),
        }
    }
}

enum StreamState {
    Negotiating,
    Ready(Stream),
    Consumed,
}
impl Default for StreamState {
    fn default() -> Self {
        Self::Negotiating
    }
}

pub enum ClientState {
    Ready(SendRequest<String>),
    Opening(JoinHandle<SendRequest<String>>),
}

mod services {
    use hyper::{Request, Response};
    use std::convert::Infallible;

    pub async fn hello(_: Request<hyper::body::Incoming>) -> Result<Response<String>, Infallible> {
        Ok(Response::new("Hello, World!".into()))
    }
}
