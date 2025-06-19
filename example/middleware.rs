use http::StatusCode;
use std::io;
use tsue::{
    request::Request,
    response::{IntoResponse, Response},
    routing::{Next, Router, get},
};

#[tokio::main]
async fn main() -> io::Result<()> {
    let routes = Router::new()
        .route("/", get(async || println!("Handler")))
        .middleware(md)
    ;

    tsue::listen("127.0.0.1:3000", routes).await
}

async fn md(req: Request, next: Next) -> Response {
    println!("[PRE]");
    if req.uri().path() == "/foo" {
        return StatusCode::NOT_FOUND.into_response();
    }
    let res = next.next(req).await;
    println!("[POST]");
    res
}

