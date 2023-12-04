/// Less boilerplate for simple handler functions that use callback to return some data.
///
/// Example:
/// ```no_run
/// generate_handler_method_blocking!({
///     EventVariant1:method_name(parameter_name:ParameterType,..)->ReturnType;
///     EventVariant2:method_name(*param:ParamType)->ReturnType;
///     ..
/// })
/// ```
///
/// Supported parameter must be in the same type and order with EventVariant.
/// `&self` is automatically filled in, but EventVariant must be a tuple.   
/// Variant can hold no data, leaving the function parameter blank.   
/// If ParamType is a reference and deref is need, add `*` in front of param name.
///
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

#[macro_export]
macro_rules! generate_handler_method {
    {$($variant:ident:$name:ident($($params:ident:$param_type:ty)*)->$return_type:ty;)+} => {
        $(pub async fn $name(&self,$($params:$param_type)*)->$return_type{
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
macro_rules! handle_listener_result {
    ($listener:ident) => {{
        use tokio::sync::broadcast::error::RecvError;
        use tracing::warn;
        match $listener.recv().await {
            Ok(v) => v,
            Err(e) => {
                match e {
                    RecvError::Closed => unreachable!("At least one sender should exist."),
                    RecvError::Lagged(count) => warn!(
                        "A broadcast recever is too slow! lagging {} message behind",
                        count
                    ),
                }
                continue;
            }
        }
    }};
}

#[macro_export]
macro_rules! handle_callback_sender {
    ($message:ident=>$sender:ident) => {
        if let Err(v) = $sender.send($message) {
            warn!("Cannot send a callback message: {:?}", v)
        };
    };
}
