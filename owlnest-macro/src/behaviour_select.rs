#[macro_export]
macro_rules! generate_event_select {
    ($(#[$meta:meta])*$name:ident{$($behaviour:ident:$ev:ty,)*}) => {
        $(#[$meta])*
        pub enum $name{
            $($behaviour($ev),)*
        }
    };
    ($(#[$meta:meta])*$name:ident{$($behaviour:ident:$variant:ident,)*};) => {
        $(#[$meta])*
        pub enum $name{
            $($variant(<$behaviour::Behaviour as ::libp2p::swarm::NetworkBehaviour>::ToSwarm),)*
        }
    };
}

#[macro_export]
macro_rules! behaviour_select {

    {$($name:ident:$behaviour:ident,)*} => {
        use ::owlnest_macro::*;
        $(use crate::net::p2p::protocols::$name;)*
        use tracing::trace;

        generate_select_struct!(Behaviour{$($name:$name::Behaviour,)*});

        generate_event_select!(
            #[derive(Debug)]
            /// A collection of all events from behaviours
            BehaviourEvent{
            $($behaviour:<$name::Behaviour as ::libp2p::swarm::NetworkBehaviour>::ToSwarm,)*
        });

        connection_handler_select!{
            $($name=>$behaviour:<$name::Behaviour as ::libp2p::swarm::NetworkBehaviour>::ConnectionHandler,)*
        }
        impl ::libp2p::swarm::NetworkBehaviour for Behaviour{
            type ConnectionHandler = handler::ConnectionHandlerSelect;
            type ToSwarm = BehaviourEvent;
            fn handle_pending_inbound_connection(
                &mut self,
                connection_id: ::libp2p::swarm::ConnectionId,
                local_addr: &::libp2p::Multiaddr,
                remote_addr: &::libp2p::Multiaddr,
            )->Result<(),::libp2p::swarm::ConnectionDenied>{
                $(::libp2p::swarm::NetworkBehaviour::handle_pending_inbound_connection(
                    &mut self.$name,
                    connection_id,
                    local_addr,
                    remote_addr,
                )?;)*
                Ok(())
            }
            fn handle_established_inbound_connection(
                &mut self,
                connection_id: ::libp2p::swarm::ConnectionId,
                peer: ::libp2p::PeerId,
                local_addr: &::libp2p::Multiaddr,
                remote_addr: &::libp2p::Multiaddr,
            ) -> Result<
                Self::ConnectionHandler,
                ::libp2p::swarm::ConnectionDenied,
            >{

                let handler = Self::ConnectionHandler{
                    $($name:self.$name.handle_established_inbound_connection(
                        connection_id,
                        peer,
                        local_addr,
                        remote_addr,
                    )?,)*
                };
                Ok(handler)

            }
            fn handle_pending_outbound_connection(
                &mut self,
                connection_id: ::libp2p::swarm::ConnectionId,
                maybe_peer: Option<::libp2p::PeerId>,
                addresses: &[::libp2p::Multiaddr],
                effective_role: ::libp2p_swarm::derive_prelude::Endpoint,
            ) -> Result<
                ::std::vec::Vec<::libp2p::Multiaddr>,
                ::libp2p::swarm::ConnectionDenied,
            > {
                let mut combined_addresses = Vec::new();
                $(
                    combined_addresses.extend(
                        ::libp2p::swarm::NetworkBehaviour::handle_pending_outbound_connection(
                            &mut self.$name,
                            connection_id,
                            maybe_peer,
                            addresses,
                            effective_role,
                        )?
                    );
                )*
                Ok(combined_addresses)
            }
            fn handle_established_outbound_connection(
                &mut self,
                connection_id: ::libp2p::swarm::ConnectionId,
                peer: ::libp2p::PeerId,
                addr: &::libp2p::Multiaddr,
                role_override: ::libp2p_swarm::derive_prelude::Endpoint,
            ) -> Result<
                Self::ConnectionHandler,
                ::libp2p::swarm::ConnectionDenied,
            > {
                let handler = Self::ConnectionHandler{
                    $($name:self.$name.handle_established_outbound_connection(
                        connection_id,
                        peer,
                        addr,
                        role_override,
                    )?,)*
                };
                Ok(handler)
            }
            fn on_connection_handler_event(
                &mut self,
                peer_id: ::libp2p::PeerId,
                connection_id: ::libp2p::swarm::ConnectionId,
                event: handler::ToBehaviourSelect,
            ) {
                match event{
                    $(handler::ToBehaviourSelect::$behaviour(ev)=>{
                        ::libp2p::swarm::NetworkBehaviour::on_connection_handler_event(
                            &mut self.$name,
                            peer_id,
                            connection_id,
                            ev,
                        )
                    },)*
                }
            }
            fn poll(
                &mut self,
                cx: &mut std::task::Context,
            ) -> std::task::Poll<
                ::libp2p::swarm::derive_prelude::ToSwarm<
                    Self::ToSwarm,
                    ::libp2p::swarm::derive_prelude::THandlerInEvent<Self>,
                >,
            > {
                trace!(name:"Poll","polling behaviour");
                $(
                    match ::libp2p::swarm::NetworkBehaviour::poll(
                        &mut self.$name,
                        cx,
                    ){
                        std::task::Poll::Ready(
                            ::libp2p::swarm::derive_prelude::ToSwarm::GenerateEvent(
                                event,
                            ),
                        ) => {
                            trace!(name:"Poll","behaviour {} generated an event",stringify!($name));
                            return std::task::Poll::Ready(
                                ::libp2p::swarm::derive_prelude::ToSwarm::GenerateEvent(
                                    BehaviourEvent::$behaviour(event),
                                ),
                            );
                        }
                        std::task::Poll::Ready(
                            ::libp2p::swarm::derive_prelude::ToSwarm::Dial { opts },
                        ) => {
                            trace!(name:"Poll","behaviour {} is trying to dial",stringify!($name));
                            return std::task::Poll::Ready(::libp2p::swarm::derive_prelude::ToSwarm::Dial {
                                opts,
                            });
                        }
                        std::task::Poll::Ready(
                            ::libp2p::swarm::derive_prelude::ToSwarm::ListenOn { opts },
                        ) => {
                            trace!(name:"Poll","behaviour {} is trying to listen",stringify!($name));
                            return std::task::Poll::Ready(::libp2p::swarm::derive_prelude::ToSwarm::ListenOn {
                                opts,
                            });
                        }
                        std::task::Poll::Ready(::libp2p::swarm::derive_prelude::ToSwarm::RemoveListener{id})=>{
                            trace!(name:"Poll","behaviour {} is removing listener id {}",stringify!($name), id);
                            return std::task::Poll::Ready(::libp2p::swarm::derive_prelude::ToSwarm::RemoveListener {
                                id
                            });
                        }
                        std::task::Poll::Ready(
                            ::libp2p::swarm::derive_prelude::ToSwarm::NotifyHandler {
                                peer_id,
                                handler,
                                event,
                            },
                        ) => {
                            trace!(name:"Poll","behaviour {} is notifying its handler",stringify!($name));
                            return std::task::Poll::Ready(::libp2p::swarm::derive_prelude::ToSwarm::NotifyHandler {
                                peer_id,
                                handler,
                                event: handler::FromBehaviourSelect::$behaviour(event)
                            });
                        }
                        std::task::Poll::Ready(::libp2p::swarm::derive_prelude::ToSwarm::NewExternalAddrCandidate(addr))=>{
                            return std::task::Poll::Ready(::libp2p::swarm::derive_prelude::ToSwarm::NewExternalAddrCandidate(addr));
                        }
                        std::task::Poll::Ready(::libp2p::swarm::derive_prelude::ToSwarm::ExternalAddrConfirmed(addr))=>{
                            return std::task::Poll::Ready(::libp2p::swarm::derive_prelude::ToSwarm::ExternalAddrConfirmed(addr));
                        }
                        std::task::Poll::Ready(::libp2p::swarm::derive_prelude::ToSwarm::ExternalAddrExpired(addr))=>{
                            return std::task::Poll::Ready(::libp2p::swarm::derive_prelude::ToSwarm::ExternalAddrExpired(addr));
                        }
                        std::task::Poll::Ready(
                            ::libp2p::swarm::derive_prelude::ToSwarm::CloseConnection {
                                peer_id,
                                connection,
                            },
                        ) => {
                            trace!(name:"Poll","behaviour {} is closing connection to peer {}",stringify!($name), peer_id);
                            return std::task::Poll::Ready(::libp2p::swarm::derive_prelude::ToSwarm::CloseConnection {
                                peer_id,
                                connection,
                            });
                        }
                        std::task::Poll::Ready(::libp2p::swarm::derive_prelude::ToSwarm::NewExternalAddrOfPeer{peer_id,address})=>{
                            return std::task::Poll::Ready(::libp2p::swarm::derive_prelude::ToSwarm::NewExternalAddrOfPeer{peer_id,address});
                        }
                        std::task::Poll::Ready(uncovered)=>{
                            unimplemented!("New branch {:?} not covered", uncovered)
                        }
                        std::task::Poll::Pending => {}
                    }
                )*
                trace!(name:"Poll","Nothing happening to swarm");
                std::task::Poll::Pending
            }
            fn on_swarm_event(
                &mut self,
                event: ::libp2p::swarm::derive_prelude::FromSwarm,
            ) {
                $(self.$name.on_swarm_event(event);)*
            }
        }
    }
}

#[macro_export]
macro_rules! generate_select_enum {
    ($(#[$meta:meta])*$name:ident$(<$life:lifetime>)?{$($identifier:ident:$inner:ty,)*}) => {
        $(#[$meta])*
        pub enum $name$(<$life>)?{
            $($identifier($inner),)*
        }
    };
}

#[macro_export]
macro_rules! generate_select_struct {
    ($(#[$meta:meta])*$name:ident$(<$life:lifetime>)?{$($field:ident:$inner:ty,)*}) => {
        $(#[$meta])*
        pub struct $name$(<$life>)?{
            $(pub $field:$inner,)*
        }
    };
}

#[macro_export]
macro_rules! generate_error_select_enum {
    ($(#[$meta:meta])*$name:ident$(<$life:lifetime>)?{$($field:ident:$inner:ty,)*}) => {
        ::owlnest_macro::generate_select_enum!(#[derive(Debug)]$name$(<$life>)?{$($field:$inner,)*});
        impl std::fmt::Display for $name{
            fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->Result<(),std::fmt::Error>{
                match self{
                    $(Self::$field(e)=>std::fmt::Display::fmt(e,f),)*
                }
            }
        }
        impl serde::ser::StdError for $name{

        }
    };
}
