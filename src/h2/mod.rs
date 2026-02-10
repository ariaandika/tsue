pub mod settings;
pub mod frame;
pub mod hpack;
pub mod stream;

pub mod state;
mod conn;

pub use conn::Connection;

#[derive(Clone, Copy, Debug)]
pub enum Role {
    Client,
    Servier,
}

