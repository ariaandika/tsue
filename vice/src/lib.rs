use anyhow::{anyhow, Result};
use bytes::BytesMut;
use httparse::EMPTY_HEADER;
use std::{
    io::Write as _,
    net::{SocketAddr, TcpListener},
    time::SystemTime,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    runtime::Builder as Tokio,
};

const ADDR: &'static str = "0.0.0.0:3000";
const HEADER_COUNT: usize = 24;
const BUF_SIZE: usize = 1024;

pub fn run() -> Result<()> {
    let tcp = TcpListener::bind(ADDR)?;
    tcp.set_nonblocking(true)?;

    Tokio::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let tcp = tokio::net::TcpListener::from_std(tcp)?;

            loop {
                match tcp.accept().await {
                    Ok((stream,addr)) => {
                        tokio::spawn(connection(stream, addr));
                    },
                    Err(err) => {
                        tracing::debug!("failed to accept new connection: {err}");
                    },
                }
            }
        })
}

async fn connection(mut stream: TcpStream, _addr: SocketAddr) {
    let mut super_buf = BytesMut::with_capacity(BUF_SIZE);
    let mut headers = [EMPTY_HEADER ;HEADER_COUNT];

    let result: Result<()> = 'root: loop {
        match stream.read_buf(&mut super_buf).await {
            Ok(0) => break Ok(()),
            Ok(_) => {}
            Err(err) => break Err(err.into()),
        }

        let buf = util::static_buf(&super_buf);
        let mut request = httparse::Request::new(&mut headers);
        let end = match request.parse(&buf) {
            Ok(httparse::Status::Partial) => continue,
            Ok(httparse::Status::Complete(end)) => end,
            Err(err) => break Err(err.into()),
        };

        {
            for header in request.headers.iter() {
                let _key = header.name;
                let _val = util::display_str(&header.value);
            }
        }

        let method = request.method.expect("parsing complete");
        match method {
            "POST" | "post" => {
                let Some(expected_len) = request.headers.iter()
                    .find(|&e|e.name.eq_ignore_ascii_case("content-length"))
                    .and_then(|e|util::display_str(e.value).parse::<usize>().ok())
                else {
                    break 'root Err(anyhow!("failed to parse content length"));
                };

                while (super_buf.len() - end) < expected_len {
                    match stream.read_buf(&mut super_buf).await {
                        Ok(0) => break 'root Ok(()),
                        Ok(_) => {}
                        Err(err) => break 'root Err(err.into()),
                    }
                }

                let buf = util::static_buf(&super_buf);
                let _body = util::display_str(&buf[end..end+expected_len]);
            }
            _ => {},
        }

        let date = httpdate::HttpDate::from(SystemTime::now());

        let mut response = Vec::<u8>::with_capacity(BUF_SIZE);
        write!(response, "HTTP/1.1 200 OK\r\nDate: {date}").ok();
        response.extend_from_slice(b"\r\nContent-Length: 0\r\n\r\n");

        if let Err(err) = stream.write_all(&response).await {
            break Err(err.into());
        }

        super_buf.clear();
    };

    if let Err(err) = result {
        tracing::error!("{err}");
    }
}

mod util {
    use super::*;

    pub fn static_buf(buf: &BytesMut) -> &'static [u8] {
        unsafe { std::slice::from_raw_parts(buf.as_ptr(), buf.len()) }
    }

    pub fn display_str(buf: &[u8]) -> &str {
        std::str::from_utf8(buf).unwrap_or("<NON-UTF8>")
    }
}

