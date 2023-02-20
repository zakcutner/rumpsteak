use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use std::{collections::HashSet, mem};
use syn::{
    parse::Parse, parse::ParseStream, parse2, parse_quote, punctuated::Punctuated, Error, Fields,
    GenericArgument, GenericParam, Ident, Index, Item, ItemEnum, ItemStruct, ItemType,
    PathArguments, Result, Token, Type,
};

static STATES: [&str; 5] = ["End", "Send", "Receive", "Branch", "Select"];

fn idents_set<P>(params: &Punctuated<GenericParam, P>) -> HashSet<Ident> {
    let idents = params.iter().filter_map(|param| match param {
        GenericParam::Type(ty) => Some(ty.ident.clone()),
        _ => None,
    });
    idents.collect::<HashSet<_>>()
}

fn punctuated_prepend<T, P: Default>(left: &mut Punctuated<T, P>, mut right: Punctuated<T, P>) {
    right.extend(mem::take(left));
    *left = right;
}

fn unroll_type(mut ty: &mut Type) -> &mut Type {
    loop {
        ty = match ty {
            Type::Group(ty) => ty.elem.as_mut(),
            Type::Paren(ty) => ty.elem.as_mut(),
            _ => break,
        }
    }

    ty
}

fn augment_type(mut ty: &mut Type, value: &Type, exclude: &HashSet<Ident>) {
    while let Type::Path(path) = unroll_type(ty) {
        if *path == parse_quote!(Self) {
            break;
        }

        let segment = match path.path.segments.last_mut() {
            Some(segment) => segment,
            _ => break,
        };

        if let PathArguments::None = segment.arguments {
            if exclude.contains(&segment.ident) {
                break;
            }

            segment.arguments = PathArguments::AngleBracketed(parse_quote!(<>));
        }

        let args = match &mut segment.arguments {
            PathArguments::AngleBracketed(args) => &mut args.args,
            _ => break,
        };

        let is_empty = args.is_empty();
        if STATES.contains(&&*segment.ident.to_string()) {
            punctuated_prepend(args, parse_quote!('__r, __R, #value));
        } else {
            punctuated_prepend(args, parse_quote!('__r, __R));
        }

        if is_empty {
            break;
        }

        ty = match args.last_mut() {
            Some(GenericArgument::Type(ty)) => ty,
            _ => break,
        };
    }
}

fn session_type(mut input: ItemType, value: Type) -> TokenStream {
    let exclude = idents_set(&input.generics.params);
    punctuated_prepend(
        &mut input.generics.params,
        parse_quote!('__r, __R: ::rumpsteak::Role),
    );
    augment_type(&mut input.ty, &value, &exclude);
    input.into_token_stream()
}

fn session_struct(mut input: ItemStruct, value: Type) -> Result<TokenStream> {
    let ident = &input.ident;
    let exclude = idents_set(&input.generics.params);

    punctuated_prepend(
        &mut input.generics.params,
        parse_quote!('__r, __R: ::rumpsteak::Role),
    );
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    if input.fields.len() != 1 {
        let message = "expected exactly one field";
        return Err(Error::new_spanned(&input.fields, message));
    }

    let field = input.fields.iter_mut().next().unwrap();
    augment_type(&mut field.ty, &value, &exclude);

    let field_ty = &field.ty;
    let field_ident = match &field.ident {
        Some(ident) => ident.to_token_stream(),
        None => Index::from(0).to_token_stream(),
    };

    let mut output = TokenStream::new();
    output.extend(quote! {
        impl #impl_generics ::rumpsteak::FromState<'__r> for #ident #ty_generics #where_clause {
            type Role = __R;
            type Value = Value;

            fn from_state(state: ::rumpsteak::State<'__r, Self::Role, Self::Value>) -> Self {
                Self { #field_ident: ::rumpsteak::FromState::from_state(state) }
            }
        }

        impl #impl_generics ::rumpsteak::IntoSession<'__r> for #ident #ty_generics #where_clause {
            type Session = #field_ty;

            fn into_session(self) -> Self::Session {
                self.#field_ident
            }
        }
    });

    #[cfg(feature = "serialize")]
    {
        let mut where_clause = where_clause.cloned().unwrap_or_else(|| parse_quote!(where));
        where_clause.predicates.push(parse_quote!(Self: 'static));

        output.extend(quote! {
            impl #impl_generics ::rumpsteak::serialize::Serialize for #ident #ty_generics #where_clause {
                fn serialize(s: &mut ::rumpsteak::serialize::Serializer) {
                    <#field_ty as ::rumpsteak::serialize::Serialize>::serialize(s);
                }
            }
        });
    }

    Ok(quote!(#input #output))
}

fn session_enum(mut input: ItemEnum, value: Type) -> Result<TokenStream> {
    if input.variants.is_empty() {
        let message = "expected at least one variant";
        return Err(Error::new_spanned(&input.variants, message));
    }

    let ident = &input.ident;
    let exclude = idents_set(&input.generics.params);

    let mut generics = input.generics.clone();
    punctuated_prepend(
        &mut generics.params,
        parse_quote!('__q, '__r, __R: ::rumpsteak::Role + '__r),
    );
    let (impl_generics, _, _) = generics.split_for_impl();

    let mut generics = input.generics.clone();
    punctuated_prepend(
        &mut generics.params,
        parse_quote!('__q, __R: ::rumpsteak::Role),
    );
    let (_, ty_generics, where_clause) = generics.split_for_impl();

    let mut idents = Vec::with_capacity(input.variants.len());
    let mut labels = Vec::with_capacity(input.variants.len());
    let mut tys = Vec::with_capacity(input.variants.len());

    for variant in &mut input.variants {
        idents.push(&variant.ident);
        let fields = match &mut variant.fields {
            Fields::Unnamed(fields) => Ok(&mut fields.unnamed),
            fields => Err(Error::new_spanned(fields, "expected tuple variants")),
        }?;

        if fields.len() != 2 {
            let message = "expected exactly two fields per variant";
            return Err(Error::new_spanned(fields, message));
        }

        let mut fields = fields.iter_mut();

        let label = &fields.next().unwrap().ty;
        labels.push(label);

        let ty = &mut fields.next().unwrap().ty;
        augment_type(ty, &value, &exclude);
        tys.push(&*ty);
    }

    let mut output = TokenStream::new();
    for (label, ty) in labels.iter().zip(&tys) {
        output.extend(quote! {
            impl #impl_generics ::rumpsteak::Choice<'__r, #label> for #ident #ty_generics #where_clause {
                type Session = #ty;
            }
        });
    }

    punctuated_prepend(
        &mut input.generics.params,
        parse_quote!('__r, __R: ::rumpsteak::Role),
    );
    let (impl_generics, ty_generics, _) = input.generics.split_for_impl();

    #[cfg(feature = "serialize")]
    {
        let mut where_clause = where_clause.cloned().unwrap_or_else(|| parse_quote!(where));
        where_clause.predicates.push(parse_quote!(Self: 'static));

        output.extend(quote! {
            impl #impl_generics ::rumpsteak::serialize::SerializeChoices for #ident #ty_generics #where_clause {
                fn serialize_choices(mut s: ::rumpsteak::serialize::ChoicesSerializer<'_>) {
                    #(s.serialize_choice::<#labels, #tys>();)*
                }
            }
        });
    }

    let mut generics = input.generics.clone();
    generics.make_where_clause().predicates.push(parse_quote! {
        __R::Message: #(::rumpsteak::Message<#labels> +)*
    });

    let (_, _, where_clause) = generics.split_for_impl();
    output.extend(quote! {
        impl #impl_generics ::rumpsteak::Choices<'__r> for #ident #ty_generics #where_clause {
            type Role = __R;
            type Value = #value;

            fn downcast(
                state: ::rumpsteak::State<'__r, Self::Role, Self::Value>,
                message: <Self::Role as Role>::Message,
            ) -> ::core::result::Result<Self, <Self::Role as Role>::Message> {
                #(let message = match ::rumpsteak::Message::downcast(message) {
                    Ok(label) => {
                        return Ok(Self::#idents(
                            label,
                            ::rumpsteak::FromState::from_state(state)
                        ));
                    }
                    Err(message) => message
                };)*

                Err(message)
            }
        }
    });

    Ok(quote!(#input #output))
}

struct SessionParams(Type, Type);

impl Parse for SessionParams {
    fn parse(content: ParseStream) -> Result<Self> {
        let type1 = content.parse()?;
        content.parse::<Token![,]>()?;
        let type2 = content.parse()?;
        Ok(SessionParams(type1, type2))
    }
}

pub fn session(attr: TokenStream, input: TokenStream) -> Result<TokenStream> {
    let input = parse2::<Item>(input)?;
    match input {
        Item::Type(input) => {
            let SessionParams(_name, value) = parse2(attr)?;
            Ok(session_type(input, value))
        }
        Item::Struct(input) => {
            let SessionParams(_name, value) = parse2(attr)?;
            session_struct(input, value)
        }
        Item::Enum(input) => {
            let SessionParams(_name, value) = parse2(attr)?;
            session_enum(input, value)
        }
        item => Err(Error::new_spanned(item, "expected a type, struct or enum")),
    }
}
