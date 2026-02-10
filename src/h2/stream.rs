
#[derive(Debug)]
#[repr(transparent)]
pub struct StreamId(u32);

#[derive(Debug, Clone, Copy)]
pub enum State {
    Open,
    HalfClosed,
    Closed,
}

#[derive(Debug)]
pub struct Stream {
    id: StreamId,
    state: State,
}

impl Stream {
    pub fn new(id: StreamId) -> Self {
        Self {
            id,
            state: State::Open,
        }
    }
}


