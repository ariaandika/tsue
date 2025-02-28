use tracing_subscriber::EnvFilter;
use vice::{http::{IntoResponse, Request, Response}, router::{get, Router}, runtime::SetupError};

fn main() -> Result<(), SetupError> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    // let route = get(app);
    // let route = Router::new().route("/", get(app));

    todo!()
    // vice::listen_blocking("0.0.0.0:3000", route)
}

async fn app(req: Request) -> Response {
    tracing::debug!("{:#?}",req);
    if req.body().content_len().is_some() {
        let mut body = req.into_body().bytes_mut().await.unwrap();
        tracing::debug!("{:?}",body);
        body.reverse();
        return Response::new(vice::body::ResBody::Bytes(body.freeze()))
    }
    ().into_response()
}

