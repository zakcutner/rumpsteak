use crate::parse;
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse2, spanned::Spanned, Data, DeriveInput, Error, Index, Result, Type};

pub fn role(input: TokenStream) -> Result<TokenStream> {
    let input = parse2::<DeriveInput>(input)?;

    let message = parse::attribute::<Type>(&input.attrs, "message", input.span())?;

    let ident = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let mut output = quote! {
        impl ::rumpsteak::Role for #ident {
            type Message = #message;
        }
    };

    let fields = match &input.data {
        Data::Struct(input) => Ok(&input.fields),
        _ => Err(Error::new_spanned(&input, "expected a struct")),
    }?;

    for (i, field) in fields.iter().enumerate() {
        let route = parse::attribute::<Type>(&field.attrs, "route", field.span())?;

        let field_ty = &field.ty;
        let field_ident = match &field.ident {
            Some(ident) => ident.to_token_stream(),
            None => Index::from(i).to_token_stream(),
        };

        output.extend(quote! {
            impl #impl_generics ::rumpsteak::Route<#route> for #ident #ty_generics #where_clause {
                type Route = #field_ty;

                fn route(&mut self) -> &mut Self::Route {
                    &mut self.#field_ident
                }
            }
        });
    }

    Ok(output)
}
