use quote::quote;
use syn::*;

// the main idea is the last field must implement FromRequest and other fields must implement
// FromRequestParts
//
// we cannot add support for auto generic bounds with multiple generics as we cannot know which
// generics is the last (one that must implement FromRequest)
//
// ```
// #[derive(IntoResponse)]
// struct App<T,E> {
//     e: E,
//     t: T,
// }
// ```
//
// to work with this, users must define their own bounds in macro attribute
//
// ```
// #[derive(IntoResponse)]
// #[response(T: IntoResponseParts, E: IntoResponse)]
// struct App<T,E> {
//     e: E,
//     t: T,
// }
// ```
//
// however, we can support single field generics

pub(crate) fn into_response(input: DeriveInput) -> Result<proc_macro::TokenStream> {
    let DeriveInput { ident, generics, data, .. } = input;

    let assertions = match &data {
        Data::Struct(DataStruct { fields, .. }) => match fields {
            Fields::Named(fields) => {
                let len = fields.named.len();
                let asserts = fields
                    .named
                    .iter()
                    .take(len.saturating_sub(1))
                    .map(|e|&e.ty)
                    .map(|ty| quote! {::tsue::response::assert_rp::<#ty>();});
                let last = fields
                    .named
                    .last()
                    .map(|e|&e.ty)
                    .map(|ty| quote! {::tsue::response::assert_rs::<#ty>();});
                quote! { #(#asserts)* #last }
            }
            Fields::Unnamed(fields) => {
                let len = fields.unnamed.len();
                let asserts = fields
                    .unnamed
                    .iter()
                    .take(len.saturating_sub(1))
                    .map(|e|&e.ty)
                    .map(|ty| quote! {::tsue::response::assert_rp::<#ty>();});
                let last = fields
                    .unnamed
                    .last()
                    .map(|e|&e.ty)
                    .map(|ty| quote! {::tsue::response::assert_rs::<#ty>();});
                quote! { #(#asserts)* #last }
            }
            Fields::Unit => quote! {},
        },
        Data::Enum(_) => todo!(),
        _ => return Err(Error::new(ident.span(), "only struct are supported")),
    };

    let destruct = match &data {
        Data::Struct(DataStruct { fields, .. }) => match fields {
            Fields::Named(fields) => {
                let fields = fields
                    .named
                    .iter()
                    .map(|e|e.ident.as_ref().cloned().expect("named"))
                    .map(|id| quote! {self.#id});
                quote! { #(#fields),* }
            }
            Fields::Unnamed(fields) => {
                let fields = fields
                    .unnamed
                    .iter()
                    .enumerate()
                    .map(|(i, _)| Index::from(i))
                    .map(|i| quote! {e.#i});
                quote! { #(#fields),* }
            }
            Fields::Unit => quote! {},
        },
        Data::Enum(_) => todo!(),
        _ => return Err(Error::new(ident.span(), "only struct are supported")),
    };

    let (g1, g2, g3) = generics.split_for_impl();

    Ok(quote! {
        const _: () = {
            use ::tsue::response::IntoResponse;
            #assertions
            #[automatically_derived]
            impl #g1 IntoResponse for #ident #g2 #g3 {
                fn into_response(self) -> ::tsue::response::Response {
                    (#destruct).into_response()
                }
            }
        };
    }
    .into())
}
