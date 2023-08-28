#[macro_export]
macro_rules! connection_handler_select {
    {$(use $path:path;)*$($name:ident=>$behaviour:ident:$handler:ty,)*} => {
        
        pub mod handler{
            use libp2p::swarm::ConnectionHandler;
            use super::*;
        pub struct ConnectionHandlerSelect{$(pub $name:$handler,)*}
        impl ConnectionHandlerSelect{
            pub fn into_inner(self)->($($handler,)*){
                ($(self.$name,)*)
            }
        }

        ::owlnest_macro::generate_event_select!(#[derive(Debug)]FromBehaviourSelect{$($behaviour:<$handler as ::libp2p::swarm::ConnectionHandler>::FromBehaviour,)*});
        ::owlnest_macro::generate_event_select!(#[derive(Debug)]ToBehaviourSelect{$($behaviour:<$handler as ::libp2p::swarm::ConnectionHandler>::ToBehaviour,)*});
        ::owlnest_macro::generate_error_select_enum!(#[derive(Debug,Clone)]HandlerErrorSelect{$($behaviour:<$handler as ::libp2p::swarm::ConnectionHandler>::Error,)*});
        ::owlnest_macro::generate_inbound_upgrade_select!($($name=>$behaviour:<$handler as ::libp2p::swarm::ConnectionHandler>::InboundProtocol,)*);
        ::owlnest_macro::generate_outbound_upgrade_select!($($name=>$behaviour:<$handler as ::libp2p::swarm::ConnectionHandler>::OutboundProtocol,)*);
        ::owlnest_macro::generate_select_enum!(OutboundOpenInfoSelect{$($behaviour:<$handler as ::libp2p::swarm::ConnectionHandler>::OutboundOpenInfo,)*});
        ::owlnest_macro::generate_select_struct!(InboundOpenInfoSelect{$($name:<$handler as ::libp2p::swarm::ConnectionHandler>::InboundOpenInfo,)*});
        ::owlnest_macro::generate_outbound_transpose!($($name=>$behaviour:$handler,)*);
        ::owlnest_macro::generate_inbound_transpose!($($name=>$behaviour:$handler,)*);
        ::owlnest_macro::generate_upgr_error_transpose!($($name=>$behaviour:$handler,)*);

        impl ConnectionHandlerSelect{
            fn on_listen_upgrade_error(
                &mut self,
                ::libp2p::swarm::handler::ListenUpgradeError {
                    info,
                    error,
                }: ::libp2p::swarm::handler::ListenUpgradeError<
                    <Self as ::libp2p::swarm::ConnectionHandler>::InboundOpenInfo,
                    <Self as ::libp2p::swarm::ConnectionHandler>::InboundProtocol,
                >,
            ) {
                match error {
                    $(inbound_upgrade_select::UpgradeErrorSelect::$behaviour(error)=> 
                        self.$name.on_connection_event(
                            ::libp2p::swarm::handler::ConnectionEvent::ListenUpgradeError(
                                ::libp2p::swarm::handler::ListenUpgradeError{
                                    info:info.$name,
                                    error}
                                )
                            )
                    ,)*
                }
            }
        }

        impl libp2p::swarm::ConnectionHandler for ConnectionHandlerSelect{
            type FromBehaviour = FromBehaviourSelect;

            type ToBehaviour = ToBehaviourSelect;

            type Error = HandlerErrorSelect;

            type InboundProtocol = inbound_upgrade_select::UpgradeSelect;

            type OutboundProtocol = outbound_upgrade_select::UpgradeSelect;

            type InboundOpenInfo = InboundOpenInfoSelect;

            type OutboundOpenInfo = OutboundOpenInfoSelect;

            fn listen_protocol(&self) -> ::libp2p::swarm::SubstreamProtocol<Self::InboundProtocol, Self::InboundOpenInfo> {
                $(let $name = self.$name.listen_protocol().into_upgrade();)*
                generate_substream_protocol($($name,)*)
            }
            fn connection_keep_alive(&self) -> ::libp2p::swarm::KeepAlive {
                vec![$(self.$name.connection_keep_alive(),)*].iter().max().unwrap_or(&::libp2p::swarm::KeepAlive::Yes).clone()
            }

            fn poll(
                &mut self,
                cx: &mut std::task::Context<'_>,
            ) -> std::task::Poll<
                ::libp2p::swarm::ConnectionHandlerEvent<
                    Self::OutboundProtocol,
                    Self::OutboundOpenInfo,
                    Self::ToBehaviour,
                    Self::Error,
                >,
            > {
                use ::std::task::Poll;
                use ::libp2p::swarm::ConnectionHandlerEvent;
                $(
                    match self.$name.poll(cx) {
                    Poll::Ready(ConnectionHandlerEvent::NotifyBehaviour(event)) => {
                        return Poll::Ready(ConnectionHandlerEvent::NotifyBehaviour(ToBehaviourSelect::$behaviour(event)));
                    }
                    Poll::Ready(ConnectionHandlerEvent::Close(event)) => {
                        return Poll::Ready(ConnectionHandlerEvent::Close(HandlerErrorSelect::$behaviour(event)));
                    }
                    Poll::Ready(ConnectionHandlerEvent::OutboundSubstreamRequest { protocol }) => {
                        return Poll::Ready(ConnectionHandlerEvent::OutboundSubstreamRequest {
                            protocol: protocol
                                .map_upgrade(|u| outbound_upgrade_select::UpgradeSelect::$behaviour(u))
                                .map_info(|i|OutboundOpenInfoSelect::$behaviour(i))
                        });
                    }
                    Poll::Pending => (),
                    _ => ()
                };)*
                Poll::Pending
            }

            fn on_behaviour_event(&mut self, event: Self::FromBehaviour) {
                match event{
                    $(FromBehaviourSelect::$behaviour(ev)=>self.$name.on_behaviour_event(ev),)*
                }
            }

            fn on_connection_event(
                &mut self,
                event: ::libp2p::swarm::handler::ConnectionEvent<
                    Self::InboundProtocol,
                    Self::OutboundProtocol,
                    Self::InboundOpenInfo,
                    Self::OutboundOpenInfo,
                >,
            ) {
                use ::libp2p::swarm::handler::ConnectionEvent;
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
        fn generate_substream_protocol(
            $($name:(<$handler as ::libp2p::swarm::ConnectionHandler>::InboundProtocol,
            <$handler as ::libp2p::swarm::ConnectionHandler>::InboundOpenInfo),)*
        )->::libp2p::swarm::SubstreamProtocol<inbound_upgrade_select::UpgradeSelect,<ConnectionHandlerSelect as ::libp2p::swarm::ConnectionHandler>::InboundOpenInfo>
        {
            ::libp2p::swarm::SubstreamProtocol::new(inbound_upgrade_select::UpgradeSelect{$($name:$name.0,)*},InboundOpenInfoSelect{$($name:$name.1,)*})
        }
        }
}
}

#[macro_export]
macro_rules! generate_inbound_upgrade_select {
    ($($name:ident=>$behaviour:ident:$upgrade:ty,)*) => {
        pub(crate) mod inbound_upgrade_select{
        use super::*;
        ::owlnest_macro::generate_select_enum!(#[derive(Clone)]UpgradeInfoSelect{$($behaviour:<$upgrade as ::libp2p::core::UpgradeInfo>::Info,)*});
        impl AsRef<str> for UpgradeInfoSelect{
            fn as_ref(&self)->&str{
                match self{
                    $(Self::$behaviour(inner)=>AsRef::<str>::as_ref(inner),)*
                }
            }
        }
        ::owlnest_macro::generate_select_struct!(
            #[derive(Clone)]
            UpgradeInfoIterSelect{
                $($name:Option<std::iter::Map<<<$upgrade as ::libp2p::core::UpgradeInfo>::InfoIter as IntoIterator>::IntoIter, fn(<$upgrade as ::libp2p::core::UpgradeInfo>::Info) -> UpgradeInfoSelect>>,)*
            }
        );
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

        ::owlnest_macro::generate_select_struct!(
            UpgradeSelect{$($name:$upgrade,)*}
        );

        ::owlnest_macro::generate_select_enum!(UpgradeErrorSelect{$($behaviour:<$upgrade as ::libp2p::core::upgrade::InboundUpgrade<::libp2p::Stream>>::Error,)*});

        impl ::libp2p::core::UpgradeInfo for UpgradeSelect{
            type Info = UpgradeInfoSelect;
            type InfoIter = UpgradeInfoIterSelect;

            fn protocol_info(&self) -> Self::InfoIter {
                UpgradeInfoIterSelect{
                    $($name:Some(self.$name.protocol_info().into_iter().map(UpgradeInfoSelect::$behaviour as fn(<$upgrade as ::libp2p::core::UpgradeInfo>::Info)->_)),)*
                }
            }
        }

        ::owlnest_macro::generate_select_enum!(SubstreamSelect{$($behaviour:<$upgrade as ::libp2p::core::upgrade::InboundUpgrade<::libp2p::Stream>>::Output,)*});
        ::owlnest_macro::generate_future_select!($($upgrade|$behaviour:<$upgrade as ::libp2p::core::upgrade::InboundUpgrade<::libp2p::Stream>>::Future,)*); // collected future

        impl ::libp2p::core::upgrade::InboundUpgrade<::libp2p::swarm::Stream> for UpgradeSelect{
            type Output = SubstreamSelect;
            type Future = FutureSelect;
            type Error = UpgradeErrorSelect;

            fn upgrade_inbound(self, sock: ::libp2p::Stream, info: UpgradeInfoSelect) -> Self::Future {
                match info {
                    $(UpgradeInfoSelect::$behaviour(info)=>Self::Future::$behaviour(self.$name.upgrade_inbound(sock,info)),)*
                }
            }
        }
    }
    };
}

#[macro_export]
macro_rules! generate_outbound_upgrade_select {
    ($($name:ident=>$behaviour:ident:$upgrade:ty,)*) => {
        pub(crate) mod outbound_upgrade_select{
            use libp2p::swarm::Stream;
            use super::*;
        ::owlnest_macro::generate_select_enum!(#[derive(Clone)]UpgradeInfoSelect{$($behaviour:<$upgrade as ::libp2p::core::UpgradeInfo>::Info,)*});
        impl AsRef<str> for UpgradeInfoSelect{
            fn as_ref(&self)->&str{
                match self{
                    $(Self::$behaviour(inner)=>AsRef::<str>::as_ref(inner),)*
                }
            }
        }
        ::owlnest_macro::generate_select_enum!(UpgradeSelect{$($behaviour:$upgrade,)*});
        ::owlnest_macro::generate_select_enum!(UpgradeErrorSelect{$($behaviour:<$upgrade as ::libp2p::core::upgrade::OutboundUpgrade<Stream>>::Error,)*});

        impl ::libp2p::core::UpgradeInfo for UpgradeSelect{
            type Info = UpgradeInfoSelect;
            type InfoIter = core::iter::Once<UpgradeInfoSelect>;

            fn protocol_info(&self) -> Self::InfoIter {
                match self{
                    $(Self::$behaviour(inner)=>core::iter::once(UpgradeInfoSelect::$behaviour(inner.protocol_info().next().unwrap())),)*
                }
            }
        }

        ::owlnest_macro::generate_select_enum!(SubstreamSelect{$($behaviour:<$upgrade as ::libp2p::core::upgrade::OutboundUpgrade<Stream>>::Output,)*});
        ::owlnest_macro::generate_future_select!($($upgrade|$behaviour:<$upgrade as ::libp2p::core::upgrade::OutboundUpgrade<Stream>>::Future,)*); // collected future

        impl ::libp2p::core::upgrade::OutboundUpgrade<Stream> for UpgradeSelect{
            type Output = SubstreamSelect;
            type Future = FutureSelect;
            type Error = UpgradeErrorSelect;

            fn upgrade_outbound(self, sock: Stream, info: UpgradeInfoSelect) -> Self::Future {
                match info {
                    $(UpgradeInfoSelect::$behaviour(info)=>Self::Future::$behaviour(match self{
                        Self::$behaviour(inner)=> inner.upgrade_outbound(sock,info),
                        #[allow(unreachable_patterns)]
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
        ::owlnest_macro::generate_select_enum!(PinSelect<'a>{$($behaviour: ::std::pin::Pin<&'a mut $future>,)*});
        ::owlnest_macro::generate_select_enum!(FutureSelect{$($behaviour:$future,)*});

        impl FutureSelect{
            pub fn as_pin_mut(self: ::std::pin::Pin<&mut Self>)-> PinSelect{ 
                unsafe{
                    match *::std::pin::Pin::get_unchecked_mut(self){
                        $(
                            FutureSelect::$behaviour(ref mut inner) => PinSelect::$behaviour(::std::pin::Pin::new_unchecked(inner)),
                        )*
                    }
                }
            }
        }

        impl ::futures::Future for FutureSelect{
            type Output = Result<SubstreamSelect,UpgradeErrorSelect>;

            fn poll(self: ::std::pin::Pin<&mut Self>, cx: &mut ::std::task::Context<'_>) -> std::task::Poll<Self::Output> {
                use ::std::task::Poll;
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
        ::owlnest_macro::generate_select_enum!(FullyNegotiatedOutboundSelect{$($behaviour:FullyNegotiatedOutbound<<$handler as ::libp2p::swarm::ConnectionHandler>::OutboundProtocol,<$handler as ::libp2p::swarm::ConnectionHandler>::OutboundOpenInfo>,)*});
        fn transpose_full_outbound(outbound:FullyNegotiatedOutbound<outbound_upgrade_select::UpgradeSelect,OutboundOpenInfoSelect>)->FullyNegotiatedOutboundSelect{
            match outbound{
                $(
                    FullyNegotiatedOutbound{
                        protocol: outbound_upgrade_select::SubstreamSelect::$behaviour(protocol),
                        info: OutboundOpenInfoSelect::$behaviour(info)
                    } => FullyNegotiatedOutboundSelect::$behaviour(FullyNegotiatedOutbound{protocol, info}),
                )*
                #[allow(unreachable_patterns)]
                _=>panic!("protocol mismatch!")
            }
        }
    }
}

#[macro_export]
macro_rules! generate_inbound_transpose {
    ($($name:ident=>$behaviour:ident:$handler:ty,)*) => {
        use libp2p::swarm::handler::FullyNegotiatedInbound;
        ::owlnest_macro::generate_select_enum!(FullyNegotiatedInboundSelect{$($behaviour:FullyNegotiatedInbound<<$handler as ::libp2p::swarm::ConnectionHandler>::InboundProtocol,<$handler as ::libp2p::swarm::ConnectionHandler>::InboundOpenInfo>,)*});
        fn transpose_full_inbound(inbound:FullyNegotiatedInbound<inbound_upgrade_select::UpgradeSelect,InboundOpenInfoSelect>)->FullyNegotiatedInboundSelect{
            match inbound{
                $(
                    FullyNegotiatedInbound{
                        protocol: inbound_upgrade_select::SubstreamSelect::$behaviour(protocol),
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
        use libp2p::swarm::handler::{DialUpgradeError,StreamUpgradeError};
        ::owlnest_macro::generate_select_enum!(DialUpgradeErrorSelect{$($behaviour:DialUpgradeError<<$handler as ::libp2p::swarm::ConnectionHandler>::OutboundOpenInfo,<$handler as ::libp2p::swarm::ConnectionHandler>::OutboundProtocol>,)*});
        fn transpose_upgr_error(error:DialUpgradeError<OutboundOpenInfoSelect,outbound_upgrade_select::UpgradeSelect>)->DialUpgradeErrorSelect{
            match error{
                $(
                    DialUpgradeError{
                        error: StreamUpgradeError::Apply(outbound_upgrade_select::UpgradeErrorSelect::$behaviour(error)),
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
