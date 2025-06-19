use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::*;

pub fn from_request(input: DeriveInput) -> Result<proc_macro::TokenStream> {
    let DeriveInput { ident, generics, data, .. } = input;
    let Data::Struct(data) = data else {
        return Err(Error::new(ident.span(), "only struct are supported"))
    };

    let (g1, g2, g3) = generics.split_for_impl();

    let tys = match &data.fields {
        Fields::Named(f) => f.named.iter().map(|e|e.ty.clone()).collect(),
        Fields::Unnamed(f) => f.unnamed.iter().map(|e|e.ty.clone()).collect(),
        Fields::Unit => vec![]
    };
    let me = quote! { (#(#tys),*) };

    let mut tokens = quote! {
        #[automatically_derived]
        impl #g1 #FromRequest for #ident #g2 #g3
    };

    brace(&mut tokens, |tokens|{
        let mut fields = TokenStream::new();

        match &data.fields {
            Fields::Named(f) => brace(&mut fields, |t|named(f, t)),
            Fields::Unnamed(f) => paren(&mut fields, |t|unamed(f, t)),
            Fields::Unit => {}
        }

        tokens.extend(quote! {
            type Error = <#me as #FromRequest>::Error;
            type Future = ::tsue::futures::Map<
                <#me as #FromRequest>::Future,
                fn(Result<#me, Self::Error>) -> Result<Self, Self::Error>,
            >;

            fn from_request(req: ::tsue::request::Request) -> Self::Future {
                ::tsue::futures::Map::new(<#me as #FromRequest>::from_request(req), |e|{
                    let e = e?;
                    Ok(Self #fields)
                })
            }
        });
    });

    Ok(tokens.into())
}

// ===== Generations =====

fn named(fields: &FieldsNamed, tokens: &mut TokenStream) {
    for (i,field) in fields.named.iter().enumerate() {
        let idx = Index::from(i);
        let id = field.ident.as_ref().cloned().expect("named");
        tokens.extend(quote! { #id: e.#idx, });
    }
}

fn unamed(fields: &FieldsUnnamed, tokens: &mut TokenStream) {
    for (i,_) in fields.unnamed.iter().enumerate() {
        let idx = Index::from(i);
        tokens.extend(quote! { e.#idx, });
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

struct FromRequest;

impl ToTokens for FromRequest {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(quote! {::tsue::request::FromRequest});
    }
}

