use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::Path;

#[proc_macro_attribute]
pub fn generate_behaviour_select(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let ast: syn::DeriveInput = syn::parse(item).unwrap();
    let selective_struct = match &ast.data {
        syn::Data::Struct(data) => data,
        _ => panic!("Not applicable for types outside struct"),
    };
    let macro_fields = selective_struct.fields.iter().map(|field| {
        let ident = field.ident.as_ref().expect("Only for named field");
        let variant = syn::parse::<syn::Variant>(field.ty.to_token_stream().into()).unwrap();
        quote! {
            #ident: #variant,
        }
    });
    quote! {
        behaviour_select!{
            #(#macro_fields)*
        }
    }
    .into()
}

#[proc_macro_attribute]
pub fn with_field(attr: TokenStream, item: TokenStream) -> TokenStream {
    let ast: syn::DeriveInput = syn::parse(item).unwrap();
    let selective_struct = match &ast.data {
        syn::Data::Struct(data) => data,
        _ => panic!("Not applicable for types outside struct"),
    };
    let attributes = &ast.attrs;
    let struct_name = &ast.ident;
    let fields_to_append = syn::parse::<syn::FieldsNamed>(attr).unwrap();
    let fields_to_append_iter = fields_to_append.named.iter();
    let struct_fields = selective_struct.fields.iter();
    let vis = &ast.vis;
    quote! {
        #(#attributes)*
        #vis struct #struct_name {
            #(#struct_fields,)*
            #(#fields_to_append_iter)*,
        }
    }
    .into()
}

#[proc_macro_attribute]
pub fn generate_manager(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let ast: syn::DeriveInput = syn::parse(item).unwrap();
    let selective_struct = match &ast.data {
        syn::Data::Struct(data) => data,
        _ => panic!("Not applicable for types outside struct"),
    };
    let paths = selective_struct.fields.iter().map(|field| {
        let ident = field.ident.as_ref().expect("Only for named field");
        syn::parse::<Path>(ident.to_token_stream().into()).unwrap()
    });
    let paths1 = paths.clone();
    let paths2 = paths.clone();
    let paths3 = paths.clone();
    let paths4 = paths.clone();
    let idents = selective_struct
        .fields
        .iter()
        .map(|field| field.ident.clone().expect("Only for named field"));
    let idents1 = idents.clone();
    let idents2 = idents.clone();
    let idents3 = idents.clone();
    let idents4 = idents.clone();
    let idents5 = idents.clone();
    let idents6 = idents.clone();
    let idents7 = idents.clone();
    let idents8 = idents.clone();
    let idents9 = idents.clone();
    let idents10 = idents.clone();
    let variants = selective_struct.fields.iter().map(|field| {
        syn::parse::<syn::Variant>(field.ty.to_token_stream().into())
            .unwrap()
            .to_token_stream()
    });
    let variants1 = variants.clone();
    let variants2 = variants.clone();
    quote! {
        use std::task::{Poll,Context};
        use std::pin::Pin;
        use tokio::sync::mpsc;
        use futures::{Stream,Future};
        use futures::stream::FusedStream;
        use crate::net::p2p::swarm::{self, EventSender};
        use super::handle::SwarmHandle;
        use std::sync::Arc;
        use crate::net::p2p::IdentityUnion;
        use tracing::trace;

        pub(crate) struct RxBundle {
            pub swarm:mpsc::Receiver<swarm::InEvent>,
            #(pub #idents:mpsc::Receiver<#paths::InEvent>,)*
        }
        impl Future for RxBundle{
            type Output = Rx;
            fn poll(self:Pin<&mut Self>,cx: &mut Context<'_>)->Poll<Self::Output>{
                let mutable_self = self.get_mut();
                match mutable_self.swarm.poll_recv(cx){
                    Poll::Pending => {}
                    Poll::Ready(Some(message)) => return Poll::Ready(Rx::Swarm(message)),
                    Poll::Ready(None) => unreachable!()
                }
                #(
                    match mutable_self.#idents1.poll_recv(cx){
                        Poll::Pending => {}
                        Poll::Ready(Some(message)) => return Poll::Ready(Rx::#variants(message)),
                        Poll::Ready(None) => unreachable!()
                    }
                )*
                Poll::Pending
            }
        }
        impl Stream for RxBundle{
            type Item = Rx;
            fn poll_next(self:Pin<&mut Self>,cx: &mut Context<'_>)->Poll<Option<Self::Item>>{
                let mutable_self = self.get_mut();
                match mutable_self.swarm.poll_recv(cx){
                    Poll::Pending => {}
                    Poll::Ready(Some(message)) => return Poll::Ready(Some(Rx::Swarm(message))),
                    Poll::Ready(None) => unreachable!()
                }
                #(
                    match mutable_self.#idents2.poll_recv(cx){
                        Poll::Pending => {}
                        Poll::Ready(Some(message)) => return Poll::Ready(Some(Rx::#variants1(message))),
                        Poll::Ready(None) => unreachable!()
                    }
                )*
                Poll::Pending
            }
        }
        impl FusedStream for RxBundle{
            fn is_terminated(&self)->bool{false}
        }
        #[derive(Debug)]
        pub(crate) enum Rx{
            #(#variants2(#paths1::InEvent),)*
            Swarm(swarm::InEvent)
        }
        pub(crate) struct HandleBundle{
            swarm:SwarmHandle,
            #(pub #idents3:#paths2::Handle,)*
        }
        impl HandleBundle{
            pub fn new(buffer: usize,swarm_event_source:&EventSender)->(Self,RxBundle){
                let swarm = SwarmHandle::new(buffer);
                #(let #idents4 = #paths3::Handle::new(buffer,swarm_event_source);)*
                (Self{
                    swarm:swarm.0.clone(),
                    #(#idents5:#idents6.0.clone(),)*
                }, RxBundle{
                    swarm:swarm.1,
                    #(#idents7:#idents8.1,)*
                })
            }
        }
        #[derive(Clone)]
        pub struct Manager{
            handle_bundle:Arc<HandleBundle>,
            identity:IdentityUnion,
            executor:tokio::runtime::Handle,
            event_out:tokio::sync::broadcast::Sender<Arc<super::SwarmEvent>>,
        }

        impl Manager{
            pub(crate) fn new(
                handle_bundle:Arc<HandleBundle>,
                identity:IdentityUnion,
                executor:tokio::runtime::Handle,
                event_out:tokio::sync::broadcast::Sender<Arc<super::SwarmEvent>>
            )->Self
            {
                Self { handle_bundle,identity, executor, event_out}
            }
            pub fn executor(&self)->&tokio::runtime::Handle{
                &self.executor
            }
            pub fn identity(&self)->&IdentityUnion{
                &self.identity
            }
            pub fn swarm(&self)-> &SwarmHandle{
                &self.handle_bundle.swarm
            }
            pub fn event_subscriber(&self) -> tokio::sync::broadcast::Sender<Arc<super::SwarmEvent>>{
                self.event_out.clone()
            }
            #(pub fn #idents9(&self)->&#paths4::Handle{
                &self.handle_bundle.#idents10
            })*
        }
    }
    .into()
}
