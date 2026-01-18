pub mod settings;
mod frame;
mod conn;

pub use frame::Header;
pub use conn::Connection;

#[derive(Clone, Copy, Debug)]
pub enum Role {
    Client,
    Servier,
}

