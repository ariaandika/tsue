use anyhow::Context;
use axum::{extract::State, routing::get, Router};
use sqlx::postgres::PgPoolOptions;
use std::{
    env::var,
    net::{IpAddr, Ipv4Addr, SocketAddr},
};
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

const DEFAULT_HOST: IpAddr = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0));
const DEFAULT_PORT: u16 = 3000;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let tcp = {
        let addr = match var("ADDR").ok().and_then(|e|e.parse().ok()) {
            Some(ok) => ok,
            None => {
                let host = var("HOST").ok().and_then(|e|e.parse().ok()).unwrap_or(DEFAULT_HOST);
                let port = var("PORT").ok().and_then(|e|e.parse().ok()).unwrap_or(DEFAULT_PORT);
                SocketAddr::new(host, port)
            },
        };
        TcpListener::bind(addr).await.with_context(||format!("failed to bind {addr}"))?
    };

    let db = {
        let db_url = var("DB_URL").context("failed to get DB_URL env")?;
        PgPoolOptions::new()
            .connect_lazy(&db_url)
            .expect("infallible")
    };

    let routes = Router::new()
        .route("/", get(||async { "Axum Dev !" }))
        .with_state(State(db));

    axum::serve(tcp,routes).await.context("failed to serve")
}

