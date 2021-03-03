use proc_macro2::Span;
use syn::{parse::Parse, Attribute, Error, Result};

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
