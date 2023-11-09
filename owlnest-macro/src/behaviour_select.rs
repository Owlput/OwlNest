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

        generate_select_struct!(Behaviour{$($name:$name::Behaviour,)*});

        const EVENT_IDENT:&str = "swarmEvent";
        generate_event_select!(
            #[derive(Debug)]
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
                            return std::task::Poll::Ready(
                                ::libp2p::swarm::derive_prelude::ToSwarm::GenerateEvent(
                                    BehaviourEvent::$behaviour(event),
                                ),
                            );
                        }
                        std::task::Poll::Ready(
                            ::libp2p::swarm::derive_prelude::ToSwarm::Dial { opts },
                        ) => {
                            return std::task::Poll::Ready(::libp2p::swarm::derive_prelude::ToSwarm::Dial {
                                opts,
                            });
                        }
                        std::task::Poll::Ready(
                            ::libp2p::swarm::derive_prelude::ToSwarm::NotifyHandler {
                                peer_id,
                                handler,
                                event,
                            },
                        ) => {
                            return std::task::Poll::Ready(::libp2p::swarm::derive_prelude::ToSwarm::NotifyHandler {
                                peer_id,
                                handler,
                                event: handler::FromBehaviourSelect::$behaviour(event)
                            });
                        }
                        std::task::Poll::Ready(
                            ::libp2p::swarm::derive_prelude::ToSwarm::CloseConnection {
                                peer_id,
                                connection,
                            },
                        ) => {
                            return std::task::Poll::Ready(::libp2p::swarm::derive_prelude::ToSwarm::CloseConnection {
                                peer_id,
                                connection,
                            });
                        }
                        std::task::Poll::Ready(::libp2p::swarm::derive_prelude::ToSwarm::ListenOn{opts})=>{
                            return std::task::Poll::Ready(::libp2p::swarm::derive_prelude::ToSwarm::ListenOn {
                                opts
                            });
                        }
                        std::task::Poll::Ready(::libp2p::swarm::derive_prelude::ToSwarm::NewExternalAddrCandidate(addr))=>{
                            return std::task::Poll::Ready(::libp2p::swarm::derive_prelude::ToSwarm::NewExternalAddrCandidate(addr));
                        }
                        std::task::Poll::Ready(::libp2p::swarm::derive_prelude::ToSwarm::ExternalAddrConfirmed(addr))=>{
                            return std::task::Poll::Ready(::libp2p::swarm::derive_prelude::ToSwarm::ExternalAddrConfirmed(addr));
                        }
                        std::task::Poll::Ready(::libp2p::swarm::derive_prelude::ToSwarm::RemoveListener{id})=>{
                            return std::task::Poll::Ready(::libp2p::swarm::derive_prelude::ToSwarm::RemoveListener {
                                id
                            });
                        }
                        std::task::Poll::Ready(::libp2p::swarm::derive_prelude::ToSwarm::ExternalAddrExpired(addr))=>{
                            return std::task::Poll::Ready(::libp2p::swarm::derive_prelude::ToSwarm::ExternalAddrExpired(addr));
                        }
                        std::task::Poll::Ready(_)=>{
                            unimplemented!("New branch not covered")
                        }
                        std::task::Poll::Pending => {}
                    }
                )*
                std::task::Poll::Pending
            }
            fn on_swarm_event(
                &mut self,
                event: ::libp2p::swarm::derive_prelude::FromSwarm,
            ) {
                match event {
                    ::libp2p::swarm::derive_prelude::FromSwarm::ConnectionEstablished(
                        ::libp2p::swarm::derive_prelude::ConnectionEstablished {
                            peer_id,
                            connection_id,
                            endpoint,
                            failed_addresses,
                            other_established,
                        },
                    ) => {
                        $(self.$name
                            .on_swarm_event(
                                ::libp2p::swarm::derive_prelude::FromSwarm::ConnectionEstablished(::libp2p::swarm::derive_prelude::ConnectionEstablished {
                                    peer_id,
                                    connection_id,
                                    endpoint,
                                    failed_addresses,
                                    other_established,
                                }),
                            );)*
                    }
                    ::libp2p::swarm::derive_prelude::FromSwarm::AddressChange(
                        ::libp2p::swarm::derive_prelude::AddressChange {
                            peer_id,
                            connection_id,
                            old,
                            new,
                        },
                    ) => {
                        $(self.$name
                            .on_swarm_event(
                                ::libp2p::swarm::derive_prelude::FromSwarm::AddressChange(::libp2p::swarm::derive_prelude::AddressChange {
                                    peer_id,
                                    connection_id,
                                    old,
                                    new,
                                }),
                            );)*
                    }
                    ::libp2p::swarm::derive_prelude::FromSwarm::ConnectionClosed(
                        ::libp2p::swarm::derive_prelude::ConnectionClosed {
                            peer_id,
                            connection_id,
                            endpoint,
                            remaining_established,
                        },
                    ) => {
                        $(self.$name
                            .on_swarm_event(
                                ::libp2p::swarm::derive_prelude::FromSwarm::ConnectionClosed(::libp2p::swarm::derive_prelude::ConnectionClosed {
                                    peer_id,
                                    connection_id,
                                    endpoint,
                                    remaining_established,
                                }),
                            );)*
                    }
                    ::libp2p::swarm::derive_prelude::FromSwarm::DialFailure(
                        ::libp2p::swarm::derive_prelude::DialFailure {
                            peer_id,
                            connection_id,
                            error,
                        },
                    ) => {
                        $(self.$name
                            .on_swarm_event(
                                ::libp2p::swarm::derive_prelude::FromSwarm::DialFailure(::libp2p::swarm::derive_prelude::DialFailure {
                                    peer_id,
                                    connection_id,
                                    error,
                                }),
                            );)*
                    }
                    ::libp2p::swarm::derive_prelude::FromSwarm::ListenFailure(
                        ::libp2p::swarm::derive_prelude::ListenFailure {
                            local_addr,
                            send_back_addr,
                            connection_id,
                            error,
                        },
                    ) => {
                        $(self.$name
                            .on_swarm_event(
                                ::libp2p::swarm::derive_prelude::FromSwarm::ListenFailure(::libp2p::swarm::derive_prelude::ListenFailure {
                                    local_addr,
                                    send_back_addr,
                                    connection_id,
                                    error,
                                }),
                            );)*
                    }
                    ::libp2p::swarm::derive_prelude::FromSwarm::NewListener(
                        ::libp2p::swarm::derive_prelude::NewListener { listener_id },
                    ) => {
                        $(self.$name
                            .on_swarm_event(
                                ::libp2p::swarm::derive_prelude::FromSwarm::NewListener(::libp2p::swarm::derive_prelude::NewListener {
                                    listener_id,
                                }),
                            );)*
                    }
                    ::libp2p::swarm::derive_prelude::FromSwarm::NewListenAddr(
                        ::libp2p::swarm::derive_prelude::NewListenAddr {
                            listener_id,
                            addr,
                        },
                    ) => {
                        $(self.$name
                            .on_swarm_event(
                                ::libp2p::swarm::derive_prelude::FromSwarm::NewListenAddr(::libp2p::swarm::derive_prelude::NewListenAddr {
                                    listener_id,
                                    addr,
                                }),
                            );)*
                    }
                    ::libp2p::swarm::derive_prelude::FromSwarm::ExpiredListenAddr(
                        ::libp2p::swarm::derive_prelude::ExpiredListenAddr {
                            listener_id,
                            addr,
                        },
                    ) => {
                        $(self.$name
                            .on_swarm_event(
                                ::libp2p::swarm::derive_prelude::FromSwarm::ExpiredListenAddr(::libp2p::swarm::derive_prelude::ExpiredListenAddr {
                                    listener_id,
                                    addr,
                                }),
                            );)*
                    }
                    ::libp2p::swarm::derive_prelude::FromSwarm::ListenerError(
                        ::libp2p::swarm::derive_prelude::ListenerError {
                            listener_id,
                            err,
                        },
                    ) => {
                        $(self.$name
                            .on_swarm_event(
                                ::libp2p::swarm::derive_prelude::FromSwarm::ListenerError(::libp2p::swarm::derive_prelude::ListenerError {
                                    listener_id,
                                    err,
                                }),
                            );)*
                    }
                    ::libp2p::swarm::derive_prelude::FromSwarm::ListenerClosed(
                        ::libp2p::swarm::derive_prelude::ListenerClosed {
                            listener_id,
                            reason,
                        },
                    ) => {
                        $(self.$name
                            .on_swarm_event(
                                ::libp2p::swarm::derive_prelude::FromSwarm::ListenerClosed(::libp2p::swarm::derive_prelude::ListenerClosed {
                                    listener_id,
                                    reason,
                                }),
                            );)*
                    }
                    #[allow(unreachable_patterns)]
                    _ => {}
                }
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
