use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens};
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

    if matches!(data,Data::Union(_)) {
        return Err(Error::new(ident.span(), "union are not supported"));
    }

    let (g1, g2, g3) = generics.split_for_impl();

    let mut tokens = quote! {
        #[automatically_derived]
        impl #g1 ::tsue::response::IntoResponse for #ident #g2 #g3
    };

    brace(&mut tokens, |tokens|{
        tokens.extend(quote! {
            fn into_response(self) -> ::tsue::response::Response
        });

        brace(tokens, |tokens|{
            match &data {
                Data::Struct(data) => {
                    tokens.extend(quote! {
                        ::tsue::response::IntoResponse::into_response
                    });

                    paren(tokens, |tokens| match &data.fields {
                        Fields::Named(f) => paren(tokens, |t|self_named(f, t)),
                        Fields::Unnamed(f) => paren(tokens, |t|self_unamed(f, t)),
                        Fields::Unit => <Token![self]>::default().to_tokens(tokens),
                    });
                }
                Data::Enum(data) => {
                    tokens.extend(quote! { match self });

                    paren(tokens, |tokens|{
                        for Variant { ident, fields, .. } in &data.variants {
                            ident.to_tokens(tokens);

                            match fields {
                                Fields::Named(f) => brace(tokens, |t|named(f, t)),
                                Fields::Unnamed(f) => paren(tokens, |t|unamed(f, t)),
                                Fields::Unit => {}
                            }

                            <Token![=>]>::default().to_tokens(tokens);

                            tokens.extend(quote! {
                                ::tsue::response::IntoResponse::into_response
                            });

                            paren(tokens, |tokens| match fields {
                                Fields::Named(f) => paren(tokens, |t|named(f, t)),
                                Fields::Unnamed(f) => paren(tokens, |t|unamed(f, t)),
                                Fields::Unit => tokens.extend(quote!{()}),
                            });
                        }
                    });
                },
                Data::Union(_) => unreachable!(),
            }
        });
    });

    Ok(tokens.into())
}

fn named(fields: &FieldsNamed, tokens: &mut TokenStream) {
    for field in &fields.named {
        field.ident.as_ref().expect("named").to_tokens(tokens);
        <Token![,]>::default().to_tokens(tokens);
    }
}

fn unamed(fields: &FieldsUnnamed, tokens: &mut TokenStream) {
    for (i,_) in fields.unnamed.iter().enumerate() {
        let idx = format_ident!("_{i}");
        tokens.extend(quote! { #idx, });
    }
}

fn self_named(fields: &FieldsNamed, tokens: &mut TokenStream) {
    for field in &fields.named {
        let id = field.ident.as_ref().cloned().expect("named");
        tokens.extend(quote! {
            self.#id,
        });
    }
}

fn self_unamed(fields: &FieldsUnnamed, tokens: &mut TokenStream) {
    for (i,_) in fields.unnamed.iter().enumerate() {
        let idx = Index::from(i);
        tokens.extend(quote! { self.#idx, });
    }
}

// ===== Utils =====

fn brace<F>(tokens: &mut TokenStream, call: F)
where
    F: FnOnce(&mut TokenStream)
{
    token::Brace::default().surround(tokens, call);
}

fn paren<F>(tokens: &mut TokenStream, call: F)
where
    F: FnOnce(&mut TokenStream)
{
    token::Paren::default().surround(tokens, call);
}

