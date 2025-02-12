use std::{io::Write as _, time::SystemTime};
use httpdate::HttpDate;
use tracing_subscriber::EnvFilter;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let result = vice::run(State, |_,headers,body,res_head,_|async move{
        tracing::trace!("Request");

        // headers
        {
            let span = tracing::trace_span!("Header");
            let _guard = span.enter();
            for header in headers {
                tracing::trace!("{}: {}",header.name,vice::util::display_str(header.value));
            }
        }

        // read body
        if headers.iter().any(|h|h.name.eq_ignore_ascii_case("content-length")) {
            let span = tracing::trace_span!("Body");
            let _guard = span.enter();

            let body = body.await.unwrap();
            tracing::trace!("len: {}",body.len());
            if body.len() > 255 {
                tracing::trace!("[body too large to display ({})]",body.len());
            } else {
                tracing::trace!("{}",vice::util::display_str(body));
            }
        }

        // send response
        res_head.extend_from_slice(b"HTTP/1.1 200 OK\r\nDate: ");
        let date = HttpDate::from(SystemTime::now());
        write!(res_head, "{date}").ok();
        res_head.extend_from_slice(b"\r\nContent-Length: 0\r\n\r\n");

    });

    result.inspect_err(|err|tracing::error!("{err:?}"))
}

#[derive(Clone)]
struct State;

