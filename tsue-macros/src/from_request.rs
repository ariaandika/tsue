use proc_macro::TokenStream;
use quote::quote;
use syn::{punctuated::Punctuated, *};

pub fn from_request(input: DeriveInput) -> Result<TokenStream> {
    let DeriveInput { ident, generics, data, .. } = input;

    let DataStruct { fields, .. } = match data {
        Data::Struct(data) => data,
        _ => return Err(Error::new(ident.span(), "only struct are supported")),
    };

    let assertions = match &fields {
        Fields::Named(fields) => {
            let len = fields.named.len();
            let asserts = fields
                .named
                .iter()
                .take(len.saturating_sub(1))
                .map(|e|&e.ty)
                .map(|ty| quote! {::tsue::request::assert_fp::<#ty>();});
            let last = fields
                .named
                .last()
                .map(|e|&e.ty)
                .map(|ty| quote! {::tsue::request::assert_fr::<#ty>();});
            quote! { #(#asserts)* #last }
        },
        Fields::Unnamed(fields) => {
            let len = fields.unnamed.len();
            let asserts = fields
                .unnamed
                .iter()
                .take(len.saturating_sub(1))
                .map(|e|&e.ty)
                .map(|ty| quote! {::tsue::request::assert_fp::<#ty>();});
            let last = fields
                .unnamed
                .last()
                .map(|e|&e.ty)
                .map(|ty| quote! {::tsue::request::assert_fr::<#ty>();});
            quote! { #(#asserts)* #last }
        }
        Fields::Unit => quote! {}
    };

    let me = match &fields {
        Fields::Named(fields) => fields.named.iter().map(|e| &e.ty).collect(),
        Fields::Unnamed(fields) => fields.unnamed.iter().map(|e| &e.ty).collect(),
        Fields::Unit => Punctuated::<_, token::Comma>::new(),
    };

    let construct = match &fields {
        Fields::Named(fields) => {
            let fields = fields
                .named
                .iter()
                .enumerate()
                .map(|(i, e)| (&e.ident, Index::from(i)))
                .map(|(e, i)| quote! {#e:e.#i});
            quote! {{ #(#fields),* }}
        }
        Fields::Unnamed(fields) => {
            let fields = fields
                .unnamed
                .iter()
                .enumerate()
                .map(|(i, _)| Index::from(i))
                .map(|i| quote! {e.#i});
            quote! {( #(#fields),* )}
        }
        Fields::Unit => quote! {},
    };

    let (g1, g2, g3) = generics.split_for_impl();

    Ok(quote! {
        const _: () = {
            use ::tsue::request::FromRequest;
            #assertions
            type Me = (#me,);
            #[automatically_derived]
            impl #g1 FromRequest for #ident #g2 #g3 {
                type Error = <Me as FromRequest>::Error;

                type Future = ::tsue::futures::Map<
                    <Me as FromRequest>::Future,
                    fn(Result<Me, Self::Error>) -> Result<Self, Self::Error>,
                >;

                fn from_request(req: ::tsue::request::Request) -> Self::Future {
                    ::tsue::futures::Map::new(<Me as FromRequest>::from_request(req), |e|{
                        let e = e?;
                        Ok(Self #construct)
                    })
                }
            }
        };
    }
    .into())
}

