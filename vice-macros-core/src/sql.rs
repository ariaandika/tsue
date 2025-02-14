use proc_macro2::TokenStream;
use quote::{quote, quote_spanned, ToTokens};
use syn::{spanned::Spanned, token::Brace, *};


pub struct Sql {
    vis: Visibility,
    ident: Ident,
    fields: Fields,

    table: String,
}

impl Sql {
    pub fn parse_derive(input: DeriveInput) -> Result<Self> {
        let data = match input.data {
            syn::Data::Struct(data) => data,
            _ => return Err(Error::new(input.ident.span(), "only struct is supported")),
        };

        Ok(Self {
            table: input.ident.to_string().to_lowercase(),

            vis: input.vis,
            ident: input.ident,
            fields: assert::named_fields(data.fields)?,
        })
    }
}

impl ToTokens for Sql {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Sql { ident, .. } = self;

        tokens.extend(quote! {
            impl #ident
        });

        Brace::default().surround(tokens, |tokens|{
            create_metadata(self, tokens);
            create_select(self, tokens);
        });
    }
}

fn create_metadata(sql: &Sql, tokens: &mut TokenStream) {
    let Sql { vis, table, fields, ident } = sql;

    let fields = fields
        .iter()
        .map(|f| f.ident.as_ref().expect("asserted").to_string())
        .map(|f| quote! { #f });

    tokens.extend(quote_spanned! {ident.span()=>
        #vis const TABLE: &'static str = #table;
        #vis const FIELDS: &'static [&'static str] = &[#(#fields)*,];
    });
}

fn create_select(sql: &Sql, tokens: &mut TokenStream) {
    let Sql { vis, table, .. } = sql;
    tokens.extend(quote! {
        #vis async fn select(db: #PgPool) -> sqlx::Result<Vec<Self>> {
            sqlx::query_as::<_, Self>(concat!("select * from ",#table)).fetch_all(db).await
        }
        #vis async fn stream(db: #PgPool) -> impl Stream<Item = sqlx::Result<Self>> {
            sqlx::query_as::<_, Self>(concat!("select * from ",#table)).fetch(db)
        }
    });
}


use alias::PgPool;

mod alias {
    use super::*;

    pub struct PgPool;

    impl ToTokens for PgPool {
        fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
            tokens.extend(quote! { &sqlx::PgPool });
        }
    }
}

mod assert {
    use super::*;

    pub fn named_fields(fields: Fields) -> Result<Fields> {
        match fields
            .iter()
            .find_map(|f|f.ident.is_none().then_some(f.ty.span()))
        {
            Some(span) => Err(Error::new(span, "only named field allowed")),
            None => Ok(fields),
        }
    }

}

