use tokio::net::TcpStream;
use vice_rc::{
    runtime::{listen_blocking, SetupError},
    service::servicefn::service_fn,
};

fn main() -> Result<(), SetupError> {
    listen_blocking("0.0.0.0", service_fn(app))
}

async fn app(_: TcpStream) -> Result<(), ()> {
    Ok(())
}
