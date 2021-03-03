use crate::parse;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse2, spanned::Spanned, Data, DeriveInput, Error, Fields, Result, Type};

pub fn choice(input: TokenStream) -> Result<TokenStream> {
    let input = parse2::<DeriveInput>(input)?;

    let message = parse::attribute::<Type>(&input.attrs, "message", input.span())?;

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
            impl #impl_generics ::rumpsteak::Choice<#label> for #ident #ty_generics #where_clause {
                type Session = #session;
            }
        });
    }

    let idents = variants.iter().map(|variant| &variant.ident);
    output.extend(quote! {
        impl #impl_generics ::rumpsteak::Choices<#message> for #ident #ty_generics #where_clause {
            fn unwrap(state: ::rumpsteak::State, message: #message) -> Option<Self> {
                match message {
                    #(#message::#idents(label) => Some(Self::#idents(
                        label,
                        ::rumpsteak::Session::from_state(state)
                    )),)*
                    _ => None,
                }
            }
        }
    });

    Ok(output)
}
