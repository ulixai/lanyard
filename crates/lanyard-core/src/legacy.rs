use crate::{
    crypto,
    items::validate_fields,
    model::{Item, State},
    storage, Category, Error, Fields, Project, Result, Vault,
};
use base64::{
    engine::general_purpose::{STANDARD, URL_SAFE},
    Engine,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fs, io::Read, path::Path};
use subtle::ConstantTimeEq;
use zeroize::Zeroizing;
#[derive(Deserialize)]
struct OldConfig {
    salt: Option<String>,
    pin_hash: Option<String>,
    #[serde(default = "default_tray")]
    close_to_tray: bool,
}
fn default_tray() -> bool {
    true
}
#[derive(Deserialize)]
struct OldMeta {
    items: Vec<OldItem>,
    #[serde(default)]
    projects: Vec<Project>,
}
#[derive(Deserialize)]
struct OldItem {
    id: String,
    title: String,
    #[serde(default = "default_category")]
    category: Category,
    project_id: Option<String>,
}
fn default_category() -> Category {
    Category::ApiKey
}
pub trait LegacyReader {
    fn read_legacy(&self, id: &str) -> Result<String>;
}
#[derive(Debug, Serialize)]
pub struct MissingLegacyItem {
    pub id: String,
    pub title: String,
}
#[derive(Debug, Serialize)]
pub struct ImportReport {
    pub imported: usize,
    pub skipped: usize,
    pub projects_added: usize,
    pub missing: Vec<MissingLegacyItem>,
}
impl Vault {
    /// One metadata commit. Original files and credentials are never modified.
    /// Legacy name-only grants are discarded; clients pair using credentials.
    pub fn import_legacy(
        &mut self,
        directory: &Path,
        old_pin: &str,
        new_pin: &str,
        reader: &dyn LegacyReader,
    ) -> Result<usize> {
        self.import_legacy_report(directory, old_pin, new_pin, reader)
            .map(|report| report.imported)
    }

    /// Merge into an unlocked vault without overwriting existing IDs or permissions.
    /// Decode every new item before writing; commit metadata only once.
    pub fn import_legacy_report(
        &mut self,
        directory: &Path,
        old_pin: &str,
        new_pin: &str,
        reader: &dyn LegacyReader,
    ) -> Result<ImportReport> {
        self.import_legacy_with_options(directory, old_pin, new_pin, reader, false)
    }

