use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tsue::{
    helper::{Json, State},
    routing::{Router, get, post},
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
        .route("/tasks", get(list))
        .route("/tasks/add", post(add))
        .state(state)
}

async fn list(State(db): State<Db>) -> Json<Vec<Tasks>> {
    Json(db.lock().unwrap().clone())
}

async fn add(State(db): State<Db>, Json(user_add): Json<AddTask>) {
    let mut db = db.lock().unwrap();
    let id = db.len();
    db.push(Tasks { id, name: user_add.name, });
}

#[derive(Clone, Serialize, Deserialize)]
struct Tasks {
    pub id: usize,
    pub name: String,
}

#[derive(Debug, Deserialize)]
struct AddTask {
    pub name: String,
}

