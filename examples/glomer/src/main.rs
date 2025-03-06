use std::io;
use vice::{
    http::{self},
    router::{get, Router},
};

fn main() -> io::Result<()> {
    vice::runtime::listen_v2("0.0.0.0:3000", ||Router::new()
        .route("/", get(index))
        .route("/foo", get(foo)))
}

async fn index(_: http::Method, _: http::Method, _: http::Method, body: String) -> String {
    body
}

async fn foo(_: http::Method, body: String) -> String {
    println!("foo");
    body
}

