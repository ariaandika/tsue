use proc_macro::TokenStream as Token1;
use quote::ToTokens;
use syn::{parse_macro_input, DeriveInput};
use vice_macros_core::sql::Sql;


#[proc_macro_derive(Sql)]
pub fn sql(input: Token1) -> Token1 {
    let derive = parse_macro_input!(input as DeriveInput);
    match Sql::parse_derive(derive) {
        Ok(ok) => ok.to_token_stream().into(),
        Err(err) => err.into_compile_error().into(),
    }
}

