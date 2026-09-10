mod download;
mod manifest;
pub mod portable;
mod signature;
pub use download::{check, download, http_client};
pub use manifest::{verify, Artifact, Config, Envelope, Manifest};
pub use signature::verify_tauri_signature;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Update configuration: {0}")]
    Configuration(String),
    #[error("Update verification: {0}")]
    Verification(String),
    #[error("Update transfer: {0}")]
    Network(#[from] reqwest::Error),
    #[error("Update files: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid update data: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Invalid update archive: {0}")]
    Zip(#[from] zip::result::ZipError),
}
impl serde::Serialize for Error {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}
pub type Result<T> = std::result::Result<T, Error>;
