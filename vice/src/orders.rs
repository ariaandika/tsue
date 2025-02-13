use axum::{extract::State, routing::get, Router};
use sqlx::{prelude::FromRow, PgPool};
use tokio_stream::StreamExt;

#[derive(Debug, FromRow)]
#[allow(dead_code)]
pub struct Order {
    name: String,
}

pub fn routes() -> Router<PgPool> {
    Router::new()
        .route("/", get(list))
}

async fn list(State(db): State<PgPool>) {
    let span = tracing::debug_span!("orders");
    let _guard = span.enter();
    let mut result = sqlx::query_as::<_, Order>("select * from orders").fetch(&db);

    loop {
        match result.try_next().await {
            Ok(Some(ok)) => {
                tracing::debug!("{ok:?}");
            }
            Ok(None) => {
                break;
            }
            Err(err) => {
                tracing::error!("{err}");
                break;
            }
        }
    }

}


