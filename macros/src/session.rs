use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use std::mem;
use syn::{
    parse::Nothing, parse2, parse_quote, punctuated::Punctuated, Error, Fields, GenericArgument,
    Index, Item, ItemEnum, ItemStruct, ItemType, PathArguments, Result, Type,
};

fn punctuated_append<T, P: Default>(left: &mut Punctuated<T, P>, mut right: Punctuated<T, P>) {
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

fn augment_type(mut ty: &mut Type) {
    while let Type::Path(path) = unroll_type(ty) {
        let args = match path.path.segments.last_mut() {
            Some(segment) => &mut segment.arguments,
            _ => break,
        };

        if let PathArguments::None = args {
            *args = PathArguments::AngleBracketed(parse_quote!(<>));
        }

        let args = match args {
            PathArguments::AngleBracketed(args) => &mut args.args,
            _ => break,
        };

        let is_empty = args.is_empty();
        punctuated_append(args, parse_quote!('__r, __R));

        if is_empty {
            break;
        }

        ty = match args.last_mut() {
            Some(GenericArgument::Type(ty)) => ty,
            _ => break,
        };
    }
}

fn session_type(mut input: ItemType) -> TokenStream {
    punctuated_append(
        &mut input.generics.params,
        parse_quote!('__r, __R: ::rumpsteak::Role),
    );
    augment_type(&mut input.ty);
    input.into_token_stream()
}

fn session_struct(mut input: ItemStruct) -> Result<TokenStream> {
    let ident = &input.ident;

    punctuated_append(
        &mut input.generics.params,
        parse_quote!('__r, __R: ::rumpsteak::Role),
    );
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    if input.fields.len() != 1 {
        let message = "expected exactly one field";
        return Err(Error::new_spanned(&input.fields, message));
    }

    let field = input.fields.iter_mut().next().unwrap();
    augment_type(&mut field.ty);

    let field_ty = &field.ty;
    let field_ident = match &field.ident {
        Some(ident) => ident.to_token_stream(),
        None => Index::from(0).to_token_stream(),
    };

    let output = quote! {
        impl #impl_generics ::rumpsteak::FromState<'__r, __R> for #ident #ty_generics #where_clause {
            fn from_state(state: ::rumpsteak::State<'__r, __R>) -> Self {
                Self { #field_ident: ::rumpsteak::FromState::from_state(state) }
            }
        }

        impl #impl_generics ::rumpsteak::IntoSession<'__r, __R> for #ident #ty_generics #where_clause {
            type Session = #field_ty;

            fn into_session(self) -> Self::Session {
                self.#field_ident
            }
        }
    };

    Ok(quote!(#input #output))
}

fn session_enum(mut input: ItemEnum) -> Result<TokenStream> {
    let ident = &input.ident;
    if input.variants.is_empty() {
        let message = "expected at least one variant";
        return Err(Error::new_spanned(&input.variants, message));
    }

    punctuated_append(
        &mut input.generics.params,
        parse_quote!('__r, __R: ::rumpsteak::Role),
    );
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

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
        augment_type(ty);
        tys.push(&*ty);
    }

    let mut output = TokenStream::new();
    for (label, ty) in labels.iter().zip(&tys) {
        output.extend(quote! {
            impl #impl_generics ::rumpsteak::Choice<'__r, __R, #label> for #ident #ty_generics #where_clause {
                type Session = #ty;
            }
        });
    }

    let mut generics = input.generics.clone();
    generics.make_where_clause().predicates.push(parse_quote! {
        __R::Message: #(::rumpsteak::Message<#labels> +)*
    });

    let (_, _, where_clause) = generics.split_for_impl();
    output.extend(quote! {
        impl #impl_generics ::rumpsteak::Choices<'__r, __R> for #ident #ty_generics #where_clause {
            fn downcast(
                state: ::rumpsteak::State<'__r, __R>,
                message: __R::Message,
            ) -> ::core::result::Result<Self, __R::Message> {
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

pub fn session(attr: TokenStream, input: TokenStream) -> Result<TokenStream> {
    let Nothing = parse2(attr)?;
    match parse2::<Item>(input)? {
        Item::Type(input) => Ok(session_type(input)),
        Item::Struct(input) => session_struct(input),
        Item::Enum(input) => session_enum(input),
        item => Err(Error::new_spanned(item, "expected a type, struct or enum")),
    }
}
