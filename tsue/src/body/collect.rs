use bytes::{BufMut, Bytes, BytesMut};
use http::HeaderMap;
use http_body::Body as _;
use std::{
    pin::Pin,
    task::{Context, Poll, ready},
};

use super::{Body, BodyError};

#[derive(Debug)]
pub struct Collect {
    body: Body,
    collected: Option<Collected>,
}

impl Collect {
    pub fn new(body: Body) -> Self {
        let size_hint = body.size_hint();
        let buffer = BytesMut::with_capacity(size_hint.upper().unwrap_or(size_hint.lower()) as _);
        Self {
            body,
            collected: Some(Collected {
                buffer,
                trailers: None,
            }),
        }
    }
}

impl Future for Collect {
    type Output = Result<Collected, BodyError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let me = self.get_mut();

        loop {
            let collected = me.collected.as_mut().expect("poll after complete");

            let Some(frame) = ready!(Pin::new(&mut me.body).poll_frame(cx)) else {
                return Poll::Ready(Ok(me.collected.take().expect("poll after complete")));
            };

            match frame?.into_data() {
                Ok(data) => collected.buffer.put(data),
                Err(frame) => if let Ok(trailer) = frame.into_trailers() {
                    match collected.trailers.as_mut() {
                        Some(map) => map.extend(trailer),
                        None => collected.trailers = Some(trailer),
                    }
                },
            }
        }
    }
}

#[derive(Debug)]
pub struct Collected {
    buffer: BytesMut,
    trailers: Option<HeaderMap>,
}

impl Collected {
    pub fn into_bytes_mut(self) -> BytesMut {
        self.buffer
    }

    pub fn into_bytes(self) -> Bytes {
        self.buffer.freeze()
    }

    pub fn trailers(&self) -> Option<&HeaderMap> {
        self.trailers.as_ref()
    }

    pub fn trailers_mut(&mut self) -> &mut Option<HeaderMap> {
        &mut self.trailers
    }
}


