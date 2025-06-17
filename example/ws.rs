use tour::Template;
use tsue::{
    helper::{ws::WebSocket, WsUpgrade, Html},
    routing::{get, Router},
};

#[derive(Template)]
#[template(path = "/example/ws.html")]
struct Ws;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    let routes = Router::new()
        .route("/", get(async||Html(Ws.render().unwrap())))
        .route("/ws", get(async|up: WsUpgrade|up.upgrade(ws)));

    tsue::listen("0.0.0.0:3000", routes).await
}

async fn ws(mut ws: WebSocket) {
    let _ = dbg!(ws.read().await);
    // ws.split()
}

