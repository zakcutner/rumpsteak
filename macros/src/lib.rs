use proc_macro::TokenStream;

mod message;
mod parse;
mod role;
mod roles;
mod session;

#[proc_macro_derive(Message)]
pub fn message(input: TokenStream) -> TokenStream {
    message::message(input.into())
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

#[proc_macro_derive(Role, attributes(message, route))]
pub fn role(input: TokenStream) -> TokenStream {
    role::role(input.into())
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

#[proc_macro_derive(Roles)]
pub fn roles(input: TokenStream) -> TokenStream {
    roles::roles(input.into())
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

#[proc_macro_attribute]
pub fn session(attr: TokenStream, input: TokenStream) -> TokenStream {
    session::session(attr.into(), input.into())
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}
