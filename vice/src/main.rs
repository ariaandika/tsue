use axum::{routing::get, Router};
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

const ADDR: &'static str = "localhost:3000";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let tcp = TcpListener::bind(ADDR).await?;
    let routes = Router::new()
        .route("/", get(||async { "Axum Dev !" }));

    Ok(axum::serve(tcp,routes).await?)
}

