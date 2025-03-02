use bytes::BytesMut;
use tokio::net::TcpStream;

use crate::http::{IntoResponse, Request};

use super::Service;


// `stream` should not moved when calling service
async fn connection<S>(stream: TcpStream, service: S)
where
    S: Service<Request> + Clone,
    S::Response: IntoResponse,
    S::Error: IntoResponse,
{
    let stream = stream;

    let mut buffer = BytesMut::with_capacity(1024);
    // let mut headers = [httparse::EMPTY_HEADER;24];
    let mut res_buffer = BytesMut::with_capacity(1024);

    'root: loop {
        /*
        {
            // wait for first request data
            let mut stream = stream.lock().await;
            let _ = match stream.read_buf(&mut buffer).await {
                Ok(0) => break 'root,
                Ok(len) => len,
                Err(err) => { #[cfg(debug_assertions)] log::error!("{err}"); break 'root },
            };
        }

        // parse http, rollback to read if partialy parsed
        let mut parse = httparse::Request::new(&mut headers);
        let body_offset = match parse.parse(unsafe {
            // SAFETY: `headers` have the same lifetime with `buffer`
            std::slice::from_raw_parts(buffer.as_ptr(), buffer.len())
        }) {
            Ok(Status::Complete(ok)) => ok,
            Ok(Status::Partial) => continue,
            Err(err) => break log::error!("{err}"),
        };

        // split header and body, `buffer` is now empty
        let mut req_buffer = buffer.split();
        let body = req_buffer.split_off(body_offset);
        let header_buffer = req_buffer.freeze();

        let mut content_len = None;
        let mut header_map = HeaderMap::with_capacity(parse.headers.len());

        // collect headers
        for header in parse.headers {
            if header.name.eq_ignore_ascii_case("content-length") {
                content_len = from_utf8(header.value).ok().and_then(|e|e.parse().ok());
            }

            let key = HeaderName::from_bytes(header.name.as_bytes());
            let val = HeaderValue::from_maybe_shared(header_buffer.slice_ref(header.value));

            match (key,val) {
                (Ok(key),Ok(val)) => { header_map.insert(key,val); },
                (Err(err),_) => log::warn!("skipping header {}: {err}",header.name),
                (_,Err(err)) => log::warn!("skipping header {}: {err}",header.name),
            }
        }

        // all headers now holded by HeaderMap
        drop(header_buffer);

        // construct request
        let req_body = Body::new(content_len, body, stream.clone());
        let mut request = Request::new(req_body);
        let _ = mem::replace(request.headers_mut(), header_map);

        // response scope to make sure all shared bytes dropped
        {
            let response = match service.clone().oneshot(request).await {
                Ok(ok) => ok.into_response(),
                Err(err) => err.into_response(),
            };

            let (mut parts, mut body) = response.into_parts();

            parts.headers.insert(
                HeaderName::from_static("content-length"),
                HeaderValue::from(body.len()),
            );

            res_buffer.extend_from_slice(match parts.version {
                http::Version::HTTP_11 => b"HTTP/1.1 ",
                http::Version::HTTP_2 => b"HTTP/2 ",
                _ => b"HTTP/1.1 ",
            });
            res_buffer.extend_from_slice(parts.status.as_str().as_bytes());
            res_buffer.extend_from_slice(b" ");
            res_buffer.extend_from_slice(
                parts
                .status
                .canonical_reason()
                .unwrap_or("unknown")
                .as_bytes(),
            );
            res_buffer.extend_from_slice(b"\r\n");

            for (key,val) in &parts.headers {
                res_buffer.extend_from_slice(key.as_ref());
                res_buffer.extend_from_slice(b": ");
                res_buffer.extend_from_slice(val.as_ref());
                res_buffer.extend_from_slice(b"\r\n");
            }

            res_buffer.extend_from_slice(b"\r\n");

            let mut stream = stream.lock().await;
            if let Err(err) = stream.write_all_buf(&mut res_buffer).await {
                break log::error!("{err}");
            }
            if let Err(err) = body.write(&mut stream).await {
                break log::error!("{err}");
            }
        }

        buffer.clear();
        res_buffer.clear();

        // presumably all shared buffer is dropped
        if buffer.try_reclaim(1024) {
            log::trace!("buffer reclaimed");
        } else {
            log::trace!("unable to reclaim buffer");
            buffer.reserve(1024);
        }
        */
    }
}

