#[macro_export]
macro_rules! generate_in_event_select {
    ($($behaviour:ident:$ev:ty),+) => {
        pub enum InEventSelect{
            $($behaviour($ev),)*
        }
        $(impl From<$ev> for InEventSelect{
            fn from(value:$ev)->InEventSelect{
                InEventSelect::$behaviour(value)
            }
        })*
        $(impl Into<$ev> for InEventSelect{
            fn into(self)->$ev{
                match self{
                    InEventSelect::$behaviour(ev)=>ev,
                    _=>panic!("Internal conversion error")
                }
            }
        })*
    };
}
#[macro_export]
macro_rules! generate_out_event_select {
    ($($behaviour:ident:$ev:ty),+) => {
        pub enum OutEventSelect{
            $($behaviour($ev),)*
        }
        $(impl From<$ev> for OutEventSelect{
            fn from(value:$ev)->OutEventSelect{
                OutEventSelect::$behaviour(value)
            }
        })*
        $(impl Into<$ev> for OutEventSelect{
            fn into(self)->$ev{
                match self{
                    OutEventSelect::$behaviour(ev)=>ev,
                    _=>panic!("Internal conversion error")
                }
            }
        })*
    };
}

#[macro_export]
macro_rules! generate_connection_handler_select {
    ($($handler:ty),+) => {
        pub struct ConnectionHandlerSelect($($behaviour:$handler,)*);

        impl libp2p::swarm::ConnectionHandler for ConnectionHandlerSelect{
            type InEvent = InEventSelect;

            type OutEvent = OutEventSelect;

            type Error = ErrorSelect;

            type InboundProtocol = InboundUpgradeSelect;

            type OutboundProtocol = OutboundUpgradeSelect;

            type InboundOpenInfo = InboundOpenInfoSelect;

            type OutboundOpenInfo = OutboundOpenInfoSelect;

            fn listen_protocol(&self) -> libp2p::swarm::SubstreamProtocol<Self::InboundProtocol, Self::InboundOpenInfo> {
                $(let $handler)*
            }

            fn connection_keep_alive(&self) -> libp2p::swarm::KeepAlive {
                todo!()
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
                todo!()
            }

            fn on_behaviour_event(&mut self, _event: Self::InEvent) {
                todo!()
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
                todo!()
            }
        }
    };
}

#[macro_export]
macro_rules! generate_inbound_upgrade_select {
    ($($name:ident=>$behaviour:ident:$upgrade:ty),+) => {
        use crate::*;
        use libp2p::InboundUpgrade;
        use libp2p::swarm::NegotiatedSubstream;
        use libp2p::core::upgrade;

        generate_select_enum!(#[derive(Clone)]InboundUpgradeInfoSelect{$($behaviour:<<$upgrade as ConnectionHandler>::InboundProtocol as upgrade::UpgradeInfo>::Info)*});
        impl AsRef<str> for InboundUpgradeInfoSelect{
            fn as_ref(&self)->&str{
                match self{
                    $(Self::$behaviour(inner)=>inner)*
                }
            }
        }

        generate_select_struct!(
            InboundUpgradeSelect{$($name:<$upgrade as ConnectionHandler>::InboundProtocol)*}
        );

        generate_select_enum!(InboundUpgradeErrorSelect{$($behaviour:<<$upgrade as ConnectionHandler>::InboundProtocol as upgrade::InboundUpgrade<NegotiatedSubstream>>::Error)*});

        impl upgrade::UpgradeInfo for InboundUpgradeSelect{
            type Info = InboundUpgradeInfoSelect;
            type InfoIter = core::iter::Once<Self::Info>;
        
            fn protocol_info(&self) -> Self::InfoIter {
                match self{
                    $(Self::$behaviour(info)=>info.protocol_info())*
                }
            }
        }

        generate_future_select!($(inbound=>$behaviour:<<$upgrade as ConnectionHandler>::InboundProtocol as upgrade::InboundUpgrade<NegotiatedSubstream>>::Future)*); // collected future

        impl InboundUpgrade<NegotiatedSubstream> for InboundUpgradeSelect{
            type Output = NegotiatedSubstream;
            type Future = inbound::FutureSelect; // future needed to be resolved in this negotiation
            type Error = InboundUpgradeErrorSelect;

            fn upgrade_inbound(self, sock: NegotiatedSubstream, info: InboundUpgradeInfoSelect) -> Self::Future {
                match info {
                    $(InboundUpgradeInfoSelect::$behaviour(info)=>Self::Future::$behaviour(self.$name.upgrade_inbound(sock,info)))*
                }
            }
        }
    };
}

#[macro_export]
macro_rules! generate_future_select {
    ($name:ident=>$($behaviour:ident:$future:ty),+) => {
        pub(crate) mod $name{
        use crate::*;
        use super::*;
        use std::pin::Pin;
        use futures::Future;
        use std::task::{Poll,Context};
        use libp2p::core::upgrade::Negotiated;
        use libp2p::core::muxing::SubstreamBox;

        generate_select_enum!(PinSelect<'a>{$($behaviour:Pin<&'a mut $future>),+});
        generate_select_enum!(FutureSelect{$($behaviour:$future),+});
        generate_select_enum!(OutputSelect{$($behaviour:<$future as Future>::Output),+});

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
            type Output = OutputSelect;

            fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                match self.as_pin_mut() {
                    $(
                        PinSelect::$behaviour(inner) => match inner.poll(cx){
                            Poll::Pending => Poll::Pending,
                            Poll::Ready(inner) => Poll::Ready(OutputSelect::$behaviour(inner))
                        }
                    )*
                }
            }
        }
    }
    };
}

#[macro_export]
macro_rules! generate_select_enum {
    ($(#[$meta:meta])*$name:ident$(<$life:lifetime>)?{$($identifier:ident:$inner:ty),+}) => {
        $(#[$meta])*
        pub enum $name$(<$life>)?{
            $($identifier($inner),)*
        }
    };
}

#[macro_export]
macro_rules! generate_select_struct {
    ($(#[$meta:meta])*$name:ident$(<$life:lifetime>)?{$($field:ident:$inner:ty),+}) => {
        $(#[$meta])*
        pub struct $name$(<$life>)?{
            $($field:$inner,)*
        }
    };
}



