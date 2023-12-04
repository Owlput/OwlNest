use super::handle::SwarmHandle;

macro_rules! generate_manager {
    {$($behaviour_name:ident:$variant_ident:ident,)+} => {
        use std::sync::Arc;
        use tokio::sync::mpsc;
        use futures::{Stream,Future, stream::FusedStream};
        use crate::net::p2p::{swarm,IdentityUnion};
        use core::task::Poll;
        use core::task::Context;
        use core::pin::Pin;

        $(use crate::net::p2p::protocols::$behaviour_name;)*

        pub(crate) struct RxBundle{
            pub swarm:mpsc::Receiver<swarm::InEvent>,
            $(pub $behaviour_name:mpsc::Receiver<$behaviour_name::InEvent>,)*
        }
        impl Future for RxBundle{
            type Output = Rx;

            fn poll(self:Pin<&mut Self>,cx: &mut Context<'_>)->Poll<Self::Output>{
                let mutable_self = self.get_mut();
                match mutable_self.swarm.poll_recv(cx){
                    Poll::Pending => {}
                    Poll::Ready(Some(message)) => return Poll::Ready(Rx::Swarm(message)),
                    Poll::Ready(None) => unreachable!()
                }
                $(
                    match mutable_self.$behaviour_name.poll_recv(cx){
                        Poll::Pending => {}
                        Poll::Ready(Some(message)) => return Poll::Ready(Rx::$variant_ident(message)),
                        Poll::Ready(None) => unreachable!()
                    }
                )+
                Poll::Pending
            }
        }
        impl Stream for RxBundle{
            type Item = Rx;

            fn poll_next(self:Pin<&mut Self>,cx: &mut Context<'_>)->Poll<Option<Self::Item>>{
                let mutable_self = self.get_mut();
                match mutable_self.swarm.poll_recv(cx){
                    Poll::Pending => {}
                    Poll::Ready(Some(message)) => return Poll::Ready(Some(Rx::Swarm(message))),
                    Poll::Ready(None) => unreachable!()
                }
                $(
                    match mutable_self.$behaviour_name.poll_recv(cx){
                        Poll::Pending => {}
                        Poll::Ready(Some(message)) => return Poll::Ready(Some(Rx::$variant_ident(message))),
                        Poll::Ready(None) => unreachable!()
                    }
                )+
                Poll::Pending
            }
        }
        impl FusedStream for RxBundle{
            fn is_terminated(&self)->bool{false}
        }

        pub(crate) enum Rx{
            $($variant_ident($behaviour_name::InEvent),)+
            Swarm(swarm::InEvent)
        }

        pub(crate) struct HandleBundle{
            swarm:SwarmHandle,
            $(pub $behaviour_name:$behaviour_name::Handle,)*
        }
        impl HandleBundle{
            pub fn new(buffer: usize,event_tx:&EventSender)->(Self,RxBundle){
                let swarm = SwarmHandle::new(buffer);
                $(let $behaviour_name = $behaviour_name::Handle::new(buffer,event_tx);)*
                (Self{
                    swarm:swarm.0.clone(),
                    $($behaviour_name:$behaviour_name.0.clone(),)*
                }, RxBundle{
                    swarm:swarm.1,
                    $($behaviour_name:$behaviour_name.1,)*
                })
            }
        }
        #[derive(Clone)]
        pub struct Manager{
            handle_bundle:Arc<HandleBundle>,
            identity:IdentityUnion,
            executor:tokio::runtime::Handle,
            event_out:tokio::sync::broadcast::Sender<Arc<super::SwarmEvent>>,
        }

        impl Manager{
            pub(crate) fn new(
                handle_bundle:Arc<HandleBundle>,
                identity:IdentityUnion,
                executor:tokio::runtime::Handle,
                event_out:tokio::sync::broadcast::Sender<Arc<super::SwarmEvent>>
            )->Self
            {
                Self { handle_bundle,identity, executor, event_out}
            }
            pub fn executor(&self)->&tokio::runtime::Handle{
                &self.executor
            }
            pub fn identity(&self)->&IdentityUnion{
                &self.identity
            }
            pub fn swarm(&self)-> &SwarmHandle{
                &self.handle_bundle.swarm
            }
            pub fn event_subscriber(&self) -> tokio::sync::broadcast::Sender<Arc<super::SwarmEvent>>{
                self.event_out.clone()
            }
            $(pub fn $behaviour_name(&self)->&$behaviour_name::Handle{
                &self.handle_bundle.$behaviour_name
            })*
        }
    };
}

use super::EventSender;

generate_manager! {
    kad:Kad,
    messaging:Messaging,
    mdns:Mdns,
    relay_ext:RelayExt,
}
