pub mod settings;
mod conn;
pub mod frame;

pub use frame::Header;
pub use conn::Connection;

#[derive(Clone, Copy, Debug)]
pub enum Role {
    Client,
    Servier,
}

