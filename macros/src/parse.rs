use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
use syn::{
    parse::{Parse, ParseStream},
    Attribute, Error, Lifetime, Result, Token, Type,
};

pub struct Role {
    pub lifetime: Lifetime,
    pub comma: Token![,],
    pub ty: Type,
}

impl Parse for Role {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self {
            lifetime: input.parse()?,
            comma: input.parse()?,
            ty: input.parse()?,
        })
    }
}

impl ToTokens for Role {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.lifetime.to_tokens(tokens);
        self.comma.to_tokens(tokens);
        self.ty.to_tokens(tokens);
    }
}

pub fn optional_attribute<T: Parse>(attrs: &[Attribute], ident: &str) -> Result<Option<T>> {
    let mut output = None;
    for attr in attrs {
        if !attr.path.is_ident(ident) {
            continue;
        }

        if output.is_some() {
            return Err(Error::new_spanned(
                attr,
                format_args!("duplicate #[{}(...)] attribute", ident),
            ));
        }

        output = Some(attr.parse_args()?);
    }

    Ok(output)
}

pub fn attribute<T: Parse>(attrs: &[Attribute], ident: &str, span: Span) -> Result<T> {
    optional_attribute(attrs, ident)?
        .ok_or_else(|| Error::new(span, format_args!("expected #[{}(...)] attribute", ident)))
}
