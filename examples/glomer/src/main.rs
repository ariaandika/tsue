use std::{io, sync::{atomic::{AtomicU8, Ordering}, Arc}};
use tsue::{extractor::State, router::{get,Router}};

fn main() -> io::Result<()> {
    dotenvy::dotenv().ok();
    env_logger::init();
    Router::new()
        .route("/", get(index).post(up))
        .state(Arc::new(AtomicU8::new(0)))
        .listen("0.0.0.0:3000")
}

async fn index() -> &'static str {
    "Tsue Dev!"
}

async fn up(State(counter): State<Arc<AtomicU8>>, body: String) -> String {
    format!("{}: {}",counter.fetch_add(1, Ordering::Relaxed),body.to_uppercase())
}

