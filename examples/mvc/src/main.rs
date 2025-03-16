use models::tasks::Tasks;
use routes::routes;
use std::sync::{Arc, Mutex};

mod models;
mod controllers;

mod routes;

type Db = Arc<Mutex<Vec<Tasks>>>;

fn main() -> std::io::Result<()> {
    let db = Db::new(Mutex::new(<_>::default()));
    routes(db).listen("0.0.0.0:3000")
}
