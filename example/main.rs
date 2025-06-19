use http::{Method, Uri};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tour::Template;
use tsue::{
    helper::{Form, Html, State},
    response::IntoResponse,
    routing::{get, Router},
    service::RouterService,
    FromRequest, IntoResponse,
};

type Db = Arc<Mutex<Vec<Tasks>>>;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let db = Db::new(Mutex::new(<_>::default()));

    let routes = Router::new()
        .nest("/users", users())
        .merge(common())
        .state(db);

    tsue::listen("0.0.0.0:3000", routes).await
}

fn common() -> Router<impl RouterService> {
    Router::new()
        .route("/", get(index).post(index_post))
        .route(
            "/example",
            get(async |arg: ExampleReq| -> ExampleRes { arg.into() }),
        )
}

fn users() -> Router<impl RouterService> {
    Router::new()
        .route("/", get(async || "Users All"))
        .route("/:id", get(async || "Users Id"))
        .route("/add/:id", get(async || "Users Add"))
}

// ===== Routes =====

async fn index(State(db): State<Db>) -> impl IntoResponse {
    let tasks = db.lock().unwrap();
    Html(Index { tasks: tasks.iter().map(|e|e.name.clone()).collect() }.render().unwrap())
}

async fn index_post(db: State<Db>, task: Form<TaskAdd>) -> impl IntoResponse {
    {
        let mut tasks = db.lock().unwrap();
        let id = tasks.len();
        tasks.push(Tasks { id, name: task.0.name });
    }
    index(db).await
}

// ===== Derive Macros =====

#[derive(FromRequest)]
struct ExampleReq {
    method: Method,
    uri: Uri,
}

#[derive(IntoResponse)]
struct ExampleRes {
    content: String,
}

#[derive(IntoResponse)]
#[allow(unused)]
enum EnumRes {
    Ok(String),
    Error {
        code: http::StatusCode,
        msg: String,
    },
    Unknown,
}

impl From<ExampleReq> for ExampleRes {
    fn from(ExampleReq { method, uri }: ExampleReq) -> Self {
        ExampleRes { content: format!("{method} {uri}",) }
    }
}

// ===== Models =====

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Tasks {
    id: usize,
    name: String,
}

#[derive(Debug, Deserialize)]
struct TaskAdd {
    name: String,
}

// ===== Pages =====

#[derive(Debug, Template)]
#[template(path = "/example/index.html")]
struct Index {
    tasks: Vec<String>,
}

