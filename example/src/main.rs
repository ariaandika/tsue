use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tour::Template;
use tsue::{
    helper::{Form, Html, State},
    response::IntoResponse,
    routing::{Router, get},
    service::HttpService,
};

type Db = Arc<Mutex<Vec<Tasks>>>;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let db = Db::new(Mutex::new(<_>::default()));
    routes(db).listen("0.0.0.0:3000").await
}

fn routes(state: Db) -> Router<impl HttpService> {
    Router::new()
        .route("/", get(index).post(index_post))
        .state(state)
}

// ===== Routes =====

async fn index(State(db): State<Db>) -> impl IntoResponse {
    let tasks = db.lock().unwrap();
    Html(Index { tasks: tasks.iter().map(|e|e.name.clone()).collect() }.render().unwrap())
}

async fn index_post(State(db): State<Db>, Form(task): Form<TaskAdd>) -> impl IntoResponse {
    {
        let mut tasks = db.lock().unwrap();
        let id = tasks.len();
        tasks.push(Tasks { id, name: task.name });
    }
    index(State(db)).await
}

// ===== Models =====

#[derive(Clone, Serialize, Deserialize)]
struct Tasks {
    pub id: usize,
    pub name: String,
}

#[derive(Debug, Deserialize)]
struct TaskAdd {
    pub name: String,
}

// ===== Pages =====

#[derive(Template)]
#[template(root = "example/index.html")]
struct Index {
    tasks: Vec<String>,
}

