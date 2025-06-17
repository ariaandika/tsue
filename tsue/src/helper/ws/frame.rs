use bytes::BytesMut;

// ===== OpCode =====

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum OpCode {
    Continuation = 0x0,
    Text = 0x1,
    Binary = 0x2,
    Close = 0x8,
    Ping = 0x9,
    Pong = 0xA,
}

impl OpCode {
    pub(crate) fn try_from_byte(byte: u8) -> Option<OpCode> {
        use OpCode::*;
        match byte {
            0x0 => Some(Continuation),
            0x1 => Some(Text),
            0x2 => Some(Binary),
            0x8 => Some(Close),
            0x9 => Some(Ping),
            0xA => Some(Pong),
            _ => None,
        }
    }

    pub fn is_control(&self) -> bool {
        matches!(self, OpCode::Close | OpCode::Ping | OpCode::Pong)
    }
}

pub struct Frame {
    fin: bool,
    opcode: OpCode,
    payload: BytesMut,
}

impl Frame {
    pub(crate) fn new(fin: bool, opcode: OpCode, payload: BytesMut) -> Self {
        Self { fin, opcode, payload }
    }

    pub fn fin(&self) -> bool {
        self.fin
    }

    pub fn opcode(&self) -> OpCode {
        self.opcode
    }

    pub fn payload(&self) -> &BytesMut {
        &self.payload
    }

    pub fn payload_mut(&mut self) -> &mut BytesMut {
        &mut self.payload
    }
}

