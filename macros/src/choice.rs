use crate::parse::{self, Role};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse2, spanned::Spanned, Data, DeriveInput, Error, Fields, Result};

pub fn choice(input: TokenStream) -> Result<TokenStream> {
    let input = parse2::<DeriveInput>(input)?;

    let role = parse::attribute::<Role>(&input.attrs, "role", input.span())?;
    let role_ty = &role.ty;

    let ident = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let variants = match &input.data {
        Data::Enum(input) => Ok(&input.variants),
        _ => Err(Error::new_spanned(&input, "expected an enum")),
    }?;

    let mut output = TokenStream::new();
    for variant in variants {
        let fields = match &variant.fields {
            Fields::Unnamed(fields) => Ok(&fields.unnamed),
            _ => {
                let message = "expected tuple variants";
                Err(Error::new_spanned(&variant.fields, message))
            }
        }?;

        let mut fields_iter = fields.iter();
        let (label, session) = match (fields_iter.next(), fields_iter.next(), fields_iter.next()) {
            (Some(label), Some(session), None) => Ok((&label.ty, &session.ty)),
            _ => {
                let message = "expected exactly two fields per variant";
                Err(Error::new_spanned(&fields, message))
            }
        }?;

        output.extend(quote! {
            impl #impl_generics ::session::choice::Internal<#role, #label> for #ident #ty_generics #where_clause {
                type Session = #session;
            }
        });
    }

    let idents = variants.iter().map(|variant| &variant.ident);
    output.extend(quote! {
        impl #impl_generics ::session::choice::External<#role> for #ident #ty_generics #where_clause {
            fn choice(
                state: ::session::State<#role>,
                message: <#role_ty as ::session::role::Role>::Message
            ) -> Option<Self> {
                type Message<R> = <R as ::session::role::Role>::Message;

                match message {
                    #(Message::<#role_ty>::#idents(label) => Some(Self::#idents(label, state.into_session())),)*
                    _ => None,
                }
            }
        }
    });

    Ok(output)
}
