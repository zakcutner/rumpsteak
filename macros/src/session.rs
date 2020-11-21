use crate::parse::{self, Role};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse2, spanned::Spanned, Data, DeriveInput, Error, Index, Result};

pub fn into_session(input: TokenStream) -> Result<TokenStream> {
    let input = parse2::<DeriveInput>(input)?;

    let role = parse::attribute::<Role>(&input.attrs, "role", input.span())?;

    let ident = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let fields = match &input.data {
        Data::Struct(input) => Ok(&input.fields),
        _ => Err(Error::new_spanned(&input, "expected a struct")),
    }?;

    let mut fields_iter = fields.iter();
    let field = match (fields_iter.next(), fields_iter.next()) {
        (Some(field), None) => Ok(field),
        _ => Err(Error::new_spanned(&fields, "expected exactly one field")),
    }?;

    let field_ty = &field.ty;
    let field_ident = match &field.ident {
        Some(ident) => ident.to_token_stream(),
        None => Index::from(0).to_token_stream(),
    };

    Ok(quote! {
        impl #impl_generics ::session::Session<#role> for #ident #ty_generics #where_clause {
            fn from_state(state: ::session::State<#role>) -> Self {
                Self { #field_ident: ::session::Session::from_state(state) }
            }
        }

        impl #impl_generics ::session::IntoSession<#role> for #ident #ty_generics #where_clause {
            type Session = #field_ty;

            fn into_session(self) -> Self::Session {
                self.#field_ident
            }
        }
    })
}
