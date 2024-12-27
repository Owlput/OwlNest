pub mod behaviour_select;
pub mod connection_handler_select;

pub mod utils {

    pub const SWARM_RECEIVER_KEPT_ALIVE_ERROR: &str =
        "Receiver in the swarm should stay alive the entire lifetime of the app.";

    /// Less boilerplate for simple handler functions that use callback to return some data.
    ///
    /// Example:
    /// ```ignore
    /// generate_handler_method_blocking!({
    ///     EventVariant1:method_name(parameter_name:ParameterType,..)->ReturnType;
    ///     EventVariant2:method_name(*param:ParamType)->ReturnType;
    ///     ..
    /// })
    /// ```
    ///
    /// Supplied parameters must be in the same type and order with EventVariant.
    /// `&self` is automatically filled in, but EventVariant must be a tuple.   
    /// Variant can hold no data(except callback), which means leaving the function parameter blank.  
    #[macro_export]
    macro_rules! generate_handler_method_blocking {
    {$($(#[$metas:meta])*$variant:ident:$name:ident($($params:ident:$param_type:ty)*)->$return_type:ty;)+} => {
        $(
            $(#[$metas])*
            pub fn $name(&self,$($params:$param_type)*)->$return_type{
                use tokio::sync::oneshot::*;
                let (tx,rx) = channel();
                let ev = InEvent::$variant($($params,)*tx);
                self.sender.blocking_send(ev).expect(owlnest_macro::utils::SWARM_RECEIVER_KEPT_ALIVE_ERROR);
                rx.blocking_recv().unwrap()
            }
        )*
    };
}

    /// Less boilerplate for simple handler functions that use callback to return some data.
    ///
    /// Example:
    /// ``` ignore
    /// generate_handler_method!({
    ///     EventVariant1:method_name(parameter_name:ParameterType,..)->ReturnType;
    ///     ..
    /// })
    /// ```
    ///
    /// Supplied parameters must be in the same type and order with EventVariant.
    /// `&self` is automatically filled in, but EventVariant must be a tuple.   
    /// Variant can hold no data(except callback), which means leaving the function parameter blank.  
    ///  
    /// Method with callback and without callback need to be generated separately.
    #[macro_export]
    macro_rules! generate_handler_method {
    {$($(#[$metas:meta])*$variant:ident:$name:ident($($params:ident:$param_type:ty$(,)?)*);)+} => {
        $(
            $(#[$metas])*
            pub async fn $name(&self,$($params:$param_type,)*)->Result<(), crate::utils::ChannelError>{
                let ev = InEvent::$variant($($params,)*);
                Ok(self.sender.send(ev).await?)
            }
        )*
    };
    {$($(#[$metas:meta])*$variant:ident:$name:ident($($params:ident:$param_type:ty$(,)?)*)->$return_type:ty;)+} => {
        $(
            $(#[$metas])*
            pub async fn $name(&self,$($params:$param_type,)*)->Result<$return_type, crate::utils::ChannelError>{
                use tokio::sync::oneshot::*;
                let (tx,rx) = channel();
                let ev = InEvent::$variant($($params,)*tx);
                self.sender.send(ev).await?;
                Ok(rx.await?)
            }
        )*
    };
    {$($(#[$metas:meta])*$variant:ident:$name:ident({$($params:ident:$param_type:ty$(,)?)*});)+} => {
        $(
            $(#[$metas])*
            pub async fn $name(&self,$($params:$param_type,)*)->Result<(), crate::utils::ChannelError>{
                use tokio::sync::oneshot::*;
                let (tx,rx) = channel();
                let ev = InEvent::$variant{$($params,)*callback:tx};
                self.sender.send(ev).await?;
                Ok(rx.await?)
            }
        )*
    };
}

    #[macro_export]
    macro_rules! handle_callback_sender {
        // ($message:ident=>$sender:ident) => {
        //     if let Err(v) = $sender.send($message) {
        //         tracing::warn!("Cannot send a callback message: {:?}", v)
        //     }
        // };
        ($message:expr=>$sender:expr) => {
            if let Err(v) = $sender.send($message) {
                tracing::warn!("Cannot send a callback message: {:?}", v)
            }
        };
    }
    /// Short-hand for listening event on swarm.
    /// If `ops` does not contain explicit exit condition, it will listen on it forever.
    /// Best suited for one-shot event.
    #[macro_export]
    macro_rules! listen_event {
        ($listener:ident for $behaviour:ident,$($pattern:pat=>{$($ops:tt)+})+) => {
            async move{
                use crate::net::p2p::swarm::{SwarmEvent, BehaviourEvent};
                while let Ok(ev) = $listener.recv().await{
                    match ev.as_ref() {
                        $(SwarmEvent::Behaviour(BehaviourEvent::$behaviour($pattern)) => {$($ops)+})+
                        _ => {}
                    }
                }
                unreachable!()
            }
        };
    }
    #[macro_export]
    macro_rules! send_to_swam {
        ($ev:ident) => {
            self.sender.send(ev)
        };
    }
}
