use tsue::extractor::{Json, State};
use crate::{models::tasks::{AddTask, Tasks}, Db};



pub async fn list(State(db): State<Db>) -> Json<Vec<Tasks>> {
    Json(db.lock().unwrap().clone())
}

pub async fn add(State(db): State<Db>, Json(user_add): Json<AddTask>) {
    let mut db = db.lock().unwrap();
    let id = db.len();
    db.push(Tasks { id, name: user_add.name, });
}

