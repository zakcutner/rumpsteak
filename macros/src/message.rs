use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse2, Data, DeriveInput, Error, Fields, Result};

pub fn message(input: TokenStream) -> Result<TokenStream> {
    let input = parse2::<DeriveInput>(input)?;

    let ident = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let variants = match &input.data {
        Data::Enum(input) => Ok(&input.variants),
        _ => Err(Error::new_spanned(&input, "expected an enum")),
    }?;

    let mut output = TokenStream::new();
    for variant in variants {
        let variant_ident = &variant.ident;
        let fields = match &variant.fields {
            Fields::Unnamed(fields) => Ok(&fields.unnamed),
            _ => Err(Error::new_spanned(&variant.fields, "expected tuple fields")),
        }?;

        let mut fields_iter = fields.iter();
        let field = match (fields_iter.next(), fields_iter.next()) {
            (Some(field), None) => Ok(field),
            _ => {
                let message = "expected exactly one field per variant";
                Err(Error::new_spanned(&fields, message))
            }
        }?;

        let ty = &field.ty;
        output.extend(quote! {
            impl #impl_generics ::rumpsteak::Message<#ty> for #ident #ty_generics #where_clause {
                fn wrap(label: #ty) -> Self {
                    Self::#variant_ident(label)
                }

                fn try_unwrap(self) -> ::core::result::Result<#ty, Self> {
                    match self {
                        Self::#variant_ident(label) => Ok(label),
                        _ => Err(self),
                    }
                }
            }
        });
    }

    Ok(output)
}
