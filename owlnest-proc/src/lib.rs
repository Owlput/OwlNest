use proc_macro::TokenStream;
use quote::quote;

#[proc_macro_attribute]
pub fn impl_stamp(_attr:TokenStream, item:TokenStream)->TokenStream{
    let ast:syn::DeriveInput = syn::parse(item).unwrap();
    let ident = &ast.ident;
    let data_enum = match &ast.data{
        syn::Data::Enum(data)=>data,
        _=>panic!("Not applicable for types outside enum")
    };
    let arm_iter = data_enum.variants.iter().map(|variant|{
        let ident = &variant.ident;
        let field_pat = match &variant.fields{
            syn::Fields::Named(_) => {
                quote!({stamp,..})
            },
            syn::Fields::Unnamed(_) => quote!((stamp,..)),
            syn::Fields::Unit => panic!("Not applicable for variants without stamp"),
        };
        quote!{
            Self::#ident #field_pat=> *stamp
        }});
    quote! {
        #ast
        impl #ident{
            pub fn stamp(&self)->u128{
                match self{
                    #(#arm_iter),*
                }
            }
        }
    }.into()
}