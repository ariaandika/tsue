use std::{io::Write as _, time::SystemTime};
use httpdate::HttpDate;
use tracing_subscriber::EnvFilter;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let result = visor::run(State, |_,store|async move{
        tracing::trace!("{} {}",store.method,store.path);

        // headers
        {
            let span = tracing::trace_span!("Header");
            let _guard = span.enter();
            for header in store.headers {
                tracing::trace!("{}: {}",header.name, visor::util::display_str(header.value));
            }
        }

        // read body
        if store.headers.iter().any(|h|h.name.eq_ignore_ascii_case("content-length")) {
            let span = tracing::trace_span!("Body");
            let _guard = span.enter();

            let body = store.body.await.unwrap();
            tracing::trace!("len: {}",body.len());
            if body.len() > 255 {
                tracing::trace!("[body too large to display ({})]",body.len());
            } else {
                tracing::trace!("{}", visor::util::display_str(body));
            }
        }

        // send response
        store.res_header_buf.extend_from_slice(b"HTTP/1.1 200 OK\r\nDate: ");
        let date = HttpDate::from(SystemTime::now());
        write!(store.res_header_buf, "{date}").ok();
        store.res_header_buf.extend_from_slice(b"\r\nContent-Length: 0\r\n\r\n");

    });

    result.inspect_err(|err|tracing::error!("{err:?}"))
}

#[derive(Clone)]
struct State;


