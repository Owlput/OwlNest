use libp2p::{identity,PeerId};
use tokio::sync::mpsc;

pub enum Libp2pSwarmOps{

}

pub struct Libp2pSwarmManager{
    sender:mpsc::Sender<Libp2pSwarmOps>
}

impl Libp2pSwarmManager{
    pub fn new()->Self{
        let (sender,mut receiver) = mpsc::channel(8);
        tokio::spawn(async move{
            if let Some(op) = receiver.recv().await{
                
            }
        });
        Libp2pSwarmManager{sender}
    }
}

pub struct Libp2p;