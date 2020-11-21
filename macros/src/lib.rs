use proc_macro::TokenStream;

mod choice;
mod label;
mod parse;
mod role;
mod roles;
mod session;

#[proc_macro_derive(Choice, attributes(role))]
pub fn choice(input: TokenStream) -> TokenStream {
    choice::choice(input.into())
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

#[proc_macro_derive(Label)]
pub fn label(input: TokenStream) -> TokenStream {
    label::label(input.into())
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

#[proc_macro_derive(IntoSession, attributes(role))]
pub fn into_session(input: TokenStream) -> TokenStream {
    session::into_session(input.into())
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}
