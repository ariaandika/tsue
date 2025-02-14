use std::sync::Mutex;

use tracing_subscriber::EnvFilter;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let state = State {
        counter: Mutex::new(0),
    };

    let result = visor::run(state, |store|async move{
        tracing::trace!("{} {}",store.method,store.path);

        // state
        {
            let state = store.state.downcast_ref::<State>().expect("asserted");
            let mut counter = state.counter.lock().unwrap();
            *counter = counter.wrapping_add(1);
            store.res_header_buf.extend_from_slice(b"Count: ");
            store.res_header_buf.extend_from_slice(itoa::Buffer::new().format(*counter).as_bytes());
            store.res_header_buf.extend_from_slice(b"\r\n");
        }

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

            store.res_body_buf.extend_from_slice(body);
        }
    });

    result.inspect_err(|err|tracing::error!("{err:?}"))
}

struct State {
    counter: Mutex<u8>,
}


