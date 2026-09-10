use crate::{model::SecretRef, Error, Result};
use std::collections::HashMap;
const CHUNK: usize = 1000;
const MAX_CHUNKS: usize = 2048;
pub trait SecretStore: Send {
    fn read(&self, account: &str) -> Result<String>;
    fn write(&mut self, account: &str, value: &str) -> Result<()>;
    fn delete(&mut self, account: &str) -> Result<()>;
}
#[derive(Default)]
pub struct MemoryStore(pub HashMap<String, String>);
impl SecretStore for MemoryStore {
    fn read(&self, a: &str) -> Result<String> {
        self.0
            .get(a)
            .cloned()
            .ok_or_else(|| Error::Storage("Credential is missing.".into()))
    }
    fn write(&mut self, a: &str, v: &str) -> Result<()> {
        self.0.insert(a.into(), v.into());
        Ok(())
    }
    fn delete(&mut self, a: &str) -> Result<()> {
        self.0.remove(a);
        Ok(())
    }
}
#[cfg(feature = "native")]
pub struct NativeStore;
#[cfg(feature = "native")]
fn entry(a: &str) -> Result<keyring::Entry> {
    keyring::Entry::new("ULIX_Lanyard_V2", a).map_err(|e| Error::Storage(e.to_string()))
}
#[cfg(feature = "native")]
impl SecretStore for NativeStore {
    fn read(&self, a: &str) -> Result<String> {
        entry(a)?
            .get_password()
            .map_err(|e| Error::Storage(e.to_string()))
    }
    fn write(&mut self, a: &str, v: &str) -> Result<()> {
        entry(a)?
            .set_password(v)
            .map_err(|e| Error::Storage(e.to_string()))
    }
    fn delete(&mut self, a: &str) -> Result<()> {
        match entry(a)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(Error::Storage(e.to_string())),
        }
    }
}
fn account(r: &SecretRef, i: usize) -> String {
    format!("{}:{i}", r.id)
}
pub(crate) fn store(store: &mut dyn SecretStore, value: &str) -> Result<SecretRef> {
    if !value.is_ascii() || value.len() > CHUNK * MAX_CHUNKS {
        return Err(Error::Invalid(
            "Encrypted item exceeds the storage limit.".into(),
        ));
    }
    let r = SecretRef {
        id: uuid::Uuid::new_v4().to_string(),
        chunks: value.len().div_ceil(CHUNK),
    };
    for (i, chunk) in value.as_bytes().chunks(CHUNK).enumerate() {
        let text = std::str::from_utf8(chunk).map_err(|e| Error::Storage(e.to_string()))?;
        if let Err(error) = store.write(&account(&r, i), text) {
            remove(store, &r);
            return Err(error);
        }
    }
    Ok(r)
}
pub(crate) fn read(store: &dyn SecretStore, r: &SecretRef) -> Result<String> {
    if r.chunks == 0 || r.chunks > MAX_CHUNKS {
        return Err(Error::Storage("Invalid credential reference.".into()));
    }
    let mut value = String::new();
    for i in 0..r.chunks {
        let chunk = store.read(&account(r, i))?;
        if chunk.len() > CHUNK {
            return Err(Error::Storage("Invalid credential chunk.".into()));
        }
        value.push_str(&chunk)
    }
    Ok(value)
}
pub(crate) fn remove(store: &mut dyn SecretStore, r: &SecretRef) {
    for i in 0..r.chunks.min(MAX_CHUNKS) {
        let _ = store.delete(&account(r, i));
    }
}
