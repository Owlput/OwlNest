
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