    /// Skipping absent credentials is opt-in; other errors always abort before commit.
    pub fn import_legacy_with_options(
        &mut self,
        directory: &Path,
        old_pin: &str,
        new_pin: &str,
        reader: &dyn LegacyReader,
        skip_missing: bool,
    ) -> Result<ImportReport> {
        let merging = self.initialized();
        if merging { self.require_key()?; }
        let config: OldConfig =
            serde_json::from_reader(fs::File::open(directory.join("config.json"))?.take(65536))?;
        let meta: OldMeta = serde_json::from_reader(
            fs::File::open(directory.join("meta.json"))?.take(16 * 1024 * 1024),
        )?;
        let cipher = match (&config.salt, &config.pin_hash) {
            (Some(salt), Some(hash)) => {
                let salt = STANDARD.decode(salt).map_err(|_| Error::InvalidPin)?;
                if salt.len() != 16 {
                    return Err(Error::InvalidPin);
                }
                let mut key = Zeroizing::new([0; 32]);
                pbkdf2::pbkdf2_hmac::<Sha256>(old_pin.as_bytes(), &salt, 480000, key.as_mut());
                let encoded = Zeroizing::new(URL_SAFE.encode(key.as_ref()));
                let digest = format!("{:x}", Sha256::digest(encoded.as_bytes()));
                if !bool::from(digest.as_bytes().ct_eq(hash.as_bytes())) {
                    return Err(Error::InvalidPin);
                }
                Some(fernet::Fernet::new(&encoded).ok_or(Error::InvalidPin)?)
            }
            (None, None) => None,
            _ => {
                return Err(Error::Invalid(
                    "Legacy PIN configuration is incomplete.".into(),
                ))
            }
        };
        let mut project_ids = std::collections::HashSet::new();
        for project in &meta.projects {
            if uuid::Uuid::parse_str(&project.id).is_err() || !project_ids.insert(project.id.clone()) {
                return Err(Error::Invalid("Invalid or duplicate legacy project ID. Nothing was imported.".into()));
            }
        }
        let mut skipped = 0;
        let mut missing = Vec::new();
        let mut decoded = Vec::new();
        let mut ids = std::collections::HashSet::new();
        for item in meta.items {
            if uuid::Uuid::parse_str(&item.id).is_err() || !ids.insert(item.id.clone()) {
                return Err(Error::Invalid(
                    "Invalid or duplicate legacy item ID.".into(),
                ));
            }
            if self.state.items.iter().any(|existing| existing.id == item.id) {
                skipped += 1;
                continue;
            }
            if item
                .project_id
                .as_ref()
                .is_some_and(|id| !meta.projects.iter().any(|p| &p.id == id))
            {
                return Err(Error::Invalid(
                    "Legacy item refers to a missing project.".into(),
                ));
            }
            let raw = Zeroizing::new(match reader.read_legacy(&item.id) {
                Ok(value) => value,
                Err(Error::NotFound) if skip_missing => {
                    missing.push(MissingLegacyItem { id: item.id, title: item.title });
                    continue;
                }
                Err(error) => {
                    let reason = if matches!(error, Error::NotFound) {
                        "The credential was not found in the system keyring".to_owned()
                    } else { error.to_string() };
                    return Err(Error::Storage(format!(
                        "Legacy item {}: {reason}. Nothing was imported; your current vault is unchanged.", item.id
                    )));
                }
            });
            let plain = match &cipher {
                Some(c) => match c.decrypt(&raw) {
                    Ok(bytes) => Zeroizing::new(bytes),
                    Err(_) => {
                        if serde_json::from_str::<Fields>(&raw).is_ok() {
                            Zeroizing::new(raw.as_bytes().to_vec())
                        } else {
                            return Err(Error::Storage(format!(
                                "Cannot decrypt legacy item '{}'. Nothing was imported.",
                                item.title
                            )));
                        }
                    }
                },
                None => Zeroizing::new(raw.as_bytes().to_vec()),
            };
            let fields: Fields = serde_json::from_slice(&plain)?;
            validate_fields(&fields)?;
            decoded.push((item, fields));
        }
        let key = Zeroizing::new(if merging { *self.require_key()? } else { crypto::random::<32>() });
        let mut next = if merging {
            self.state.clone()
        } else {
            State {
                wrap: Some(crypto::wrap(new_pin, &key)?),
                close_to_tray: config.close_to_tray,
                ..State::default()
            }
        };
        let mut projects_added = 0;
        for project in meta.projects {
            if !next.projects.iter().any(|existing| existing.id == project.id) {
                next.projects.push(project);
                projects_added += 1;
            }
        }
        let report = ImportReport { imported: decoded.len(), skipped, projects_added, missing };
        if merging && report.imported == 0 && projects_added == 0 { return Ok(report); }
        let mut created = Vec::new();
        let result = (|| -> Result<()> {
            for (old, fields) in decoded {
                let plain = Zeroizing::new(serde_json::to_vec(&fields)?);
                let (nonce, cipher) = crypto::encrypt(&key, &plain, old.id.as_bytes())?;
                let secret = storage::store(self.store.as_mut(), &format!("{nonce}.{cipher}"))?;
                created.push(secret.clone());
                next.items.push(Item {
                    id: old.id,
                    title: old.title,
                    category: old.category,
                    project_id: old.project_id,
                    fields: fields.keys().cloned().collect(),
                    secret,
                });
            }
            if !merging { self.key = Some(key); }
            self.commit(next)
        })();
        if let Err(e) = result {
            if !merging { self.key = None; }
            for r in created {
                storage::remove(self.store.as_mut(), &r)
            }
            return Err(e);
        }
        Ok(report)
    }
}
#[cfg(feature = "native")]
pub struct NativeLegacyReader;
#[cfg(all(feature = "native", not(windows)))]
impl LegacyReader for NativeLegacyReader {
    fn read_legacy(&self, id: &str) -> Result<String> {
        keyring::Entry::new("Lanyard_Secure_Vault", id)
            .map_err(|e| Error::Storage(e.to_string()))?
            .get_password()
            .map_err(|e| match e {
                keyring::Error::NoEntry => Error::NotFound,
                other => Error::Storage(other.to_string()),
            })
    }
}
#[cfg(all(feature = "native", windows))]
impl LegacyReader for NativeLegacyReader {
    fn read_legacy(&self, id: &str) -> Result<String> {
        use windows_sys::Win32::Security::Credentials::{
            CredFree, CredReadW, CREDENTIALW, CRED_TYPE_GENERIC,
        };
        // Python's newest account uses the service target; older accounts use username@service.
        for name in [
            "Lanyard_Secure_Vault".into(),
            format!("{id}@Lanyard_Secure_Vault"),
        ] {
            let target: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
            let mut credential: *mut CREDENTIALW = std::ptr::null_mut();
            // SAFETY: target is NUL-terminated; the allocated result is freed exactly once.
            if unsafe { CredReadW(target.as_ptr(), CRED_TYPE_GENERIC, 0, &mut credential) } == 0 {
                let code = unsafe { windows_sys::Win32::Foundation::GetLastError() };
                if code == windows_sys::Win32::Foundation::ERROR_NOT_FOUND { continue; }
                return Err(Error::Storage(format!("Cannot read legacy credential {id}: Windows error {code}")));
            }
            if credential.is_null() {
                return Err(Error::Storage(format!("Windows returned no credential data for legacy item {id}")));
            }
            let result = (|| {
                let value = unsafe { &*credential };
                if value.UserName.is_null() {
                    return Err(Error::Storage(format!("Legacy credential {id} has no account name")));
                }
                let mut n = 0;
                while n < 32768 && unsafe { *value.UserName.add(n) } != 0 {
                    n += 1
                }
                if n == 32768 {
                    return Err(Error::Storage(format!("Legacy credential {id} has an invalid account name")));
                }
                let username =
                    String::from_utf16(unsafe { std::slice::from_raw_parts(value.UserName, n) })
                        .map_err(|_| Error::Storage(format!("Legacy credential {id} has an invalid account name")))?;
                if username != id { return Ok(None); }
                if value.CredentialBlob.is_null() || value.CredentialBlobSize == 0 {
                    return Err(Error::Storage(format!("Legacy credential {id} has an empty payload")));
                }
                let bytes = unsafe {
                    std::slice::from_raw_parts(
                        value.CredentialBlob,
                        value.CredentialBlobSize as usize,
                    )
                };
                decode_legacy_blob(bytes).map(Some).map_err(|_| Error::Storage(
                    format!("Legacy credential {id} exists but its payload encoding is unsupported")
                ))
            })();
            unsafe { CredFree(credential.cast()) };
            if let Some(value) = result? {
                return Ok(value);
            }
        }
        Err(Error::NotFound)
    }
}

