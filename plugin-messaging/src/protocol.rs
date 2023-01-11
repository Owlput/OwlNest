use futures::*;

pub const PROTOCOL_NAME:&[u8] = b"/owlput/messaging/0.0.1";

#[derive(Default,Debug,Clone, Copy)]
pub struct Messaging;

pub async fn send<S>(stream:S)->io::Result<S>
where S:AsyncWrite + Unpin
{
    Ok(stream)
}