use crate::{controllers::tasks, Db};
use tsue::{
    route::{get, post, Router},
    service::HttpService,
};

pub fn routes(state: Db) -> Router<impl HttpService> {
    Router::new()
        .route("/tasks", get(tasks::list))
        .route("/tasks/add", post(tasks::add))
        .state(state)
}

