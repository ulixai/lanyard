mod crypto;
mod items;
pub mod legacy;
mod model;
mod permissions;
mod storage;
pub mod utilities;
mod vault;
pub use model::*;
#[cfg(feature = "native")]
pub use storage::NativeStore;
pub use storage::{MemoryStore, SecretStore};
pub use vault::Vault;
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Unlock Lanyard to continue.")]
    Locked,
    #[error("The PIN is incorrect, or the vault integrity check failed.")]
    InvalidPin,
    #[error("{0}")]
    Invalid(String),
    #[error("{0}")]
    Storage(String),
    #[error("This item or project no longer exists.")]
    NotFound,
    #[error("Application access was denied.")]
    Denied,
    #[error("Vault files: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid vault data: {0}")]
    Json(#[from] serde_json::Error),
}
impl serde::Serialize for Error {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}
pub type Result<T> = std::result::Result<T, Error>;