// Python normally writes UTF-16LE; recognize UTF-8 from older/custom backends too.
#[cfg(any(windows, test))]
fn decode_legacy_blob(bytes: &[u8]) -> std::result::Result<String, ()> {
    fn plausible(text: &str) -> bool {
        serde_json::from_str::<Fields>(text).is_ok() ||
            URL_SAFE.decode(text).map(|raw| raw.len() >= 73 && raw[0] == 0x80).unwrap_or(false)
    }
    if bytes.len() % 2 == 0 {
        let words = Zeroizing::new(bytes.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect::<Vec<_>>());
        if let Ok(text) = String::from_utf16(&words) {
            let text = Zeroizing::new(text);
            let trimmed = text.trim_start_matches('\u{feff}');
            if plausible(trimmed) { return Ok(trimmed.to_owned()); }
        }
    }
    if let Ok(text) = std::str::from_utf8(bytes) {
        if plausible(text) { return Ok(text.to_owned()); }
    }
    Err(())
}
#[cfg(test)]
mod decoding_tests {
    use super::*;
    #[test]
    fn reads_python_utf16_and_legacy_utf8_without_exposing_payloads() {
        for text in [r#"{"key":"a"}"#.to_owned(), r#"{"key":"ab"}"#.to_owned(),
            fernet::Fernet::new(&fernet::Fernet::generate_key()).unwrap().encrypt(b"example")] {
            let wide = text.encode_utf16().flat_map(u16::to_le_bytes).collect::<Vec<_>>();
            assert_eq!(decode_legacy_blob(&wide).unwrap(), text);
            assert_eq!(decode_legacy_blob(text.as_bytes()).unwrap(), text);
        }
        assert!(decode_legacy_blob(&[255,255,255]).is_err());
    }
}
