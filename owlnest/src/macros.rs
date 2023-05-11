#[macro_export]
macro_rules! generate_event_select {
    ($name:ident{$($behaviour:ident:$ev:ty,)*}) => {
        #[derive(Debug)]
        pub enum $name{
            $($behaviour($ev),)*
        }
        // $(impl From<$ev> for $name{
        //     fn from(value:$ev)->$name{
        //         Self::$behaviour(value)
        //     }
        // })*
        // $(impl Into<$ev> for $name{
        //     fn into(self)->$ev{
        //         match self{
        //             Self::$behaviour(ev)=>ev,
        //             _=>panic!("Internal conversion error")
        //         }
        //     }
        // })*
    };
}

#[macro_export]
macro_rules! behaviour_select {
    {$($name:ident=>$behaviour:ident:$behaviour_type:ty,)*} => {
        use libp2p::swarm::*;
        use libp2p::{PeerId,Multiaddr};
        use crate::*;

        generate_select_struct!(Behaviour{$($name:$behaviour_type,)*});
        generate_event_select!(OutEvent{
            $($behaviour:<$behaviour_type as NetworkBehaviour>::OutEvent,)*
        });


        connection_handler_select!{
            $($name=>$behaviour:<$behaviour_type as NetworkBehaviour>::ConnectionHandler,)*
        }
        impl NetworkBehaviour for Behaviour{
            type ConnectionHandler = ConnectionHandlerSelect;
            type OutEvent = OutEvent;
            fn handle_pending_inbound_connection(
                &mut self,
                connection_id: ConnectionId,
                local_addr: &Multiaddr,
                remote_addr: &Multiaddr,
            )->Result<(),ConnectionDenied>{
                $(NetworkBehaviour::handle_pending_inbound_connection(
                    &mut self.$name,
                    connection_id,
                    local_addr,
                    remote_addr,
                )?;)*
                Ok(())
            }
            fn handle_established_inbound_connection(
                &mut self,
                connection_id: ConnectionId,
                peer: PeerId,
                local_addr: &Multiaddr,
                remote_addr: &Multiaddr,
            ) -> Result<
                Self::ConnectionHandler,
                ConnectionDenied,
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
                connection_id: ConnectionId,
                maybe_peer: Option<PeerId>,
                addresses: &[Multiaddr],
                effective_role: derive_prelude::Endpoint,
            ) -> Result<
                ::std::vec::Vec<Multiaddr>,
                ConnectionDenied,
            > {
                let mut combined_addresses = Vec::new();
                $(
                    combined_addresses.extend(
                        NetworkBehaviour::handle_pending_outbound_connection(
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
                connection_id: ConnectionId,
                peer: PeerId,
                addr: &Multiaddr,
                role_override: derive_prelude::Endpoint,
            ) -> Result<
                Self::ConnectionHandler,
                ConnectionDenied,
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
                peer_id: PeerId,
                connection_id: ConnectionId,
                event: ToBehaviourSelect,
            ) {
                match event{
                    $(ToBehaviourSelect::$behaviour(ev)=>{
                        NetworkBehaviour::on_connection_handler_event(
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
                poll_params: &mut impl ::libp2p::swarm::derive_prelude::PollParameters,
            ) -> std::task::Poll<
                ::libp2p::swarm::derive_prelude::ToSwarm<
                    Self::OutEvent,
                    ::libp2p::swarm::derive_prelude::THandlerInEvent<Self>,
                >,
            > {
                $(
                    match NetworkBehaviour::poll(
                        &mut self.$name,
                        cx,
                        poll_params,
                    ){
                        std::task::Poll::Ready(
                            ::libp2p::swarm::derive_prelude::ToSwarm::GenerateEvent(
                                event,
                            ),
                        ) => {
                            return std::task::Poll::Ready(
                                ::libp2p::swarm::derive_prelude::ToSwarm::GenerateEvent(
                                    OutEvent::$behaviour(event),
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
                                event: FromBehaviourSelect::$behaviour(event)
                            });
                        }
                        std::task::Poll::Ready(
                            ::libp2p::swarm::derive_prelude::ToSwarm::ReportObservedAddr {
                                address,
                                score,
                            },
                        ) => {
                            return std::task::Poll::Ready(::libp2p::swarm::derive_prelude::ToSwarm::ReportObservedAddr {
                                address,
                                score,
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
                        std::task::Poll::Pending => {}
                    }
                )*
                Poll::Pending
            }
            fn on_swarm_event(
                &mut self,
                event: ::libp2p::swarm::derive_prelude::FromSwarm<
                    Self::ConnectionHandler,
                >,
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
                            handler: handlers,
                            remaining_established,
                        },
                    ) => {
                        let ($($name,)*) = handlers.into_inner();
                        $(self.$name
                            .on_swarm_event(
                                ::libp2p::swarm::derive_prelude::FromSwarm::ConnectionClosed(::libp2p::swarm::derive_prelude::ConnectionClosed {
                                    peer_id,
                                    connection_id,
                                    endpoint,
                                    handler:$name,
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
                    ::libp2p::swarm::derive_prelude::FromSwarm::NewExternalAddr(
                        ::libp2p::swarm::derive_prelude::NewExternalAddr { addr },
                    ) => {
                        $(self.$name
                            .on_swarm_event(
                                ::libp2p::swarm::derive_prelude::FromSwarm::NewExternalAddr(::libp2p::swarm::derive_prelude::NewExternalAddr {
                                    addr,
                                }),
                            );)*
                    }
                    ::libp2p::swarm::derive_prelude::FromSwarm::ExpiredExternalAddr(
                        ::libp2p::swarm::derive_prelude::ExpiredExternalAddr {
                            addr,
                        },
                    ) => {
                        $(self.$name
                            .on_swarm_event(
                                ::libp2p::swarm::derive_prelude::FromSwarm::ExpiredExternalAddr(::libp2p::swarm::derive_prelude::ExpiredExternalAddr {
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
macro_rules! connection_handler_select {
    {$($name:ident=>$behaviour:ident:$handler:ty,)*} => {
        use libp2p::swarm::handler::ConnectionEvent;
        use std::task::Poll;

        pub mod upgrade{
            pub use libp2p::core::upgrade::UpgradeInfo;
            pub mod inbound{
                pub use libp2p::core::upgrade::InboundUpgrade as Upgrade;
            }
            pub mod outbound{
                pub use libp2p::core::upgrade::OutboundUpgrade as Upgrade;
            }
        }

        pub struct ConnectionHandlerSelect{$($name:$handler,)*}
        impl ConnectionHandlerSelect{
            pub fn into_inner(self)->($($handler,)*){
                ($(self.$name,)*)
            }
        }

        generate_event_select!(FromBehaviourSelect{$($behaviour:<$handler as ConnectionHandler>::InEvent,)*});
        generate_event_select!(ToBehaviourSelect{$($behaviour:<$handler as ConnectionHandler>::OutEvent,)*});
        generate_error_select_enum!(HandlerErrorSelect{$($behaviour:<$handler as ConnectionHandler>::Error,)*});
        generate_inbound_upgrade_select!(inbound,upgrade_inbound{$($name=>$behaviour:<$handler as ConnectionHandler>::InboundProtocol,)*});
        generate_outbound_upgrade_select!(outbound,upgrade_outbound{$($name=>$behaviour:<$handler as ConnectionHandler>::OutboundProtocol,)*});
        generate_select_enum!(OutboundOpenInfoSelect{$($behaviour:<$handler as ConnectionHandler>::OutboundOpenInfo,)*});
        generate_select_struct!(InboundOpenInfoSelect{$($name:<$handler as ConnectionHandler>::InboundOpenInfo,)*});
        generate_outbound_transpose!($($name=>$behaviour:$handler,)*);
        generate_inbound_transpose!($($name=>$behaviour:$handler,)*);
        generate_upgr_error_transpose!($($name=>$behaviour:$handler,)*);

        impl ConnectionHandlerSelect{
            fn on_listen_upgrade_error(
                &mut self,
                handler::ListenUpgradeError {
                    info,
                    error,
                }: handler::ListenUpgradeError<
                    <Self as ConnectionHandler>::InboundOpenInfo,
                    <Self as ConnectionHandler>::InboundProtocol,
                >,
            ) {
                match error {
                    $(inbound::UpgradeErrorSelect::$behaviour(error)=> self.$name.on_connection_event(handler::ConnectionEvent::ListenUpgradeError(handler::ListenUpgradeError{
                        info:info.$name,
                        error
                    })),)*
                }
            }
        }

        impl libp2p::swarm::ConnectionHandler for ConnectionHandlerSelect{
            type InEvent = FromBehaviourSelect;

            type OutEvent = ToBehaviourSelect;

            type Error = HandlerErrorSelect;

            type InboundProtocol = inbound::UpgradeSelect;

            type OutboundProtocol = outbound::UpgradeSelect;

            type InboundOpenInfo = InboundOpenInfoSelect;

            type OutboundOpenInfo = OutboundOpenInfoSelect;

            fn listen_protocol(&self) -> libp2p::swarm::SubstreamProtocol<Self::InboundProtocol, Self::InboundOpenInfo> {
                $(let $name = self.$name.listen_protocol().into_upgrade();)*
                generate_substream_protocol($($name,)*)
            }
            fn connection_keep_alive(&self) -> libp2p::swarm::KeepAlive {
                vec![$(self.$name.connection_keep_alive(),)*].iter().max().unwrap_or(&libp2p::swarm::KeepAlive::Yes).clone()
            }

            fn poll(
                &mut self,
                cx: &mut std::task::Context<'_>,
            ) -> std::task::Poll<
                libp2p::swarm::ConnectionHandlerEvent<
                    Self::OutboundProtocol,
                    Self::OutboundOpenInfo,
                    Self::OutEvent,
                    Self::Error,
                >,
            > {
                $(
                    match self.$name.poll(cx) {
                    Poll::Ready(ConnectionHandlerEvent::Custom(event)) => {
                        return Poll::Ready(ConnectionHandlerEvent::Custom(ToBehaviourSelect::$behaviour(event)));
                    }
                    Poll::Ready(ConnectionHandlerEvent::Close(event)) => {
                        return Poll::Ready(ConnectionHandlerEvent::Close(HandlerErrorSelect::$behaviour(event)));
                    }
                    Poll::Ready(ConnectionHandlerEvent::OutboundSubstreamRequest { protocol }) => {
                        return Poll::Ready(ConnectionHandlerEvent::OutboundSubstreamRequest {
                            protocol: protocol
                                .map_upgrade(|u| outbound::UpgradeSelect::$behaviour(u))
                                .map_info(|i|OutboundOpenInfoSelect::$behaviour(i))
                        });
                    }
                    Poll::Pending => (),
                    _ => ()
                };)*
                Poll::Pending
            }

            fn on_behaviour_event(&mut self, event: Self::InEvent) {
                match event{
                    $(FromBehaviourSelect::$behaviour(ev)=>self.$name.on_behaviour_event(ev),)*
                }
            }

            fn on_connection_event(
                &mut self,
                event: libp2p::swarm::handler::ConnectionEvent<
                    Self::InboundProtocol,
                    Self::OutboundProtocol,
                    Self::InboundOpenInfo,
                    Self::OutboundOpenInfo,
                >,
            ) {
                match event {
                    ConnectionEvent::FullyNegotiatedOutbound(fully_negotiated_outbound) => {
                        match transpose_full_outbound(fully_negotiated_outbound) {
                            $(FullyNegotiatedOutboundSelect::$behaviour(inner)=>self.$name.on_connection_event(ConnectionEvent::FullyNegotiatedOutbound(inner)),)*
                        }
                    }
                    ConnectionEvent::FullyNegotiatedInbound(fully_negotiated_inbound) => {
                        match transpose_full_inbound(fully_negotiated_inbound) {
                            $(FullyNegotiatedInboundSelect::$behaviour(inner)=>self.$name.on_connection_event(ConnectionEvent::FullyNegotiatedInbound(inner)),)*
                        }
                    }
                    ConnectionEvent::AddressChange(address) => {
                        $(self.$name
                            .on_connection_event(ConnectionEvent::AddressChange(libp2p::swarm::handler::AddressChange {
                                new_address: address.new_address,
                            }));)*
                    }
                    ConnectionEvent::DialUpgradeError(dial_upgrade_error) => {
                        match transpose_upgr_error(dial_upgrade_error) {
                            $(DialUpgradeErrorSelect::$behaviour(inner)=>self.$name.on_connection_event(ConnectionEvent::DialUpgradeError(inner)),)*
                        }
                    }
                    ConnectionEvent::ListenUpgradeError(listen_upgrade_error) => {
                        self.on_listen_upgrade_error(listen_upgrade_error)
                    }
                    ConnectionEvent::LocalProtocolsChange(supported_protocols) => {
                        $(self.$name
                            .on_connection_event(ConnectionEvent::LocalProtocolsChange(
                                supported_protocols.clone(),
                            ));)*
                    }
                    ConnectionEvent::RemoteProtocolsChange(supported_protocols) => {
                        $(self.$name
                            .on_connection_event(ConnectionEvent::RemoteProtocolsChange(
                                supported_protocols.clone(),
                            ));)*
                    }
                }
            }
        }
        fn generate_substream_protocol($($name:(<$handler as ConnectionHandler>::InboundProtocol,<$handler as ConnectionHandler>::InboundOpenInfo),)*)->SubstreamProtocol<inbound::UpgradeSelect,<ConnectionHandlerSelect as ConnectionHandler>::InboundOpenInfo>{
            SubstreamProtocol::new(inbound::UpgradeSelect{$($name:$name.0,)*},InboundOpenInfoSelect{$($name:$name.1,)*})
        }
    };
}

#[macro_export]
macro_rules! generate_inbound_upgrade_select {
    ($direction:ident,$impl_name:ident{$($name:ident=>$behaviour:ident:$upgrade:ty,)*}) => {
        pub(crate) mod $direction{
            use crate::*;
            use super::*;
            use libp2p::swarm::NegotiatedSubstream;

        generate_select_enum!(#[derive(Clone)]UpgradeInfoSelect{$($behaviour:<$upgrade as upgrade::UpgradeInfo>::Info,)*});
        impl AsRef<str> for UpgradeInfoSelect{
            fn as_ref(&self)->&str{
                match self{
                    $(Self::$behaviour(inner)=>AsRef::<str>::as_ref(inner),)*
                }
            }
        }
        generate_select_struct!(#[derive(Clone)]UpgradeInfoIterSelect{$($name:Option<std::iter::Map<<<$upgrade as upgrade::UpgradeInfo>::InfoIter as IntoIterator>::IntoIter, fn(<$upgrade as upgrade::UpgradeInfo>::Info) -> UpgradeInfoSelect>>,)*});
        impl Iterator for UpgradeInfoIterSelect{
            type Item = UpgradeInfoSelect;
            fn next(&mut self)->Option<UpgradeInfoSelect>{
                $(
                    if let Some(map) = &mut self.$name{
                        match map.next(){
                            Some(v)=> return Some(v),
                            None => self.$name = None,
                        }
                    }
                )*
                None
            }
        }

        generate_select_struct!(
            UpgradeSelect{$($name:$upgrade,)*}
        );

        generate_select_enum!(UpgradeErrorSelect{$($behaviour:<$upgrade as upgrade::$direction::Upgrade<NegotiatedSubstream>>::Error,)*});

        impl upgrade::UpgradeInfo for UpgradeSelect{
            type Info = UpgradeInfoSelect;
            type InfoIter = UpgradeInfoIterSelect;

            fn protocol_info(&self) -> Self::InfoIter {
                UpgradeInfoIterSelect{
                    $($name:Some(self.$name.protocol_info().into_iter().map(UpgradeInfoSelect::$behaviour as fn(<$upgrade as upgrade::UpgradeInfo>::Info)->_)),)*
                }
            }
        }

        generate_select_enum!(SubstreamSelect{$($behaviour:<$upgrade as upgrade::$direction::Upgrade<NegotiatedSubstream>>::Output,)*});
        generate_future_select!($($upgrade|$behaviour:<$upgrade as upgrade::$direction::Upgrade<NegotiatedSubstream>>::Future,)*); // collected future

        impl upgrade::$direction::Upgrade<NegotiatedSubstream> for UpgradeSelect{
            type Output = SubstreamSelect;
            type Future = FutureSelect;
            type Error = UpgradeErrorSelect;

            fn $impl_name(self, sock: NegotiatedSubstream, info: UpgradeInfoSelect) -> Self::Future {
                match info {
                    $(UpgradeInfoSelect::$behaviour(info)=>Self::Future::$behaviour(self.$name.$impl_name(sock,info)),)*
                }
            }
        }
    }
    };
}

#[macro_export]
macro_rules! generate_outbound_upgrade_select {
    ($direction:ident,$impl_name:ident{$($name:ident=>$behaviour:ident:$upgrade:ty,)*}) => {
        pub(crate) mod $direction{
            use crate::*;
            use super::*;
            use libp2p::swarm::NegotiatedSubstream;
        generate_select_enum!(#[derive(Clone)]UpgradeInfoSelect{$($behaviour:<$upgrade as upgrade::UpgradeInfo>::Info,)*});
        impl AsRef<str> for UpgradeInfoSelect{
            fn as_ref(&self)->&str{
                match self{
                    $(Self::$behaviour(inner)=>AsRef::<str>::as_ref(inner),)*
                }
            }
        }
        generate_select_enum!(
            UpgradeSelect{$($behaviour:$upgrade,)*}
        );

        generate_select_enum!(UpgradeErrorSelect{$($behaviour:<$upgrade as upgrade::$direction::Upgrade<NegotiatedSubstream>>::Error,)*});

        impl upgrade::UpgradeInfo for UpgradeSelect{
            type Info = UpgradeInfoSelect;
            type InfoIter = core::iter::Once<UpgradeInfoSelect>;

            fn protocol_info(&self) -> Self::InfoIter {
                match self{
                    $(Self::$behaviour(inner)=>core::iter::once(UpgradeInfoSelect::$behaviour(inner.protocol_info().next().unwrap())),)*
                }
            }
        }

        generate_select_enum!(SubstreamSelect{$($behaviour:<$upgrade as upgrade::$direction::Upgrade<NegotiatedSubstream>>::Output,)*});
        generate_future_select!($($upgrade|$behaviour:<$upgrade as upgrade::$direction::Upgrade<NegotiatedSubstream>>::Future,)*); // collected future

        impl upgrade::$direction::Upgrade<NegotiatedSubstream> for UpgradeSelect{
            type Output = SubstreamSelect;
            type Future = FutureSelect;
            type Error = UpgradeErrorSelect;

            fn $impl_name(self, sock: NegotiatedSubstream, info: UpgradeInfoSelect) -> Self::Future {
                match info {
                    $(UpgradeInfoSelect::$behaviour(info)=>Self::Future::$behaviour(match self{
                        Self::$behaviour(inner)=> inner.$impl_name(sock,info),
                        _ => panic!("upgrade info and upgrade mismatch!")
                    }),)*
                }
            }
        }
    }
    };
}

#[macro_export]
macro_rules! generate_future_select {
    ($($upgrade:ty|$behaviour:ident:$future:ty,)*) => {
        use std::pin::Pin;
        use futures::Future;
        use std::task::{Poll,Context};

        generate_select_enum!(PinSelect<'a>{$($behaviour:Pin<&'a mut $future>,)*});
        generate_select_enum!(FutureSelect{$($behaviour:$future,)*});


        impl FutureSelect{
            pub fn as_pin_mut(self:Pin<&mut Self>)-> PinSelect{
                unsafe{
                    match *Pin::get_unchecked_mut(self){
                        $(
                            FutureSelect::$behaviour(ref mut inner) => PinSelect::$behaviour(Pin::new_unchecked(inner)),
                        )*
                    }
                }
            }
        }

        impl Future for FutureSelect{
            type Output = Result<SubstreamSelect,UpgradeErrorSelect>;

            fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                match self.as_pin_mut() {
                    $(
                        PinSelect::$behaviour(inner) => match inner.poll(cx){
                            Poll::Pending => Poll::Pending,
                            Poll::Ready(inner) => Poll::Ready(match inner{
                                Ok(stream) => Ok(SubstreamSelect::$behaviour(stream)),
                                Err(upg_err)=> Err(UpgradeErrorSelect::$behaviour(upg_err))
                            })
                        }
                    )*
                }
            }
        }
    };
}

#[macro_export]
macro_rules! generate_outbound_transpose {
    ($($name:ident=>$behaviour:ident:$handler:ty,)*) => {
        use libp2p::swarm::handler::FullyNegotiatedOutbound;
        generate_select_enum!(FullyNegotiatedOutboundSelect{$($behaviour:FullyNegotiatedOutbound<<$handler as ConnectionHandler>::OutboundProtocol,<$handler as ConnectionHandler>::OutboundOpenInfo>,)*});
        fn transpose_full_outbound(outbound:FullyNegotiatedOutbound<outbound::UpgradeSelect,OutboundOpenInfoSelect>)->FullyNegotiatedOutboundSelect{
            match outbound{
                $(
                    FullyNegotiatedOutbound{
                        protocol: outbound::SubstreamSelect::$behaviour(protocol),
                        info: OutboundOpenInfoSelect::$behaviour(info)
                    } => FullyNegotiatedOutboundSelect::$behaviour(FullyNegotiatedOutbound{protocol, info}),
                )*
                _=>panic!("protocol mismatch!")
            }
        }
    }
}

#[macro_export]
macro_rules! generate_inbound_transpose {
    ($($name:ident=>$behaviour:ident:$handler:ty,)*) => {
        use libp2p::swarm::handler::FullyNegotiatedInbound;
        generate_select_enum!(FullyNegotiatedInboundSelect{$($behaviour:FullyNegotiatedInbound<<$handler as ConnectionHandler>::InboundProtocol,<$handler as ConnectionHandler>::InboundOpenInfo>,)*});
        fn transpose_full_inbound(inbound:FullyNegotiatedInbound<inbound::UpgradeSelect,InboundOpenInfoSelect>)->FullyNegotiatedInboundSelect{
            match inbound{
                $(
                    FullyNegotiatedInbound{
                        protocol: inbound::SubstreamSelect::$behaviour(protocol),
                        info: info,
                    } => FullyNegotiatedInboundSelect::$behaviour(FullyNegotiatedInbound{protocol, info:info.$name}),
                )*
            }
        }
    }
}

#[macro_export]
macro_rules! generate_upgr_error_transpose {
    ($($name:ident=>$behaviour:ident:$handler:ty,)*) => {
        use libp2p::swarm::handler::{DialUpgradeError};
        generate_select_enum!(DialUpgradeErrorSelect{$($behaviour:DialUpgradeError<<$handler as ConnectionHandler>::OutboundOpenInfo,<$handler as ConnectionHandler>::OutboundProtocol>,)*});
        fn transpose_upgr_error(error:DialUpgradeError<OutboundOpenInfoSelect,outbound::UpgradeSelect>)->DialUpgradeErrorSelect{
            match error{
                $(
                    DialUpgradeError{
                        error: StreamUpgradeError::Apply(outbound::UpgradeErrorSelect::$behaviour(error)),
                        info: OutboundOpenInfoSelect::$behaviour(info),
                    } => DialUpgradeErrorSelect::$behaviour(DialUpgradeError{info,error: StreamUpgradeError::Apply(error)}),
                )*
                $(
                    DialUpgradeError{
                        error: e,
                        info: OutboundOpenInfoSelect::$behaviour(info),
                    } => DialUpgradeErrorSelect::$behaviour(DialUpgradeError{info,error:e.map_upgrade_err(|_|panic!("already handled above"))}),
                )*
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
        generate_select_enum!(#[derive(Debug)]$name$(<$life>)?{$($field:$inner,)*});
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
