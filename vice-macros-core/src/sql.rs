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
        #vis async fn select(db: #POOL) -> sqlx::Result<Vec<Self>> {
            sqlx::query_as::<_, Self>(concat!("select * from ",#table)).fetch_all(db).await
        }
        #vis async fn stream<'a>(db: #POOL_LT) -> impl tokio_stream::Stream<Item = sqlx::Result<Self>> + use<'a> {
            sqlx::query_as::<_, Self>(concat!("select * from ",#table)).fetch(db)
        }
    });
}


use alias::PgPool;
const POOL: PgPool = PgPool(false);
const POOL_LT: PgPool = PgPool(true);

mod alias {
    use super::*;

    pub struct PgPool(pub bool);

    impl ToTokens for PgPool {
        fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
            if self.0 {
                tokens.extend(quote! { &'a sqlx::PgPool });
            } else {
                tokens.extend(quote! { &sqlx::PgPool });
            }
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

