use proc_macro::TokenStream;

mod from_request;
mod into_response;

#[proc_macro_derive(FromRequest)]
pub fn from_request(input: TokenStream) -> TokenStream {
    match from_request::from_request(syn::parse_macro_input!(input as syn::DeriveInput)) {
        Ok(ok) => ok,
        Err(err) => err.into_compile_error().into(),
    }
}

#[proc_macro_derive(IntoResponse)]
pub fn into_response(input: TokenStream) -> TokenStream {
    match into_response::into_response(syn::parse_macro_input!(input as syn::DeriveInput)) {
        Ok(ok) => ok,
        Err(err) => err.into_compile_error().into(),
    }
}
