use proc_macro::TokenStream;
use quote::quote;

#[proc_macro_attribute]
pub fn impl_stamp(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let ast: syn::DeriveInput = syn::parse(item).unwrap();
    let ident = &ast.ident;
    let data_enum = match &ast.data {
        syn::Data::Enum(data) => data,
        _ => panic!("Not applicable for types outside enum"),
    };
    let arm_iter = data_enum.variants.iter().map(|variant| {
        let ident = &variant.ident;
        let field_pat = match &variant.fields {
            syn::Fields::Named(_) => {
                quote!({stamp,..})
            }
            syn::Fields::Unnamed(_) => quote!((stamp, ..)),
            syn::Fields::Unit => panic!("Not applicable for variants without stamp"),
        };
        quote! {
            Self::#ident #field_pat=> *stamp
        }
    });
    quote! {
        #ast
        impl #ident{
            pub fn stamp(&self)->u128{
                match self{
                    #(#arm_iter),*
                }
            }
        }
    }
    .into()
}

#[proc_macro_attribute]
pub fn into_kind(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let ast: syn::DeriveInput = syn::parse(item).unwrap();
    let ident = &ast.ident;
    let data_enum = match &ast.data {
        syn::Data::Enum(data) => data,
        _ => panic!("Not applicable for types outside enum"),
    };
    let variant_iter = data_enum.variants.iter().map(|variant|{
        let ident = &variant.ident;
        quote!(
            #ident,
        )
    });
    let arm_iter = data_enum.variants.iter().map(|variant| {
        let ident = &variant.ident;
        let field_pat = match &variant.fields {
            syn::Fields::Named(_) => quote!({..}=>),
            syn::Fields::Unnamed(_) => quote!((..)=>),
            syn::Fields::Unit => quote!(=>),
        };
        quote! (
            #field_pat Kind::#ident,
        )
    });
    quote! {
        #ast
        #[derive(Debug)]
        pub enum Kind{
            #(#variant_iter)*
        }
        impl Into<Kind> for &#ident{
            fn into(self)->Kind{
                match self{
                    #(#ident::#arm_iter)*
                }
            }
        }
        impl std::hash::Hash for Kind{
            fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
                format!("{}:{:?}",protocol::PROTOCOL_NAME,self).hash(state);
            }
        }
    }
    .into()
}

