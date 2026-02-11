#![allow(unused, reason = "TODO")]

// ===== Stream =====

#[derive(Debug, Clone, Copy)]
pub enum State {
    // Idle,
    Open,
    ReservedLocal,
    ReservedRemote,
    HalfClosedLocal,
    HalfClosedRemote,
    Closed,
}

#[derive(Debug)]
pub struct Stream {
    id: u32,
    state: State,
}

impl Stream {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            state: State::Open,
        }
    }

    pub fn state(&self) -> State {
        self.state
    }

    pub fn set_state(&mut self, state: State) {
        self.state = state;
    }

    pub fn is_reserved(&self) -> bool {
        matches!(self.state, State::ReservedLocal | State::ReservedRemote)
    }
}

// ===== Stream List =====

#[derive(Debug)]
pub struct StreamList {
    max_stream: usize,
}

impl StreamList {
    pub fn new(max_stream: usize) -> Self {
        Self { max_stream }
    }

    pub fn stream_mut(&mut self, id: u32) -> Option<&mut Stream> {
        todo!()
    }

    pub fn create(&mut self, id: u32) -> &mut Stream {
        todo!()
    }
}

