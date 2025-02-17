pub mod behaviour_select;
pub mod connection_handler_select;

pub mod utils {

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
    // entry
    ($($(#[$metas:meta])*$variant:ident:$name:ident($($tokens:tt)*)$(->$return_type:ty)*;)+) => {
        $(generate_handler_method!(()($($tokens)*,)($(#[$metas])*$variant:$name)()()()($($return_type)*));)+
    };    

    // clone parameter continue
    (($($param_order:tt)*)($param_name:ident:<$param_type:ty>,$($tokens:tt)*)($($metadata:tt)+)($($copy_params:tt)*)($($clone_params:tt)*)($($owned_params:tt)*)($($return_type:tt)*)) => {
        generate_handler_method!(($($param_order)*$param_name:$param_type,)($($tokens)*)($($metadata)*)($($copy_params)*)($param_name:$param_type,$($clone_params)*)($($owned_params)*)($($return_type)*));
    };

    // copy parameter continue
    (($($param_order:tt)*)($param_name:ident:$param_type:ty,$($tokens:tt)*)($($metadata:tt)+)($($copy_params:tt)*)($($clone_params:tt)*)($($owned_params:tt)*)($($return_type:tt)*)) => {
        generate_handler_method!(($($param_order)*$param_name:$param_type,)($($tokens)*)($($metadata)*)($param_name:$param_type,$($copy_params)*)($($clone_params)*)($($owned_params)*)($($return_type)*));
    };
    // own parameter continue
    (($($param_order:tt)*)($param_name:ident:|$param_type:ty|,$($tokens:tt)*)($($metadata:tt)+)($($copy_params:tt)*)($($clone_params:tt)*)($($owned_params:tt)*)($($return_type:tt)*)) => {
        generate_handler_method!(($($param_order)*$param_name:$param_type,)($($tokens)*)($($metadata)*)($($copy_params)*)($($clone_params)*)($param_name:$param_type,$($owned_params)*)($($return_type)*));
    };
    // exit
    (($($param_order:tt)*)($(,)*)($($metadata:tt)+)($($copy_params:tt)*)($($clone_params:tt)*)($($owned_params:tt)*)($($return_type:tt)*)) => {
        generate_handler_method!(($($param_order)*)($($metadata)+)($($copy_params)*)($($clone_params)*)($($owned_params)*)($($return_type)*));
    };    
    (($($param_order:tt)*)($(#[$metas:meta])*$variant:ident:$name:ident)($($copy_param_name:ident:$copy_param_type:ty,)*)($($clone_param_name:ident:$clone_param_type:ty,)*)($($owned_param_name:ident:$owned_param_type:ty,)*)($return_type:ty))=>{
        $(#[$metas])*
        pub async fn $name(&self,$($param_order)*)->$return_type{
            let (tx, rx) = tokio::sync::oneshot::channel();
            let ev = InEvent::$variant{
                callback:tx,
                $($copy_param_name:*$copy_param_name,)*
                $($clone_param_name:$clone_param_name.clone(),)*
                $($owned_param_name,)*
            };
            self.sender.send(ev).await.expect(owlnest_core::expect::SWARM_RECEIVER_KEPT_ALIVE);
            rx.await.expect(owlnest_core::expect::CALLBACK_CLEAR)
        }
    };
    (($($param_order:tt)*)($(#[$metas:meta])*$variant:ident:$name:ident)($($copy_param_name:ident:$copy_param_type:ty,)*)($($clone_param_name:ident:$clone_param_type:ty,)*)($($owned_param_name:ident:$owned_param_type:ty,)*)())=>{
        $(#[$metas])*
        pub async fn $name(&self,$($param_order)*){
            let ev = InEvent::$variant{
                $($copy_param_name:*$copy_param_name,)*
                $($clone_param_name:$clone_param_name.clone(),)*
                $($owned_param_name,)*
            };
            self.sender.send(ev).await.expect(owlnest_core::expect::SWARM_RECEIVER_KEPT_ALIVE)
        }
    };

}


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
// entry
($($(#[$metas:meta])*$variant:ident:$name:ident($($tokens:tt)*)$(->$return_type:ty)?;)+) => {
    $(generate_handler_method_blocking!(()($($tokens)*,)($(#[$metas])*$variant:$name)()()()$($return_type)?);)+
};
// clone parameter continue
(($($param_order:tt)*)($param_name:ident:<$param_type:ty>,$($tokens:tt)*)($($metadata:tt)+)($($copy_params:tt)*)($($clone_params:tt)*)($($owned_params:tt)*)$($return_type:ty)?) => {
    generate_handler_method_blocking!(($($param_order)*$param_name:$param_type,)($($tokens)*)($($metadata)*)($($copy_params)*)($param_name:$param_type,$($clone_params)*)($($owned_params)*)$($return_type)?);
};
// own parameter continue
(($($param_order:tt)*)($param_name:ident:|$param_type:ty|,$($tokens:tt)*)($($metadata:tt)+)($($copy_params:tt)*)($($clone_params:tt)*)($($owned_params:tt)*)$($return_type:ty)?) => {
    generate_handler_method_blocking!(($($param_order)*$param_name:$param_type,)($($tokens)*)($($metadata)*)($($copy_params)*)($($clone_params)*)($param_name:$param_type,$($owned_params)*)$($return_type)?);
};
// copy parameter continue
(($($param_order:tt)*)($param_name:ident:$param_type:ty,$($tokens:tt)*)($($metadata:tt)+)($($copy_params:tt)*)($($clone_params:tt)*)($($owned_params:tt)*)$($return_type:ty)?) => {
    generate_handler_method_blocking!(($($param_order)*$param_name:$param_type,)($($tokens)*)($($metadata)*)($param_name:$param_type,$($copy_params)*)($($clone_params)*)($($owned_params)*)$($return_type)?);
};

// exit
(($($param_order:tt)*)($(,)*)($($metadata:tt)+)($($copy_params:tt)*)($($clone_params:tt)*)($($owned_params:tt)*)$($return_type:ty)?) => {
    generate_handler_method_blocking!(($($param_order)*)($($metadata)+)($($copy_params)*)($($clone_params)*)($($owned_params)*)$($return_type)?);
};
(($($param_order:tt)*)($(#[$metas:meta])*$variant:ident:$name:ident)($($copy_param_name:ident:$copy_param_type:ty,)*)($($clone_param_name:ident:$clone_param_type:ty,)*)($($owned_param_name:ident:$owned_param_type:ty,)*))=>{
    $(#[$metas])*
    pub fn $name(&self,$($param_order)*){
        let ev = InEvent::$variant{
            $($copy_param_name:*$copy_param_name,)*
            $($clone_param_name:$clone_param_name.clone(),)*
            $($owned_param_name,)*
        };
        self.sender.blocking_send(ev).expect(owlnest_core::expect::SWARM_RECEIVER_KEPT_ALIVE)
    }
};
(($($param_order:tt)*)($(#[$metas:meta])*$variant:ident:$name:ident)($($copy_param_name:ident:$copy_param_type:ty,)*)($($clone_param_name:ident:$clone_param_type:ty,)*)($($owned_param_name:ident:$owned_param_type:ty)*)$return_type:ty)=>{
    $(#[$metas])*
    pub fn $name(&self,$($param_order)*)->$return_type{
        let (tx, rx) = tokio::sync::oneshot::channel();
        let ev = InEvent::$variant{
            callback:tx,
            $($copy_param_name:*$copy_param_name,)*
            $($clone_param_name:$clone_param_name.clone(),)*
            $($owned_param_name,)*
        };
        self.sender.blocking_send(ev).expect(owlnest_core::expect::SWARM_RECEIVER_KEPT_ALIVE);
        rx.blocking_recv().expect(owlnest_core::expect::CALLBACK_CLEAR)
    }
}
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
