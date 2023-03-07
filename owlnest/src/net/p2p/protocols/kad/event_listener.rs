use super::OutEvent;
use libp2p::kad::QueryId;
use tokio::{select, sync::mpsc};

#[repr(i8)]
#[derive(Debug)]
pub enum EventListener {
    OnOutboundQueryProgressed(QueryId, mpsc::Sender<OutEvent>) = 0,
}

pub enum EventListenerOp {
    Add(EventListener),
    Remove(EventListener),
}

pub fn setup_event_listener(
    mut event_in: mpsc::Receiver<OutEvent>,
    mut op_in: mpsc::Receiver<EventListenerOp>,
) {
    tokio::spawn(async move {
        let mut listener_store: Box<[Vec<EventListener>; 1]> = Box::new([vec![]]);
        loop {
            select! {
                Some(ev) = event_in.recv() =>{
                    match ev {
                        OutEvent::OutboundQueryProgressed {
                            id,
                            result,
                            stats,
                            step,
                        } => {
                            for EventListener::OnOutboundQueryProgressed(query,sender) in &mut listener_store[0] {
                                if query.clone() == id{
                                    let ev = OutEvent::OutboundQueryProgressed {
                                        id:id.clone(),
                                        result:result.clone(),
                                        stats:stats.clone(),
                                        step:step.clone(),
                                    };
                                    sender.send(ev).await.unwrap()
                                }
                            }
                        }
                        _ => {}
                    }
                }
                Some(ev) = op_in.recv()=>{
                    match ev{
                        EventListenerOp::Add(listener) => {},
                        EventListenerOp::Remove(id) => todo!(),                    
                    }
                }
            }
        }
    });
}
