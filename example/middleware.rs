use http::StatusCode;
use std::{
    io,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use tsue::{
    helper::State,
    request::{Request, RequestExt},
    response::{IntoResponse, Response},
    routing::{Next, Router, get},
};

struct App {
    maintenance: AtomicBool,
}

type Data = State<Arc<App>>;

#[tokio::main]
async fn main() -> io::Result<()> {
    let routes = Router::new()
        .route("/", get(async || "Ok"))
        .route("/maintenance", get(maintenance))
        .middleware(md)
        .state(Arc::new(App {
            maintenance: AtomicBool::new(false),
        }));

    tsue::listen("127.0.0.1:3000", routes).await
}

async fn maintenance(state: Data) {
    state.maintenance.fetch_not(Ordering::Relaxed);
}

async fn md(mut req: Request, next: Next) -> Response {
    let app = req.extract_parts::<Data>().await.unwrap();

    if req.uri().path() != "/maintenance" && app.maintenance.load(Ordering::Relaxed) {
        return (StatusCode::SERVICE_UNAVAILABLE, "Under Maintenance").into_response();
    }

    next.next(req).await
}

