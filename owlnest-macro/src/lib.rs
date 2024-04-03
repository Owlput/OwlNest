pub mod behaviour_select;
pub mod connection_handler_select;

pub mod utils {
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
    {$($variant:ident:$name:ident($($params:ident:$param_type:ty)*)->$return_type:ty;)+} => {
        $(pub fn $name(&self,$($params:$param_type)*)->$return_type{
            use tokio::sync::oneshot::*;
            let (tx,rx) = channel();
            let ev = InEvent::$variant($($params,)*tx);
            self.sender.blocking_send(ev).unwrap();
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
    {$($(#[$metas:meta])*$variant:ident:$name:ident($($params:ident:$param_type:ty$(,)?)*)->();)+} => {
        $(
            $(#[$metas])*
            pub async fn $name(&self,$($params:$param_type,)*){
                let ev = InEvent::$variant($($params,)*);
                self.sender.send(ev).await.unwrap();
            }
        )*
    };
    {$($(#[$metas:meta])*$variant:ident:$name:ident($($params:ident:$param_type:ty$(,)?)*)->$return_type:ty;)+} => {
        $(
            $(#[$metas])*
            pub async fn $name(&self,$($params:$param_type,)*)->$return_type{
                use tokio::sync::oneshot::*;
                let (tx,rx) = channel();
                let ev = InEvent::$variant($($params,)*tx);
                self.sender.send(ev).await.unwrap();
                rx.await.unwrap()
            }
        )*
    };
}

    #[macro_export]
    macro_rules! handle_callback_sender {
        ($message:ident=>$sender:ident) => {
            if let Err(v) = $sender.send($message) {
                tracing::warn!("Cannot send a callback message: {:?}", v)
            }
        };
        ($message:expr=>$sender:ident) => {
            if let Err(v) = $sender.send($message) {
                tracing::warn!("Cannot send a callback message: {:?}", v)
            }
        };
    }
    #[macro_export]
    macro_rules! listen_event {
        ($listener:ident for $behaviour:ident,$pattern:pat=>{$($ops:tt)+}) => {
            async move{
                while let Ok(ev) = $listener.recv().await{
                    if let SwarmEvent::Behaviour(BehaviourEvent::$behaviour($pattern)) = ev.as_ref() {
                        $($ops)+
                    }
                }
                unreachable!()
            }
        };
    }
    #[macro_export]
    macro_rules! with_timeout {
        ($future:ident,$timeout:literal) => {{
            let timer = futures_timer::Delay::new(std::time::Duration::from_secs($timeout));
            tokio::select! {
                _ = timer =>{
                    Err(())
                }
                v = $future => {
                    Ok(v)
                }
            }
        }};
    }
}
