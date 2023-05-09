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
macro_rules! connection_handler_select {
    {$($name:ident=>$behaviour:ident:$handler:ty,)*} => {
        use crate::*;
        use libp2p::swarm::{NetworkBehaviour,ConnectionHandler};

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
        generate_event_select!(FromBehaviourSelect{$($behaviour:<$handler as ConnectionHandler>::InEvent,)*});
        generate_event_select!(ToBehaviourSelect{$($behaviour:<$handler as ConnectionHandler>::OutEvent,)*});
        generate_error_select_enum!(HandlerErrorSelect{$($behaviour:<$handler as ConnectionHandler>::Error,)*});
        generate_upgrade_select!(inbound,upgrade_inbound{$($name=>$behaviour:<$handler as ConnectionHandler>::InboundProtocol,)*});
        generate_upgrade_select!(outbound,upgrade_outbound{$($name=>$behaviour:<$handler as ConnectionHandler>::OutboundProtocol,)*});

        impl libp2p::swarm::ConnectionHandler for ConnectionHandlerSelect{
            type InEvent = FromBehaviourSelect;

            type OutEvent = ToBehaviourSelect;

            type Error = HandlerErrorSelect;

            type InboundProtocol = inbound::UpgradeSelect;

            type OutboundProtocol = outbound::UpgradeSelect;

            type InboundOpenInfo = inbound::UpgradeInfoSelect;

            type OutboundOpenInfo = outbound::UpgradeInfoSelect;

            fn listen_protocol(&self) -> libp2p::swarm::SubstreamProtocol<Self::InboundProtocol, Self::InboundOpenInfo> {
                todo!()
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
macro_rules! generate_upgrade_select {
    ($direction:ident,$impl_name:ident{$($name:ident=>$behaviour:ident:$upgrade:ty,)*}) => {
        pub(crate) mod $direction{
            use crate::*;
            use super::*;
            use libp2p::swarm::NegotiatedSubstream;
        generate_select_enum!(#[derive(Clone)]UpgradeInfoSelect{$($behaviour:<$upgrade as upgrade::UpgradeInfo>::Info,)*});
        impl AsRef<str> for UpgradeInfoSelect{
            fn as_ref(&self)->&str{
                match self{
                    $(Self::$behaviour(inner)=>inner,)*
                }
            }
        }
        generate_select_struct!(
            UpgradeSelect{$($name:$upgrade,)*}
        );

        generate_select_enum!(UpgradeErrorSelect{$($behaviour:<$upgrade as upgrade::$direction::Upgrade<NegotiatedSubstream>>::Error,)*});

        impl upgrade::UpgradeInfo for UpgradeSelect{
            type Info = UpgradeInfoSelect;
            type InfoIter = Vec<UpgradeInfoSelect>;

            fn protocol_info(&self) -> Self::InfoIter {
                vec![$(UpgradeInfoSelect::$behaviour(self.$name.protocol_info().next().unwrap()),)*]      
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
            $($field:$inner,)*
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
