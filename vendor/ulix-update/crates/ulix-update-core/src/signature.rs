use crate::{Error, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
pub fn verify_tauri_signature(public: &str, signature: &str, bytes: &[u8]) -> Result<()> {
    let decode = |s: &str| -> Result<String> {
        String::from_utf8(
            STANDARD
                .decode(s.trim())
                .map_err(|_| Error::Verification("Invalid Minisign encoding.".into()))?,
        )
        .map_err(|_| Error::Verification("Invalid Minisign text.".into()))
    };
    let key = minisign_verify::PublicKey::decode(&decode(public)?)
        .map_err(|e| Error::Verification(e.to_string()))?;
    let signature = minisign_verify::Signature::decode(&decode(signature)?)
        .map_err(|e| Error::Verification(e.to_string()))?;
    key.verify(bytes, &signature, true)
        .map_err(|e| Error::Verification(e.to_string()))
}
