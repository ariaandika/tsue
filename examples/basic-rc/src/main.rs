use tokio::net::TcpStream;
use vice_rc::{
    http::{noop::Noop, service::HttpService},
    runtime::{listen_blocking, SetupError},
};

fn main() -> Result<(), SetupError> {
    env_logger::init();
    let service = HttpService::new(Noop);
    listen_blocking("0.0.0.0:3000", service)
}

async fn http_service(_: TcpStream) -> Result<(), ()> {
    Ok(())
}
