use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens};
use syn::{parse2, Data, DeriveInput, Error, Index, Result};

pub fn roles(input: TokenStream) -> Result<TokenStream> {
    let input = parse2::<DeriveInput>(input)?;

    let ident = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let fields = match &input.data {
        Data::Struct(input) => Ok(&input.fields),
        _ => Err(Error::new_spanned(&input, "expected a struct")),
    }?;

    let pairs = (0..fields.len()).flat_map(|i| {
        (i + 1..fields.len()).map(move |j| {
            let left = format_ident!("role_{}_{}", i, j);
            let right = format_ident!("role_{}_{}", j, i);
            quote! { let (#left, #right) = ::rumpsteak::channel::Pair::pair(); }
        })
    });

    let roles = fields.iter().enumerate().map(|(i, field)| {
        let fields = fields.iter().enumerate().filter(|(j, _)| i != *j);
        let role = fields.enumerate().map(|(index, (j, field))| {
            let field_ident = match &field.ident {
                Some(ident) => ident.to_token_stream(),
                None => Index::from(index).to_token_stream(),
            };

            let ident = format_ident!("role_{}_{}", i, j);
            quote! { #ident }
        });

        let ident = match &field.ident {
            Some(ident) => ident.to_token_stream(),
            None => Index::from(i).to_token_stream(),
        };

        let ty = &field.ty;
        quote! { #ident: #ty::new(#(#role),*) }
    });

    Ok(quote! {
        impl #impl_generics ::core::default::Default for #ident #ty_generics #where_clause {
            fn default() -> Self {
                #(#pairs)*
                Self { #(#roles),* }
            }
        }
    })
}
