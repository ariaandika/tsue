use serde::Deserialize;
use tsue::{
    helper::Params,
    routing::{Router, get},
};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    let routes = Router::new()
        .route("/users/:id/:name", get(extract))
        .route("/u2/:id/:name", get(extract2));

    tsue::listen("0.0.0.0:3000", routes).await
}

async fn extract(params: Params<(i32,i32)>) {
    dbg!(params);
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
struct Param {
    name: i32,
    id: i32,
}

async fn extract2(params: Params<Param>) {
    dbg!(params);
}

