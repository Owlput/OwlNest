
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