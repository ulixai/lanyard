use base64::{
    engine::general_purpose::{STANDARD, URL_SAFE, URL_SAFE_NO_PAD},
    Engine,
};
use lanyard_core::{Category, ClientAuth, Fields, ItemInput, MemoryStore, Vault};
use sha2::{Digest, Sha256};
use std::{
    fs,
    sync::{Arc, Mutex},
};
#[derive(Clone, Default)]
struct Shared(Arc<Mutex<MemoryStore>>);
impl lanyard_core::SecretStore for Shared {
    fn read(&self, a: &str) -> lanyard_core::Result<String> {
        lanyard_core::SecretStore::read(&*self.0.lock().unwrap(), a)
    }
    fn write(&mut self, a: &str, v: &str) -> lanyard_core::Result<()> {
        lanyard_core::SecretStore::write(&mut *self.0.lock().unwrap(), a, v)
    }
    fn delete(&mut self, a: &str) -> lanyard_core::Result<()> {
        lanyard_core::SecretStore::delete(&mut *self.0.lock().unwrap(), a)
    }
}
fn input(project: Option<String>) -> ItemInput {
    ItemInput {
        id: None,
        title: "Example <script>".into(),
        category: Category::ApiKey,
        project_id: project,
        fields: Fields::from([("API_KEY".into(), "  secret\nmultiline  ".into())]),
    }
}
fn auth() -> ClientAuth {
    ClientAuth {
        client_id: uuid::Uuid::new_v4().to_string(),
        app_name: "Skudio".into(),
        token: URL_SAFE_NO_PAD.encode([3; 32]),
    }
}
#[test]
fn pin_rotation_and_reopening_preserve_exact_secrets() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("vault.json");
    let store = Shared::default();
    let mut v = Vault::open(&path, Box::new(store.clone())).unwrap();
    v.initialize("123456").unwrap();
    let id = v.save_item(input(None)).unwrap();
    v.change_pin("123456", "new passphrase").unwrap();
    v.lock();
    assert!(v.reveal(&id).is_err());
    assert!(v.save_item(input(None)).is_err());
    assert!(v.delete_item(&id).is_err());
    assert!(v.snapshot().items.is_empty());
    let mut v = Vault::open(&path, Box::new(store)).unwrap();
    assert!(v.unlock("123456").is_err());
    v.unlock("new passphrase").unwrap();
    assert_eq!(v.reveal(&id).unwrap()["API_KEY"], "  secret\nmultiline  ");
}
#[test]
fn chunking_projects_grants_and_spoofing() {
    let dir = tempfile::tempdir().unwrap();
    let mut v = Vault::open(dir.path().join("vault.json"), Box::<MemoryStore>::default()).unwrap();
    v.initialize("123456").unwrap();
    let p = v.save_project(None, "Work", "Description").unwrap();
    let mut data = input(Some(p.clone()));
    data.fields.insert("large".into(), "a".repeat(50000));
    let id = v.save_item(data).unwrap();
    assert_eq!(v.reveal(&id).unwrap()["large"].len(), 50000);
    let client = auth();
    v.approve(&client, &id, true).unwrap();
    assert!(v.has_grant(&client, &id).unwrap());
    let mut spoof = auth();
    assert!(!v.has_grant(&spoof, &id).unwrap());
    spoof.client_id = client.client_id.clone();
    spoof.token = URL_SAFE_NO_PAD.encode([4; 32]);
    assert!(v.has_grant(&spoof, &id).is_err());
    v.revoke(&client.client_id, Some(&id)).unwrap();
    assert!(!v.has_grant(&client, &id).unwrap());
    let mut data = input(None);
    data.id = Some(id.clone());
    v.save_item(data).unwrap();
    v.delete_project(&p).unwrap();
    assert!(v.reveal(&id).is_ok());
    v.delete_item(&id).unwrap();
    assert!(v.reveal(&id).is_err());
}
#[test]
fn forged_metadata_cannot_grant_access() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("vault.json");
    let store = Shared::default();
    let mut v = Vault::open(&path, Box::new(store.clone())).unwrap();
    v.initialize("123456").unwrap();
    let id = v.save_item(input(None)).unwrap();
    let mut data: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    data["grants"] = serde_json::json!([{"client_id":"forged","item_id":id}]);
    fs::write(&path, data.to_string()).unwrap();
    let mut v = Vault::open(&path, Box::new(store)).unwrap();
    assert!(v.unlock("123456").is_err());
    assert!(v.locked());
}
struct Reader(String);
impl lanyard_core::legacy::LegacyReader for Reader {
    fn read_legacy(&self, _: &str) -> lanyard_core::Result<String> {
        Ok(self.0.clone())
    }
}
#[test]
fn python_fernet_import_preserves_original() {
    let dir = tempfile::tempdir().unwrap();
    let old = dir.path().join("old");
    fs::create_dir(&old).unwrap();
    let salt = [5; 16];
    let mut key = [0; 32];
    pbkdf2::pbkdf2_hmac::<Sha256>(b"1234", &salt, 480000, &mut key);
    let encoded = URL_SAFE.encode(key);
    let config=serde_json::json!({"salt":STANDARD.encode(salt),"pin_hash":format!("{:x}",Sha256::digest(encoded.as_bytes()))}).to_string();
    fs::write(old.join("config.json"), &config).unwrap();
    let id = uuid::Uuid::new_v4().to_string();
    fs::write(old.join("meta.json"),serde_json::json!({"items":[{"id":id,"title":"Old","category":"api_key","project_id":null,"allowed_apps":["Skudio"]}]}).to_string()).unwrap();
    let reader = Reader(
        fernet::Fernet::new(&encoded)
            .unwrap()
            .encrypt(br#"{"token":"legacy secret"}"#),
    );
    let mut v = Vault::open(
        dir.path().join("new/vault.json"),
        Box::<MemoryStore>::default(),
    )
    .unwrap();
    assert!(v.import_legacy(&old, "bad", "new pin", &reader).is_err());
    assert!(!v.initialized());
    assert_eq!(
        v.import_legacy(&old, "1234", "new pin", &reader).unwrap(),
        1
    );
    assert_eq!(v.reveal(&id).unwrap()["token"], "legacy secret");
    assert!(v.snapshot().grants.is_empty());
    assert_eq!(fs::read_to_string(old.join("config.json")).unwrap(), config);
}
#[test]
fn corrupt_import_aborts() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("config.json"), "{}").unwrap();
    fs::write(dir.path().join("meta.json"),serde_json::json!({"items":[{"id":uuid::Uuid::new_v4(),"title":"Broken","project_id":null}]}).to_string()).unwrap();
    let path = dir.path().join("new.json");
    let mut v = Vault::open(&path, Box::<MemoryStore>::default()).unwrap();
    assert!(v
        .import_legacy(dir.path(), "", "123456", &Reader("broken".into()))
        .is_err());
    assert!(!path.exists());
}
#[test]
fn dotenv_quotes_and_data_only() {
    let f=lanyard_core::utilities::parse_env("export A=abc # comment\nB=\"first\\nsecond\"\nC='line one\nline two'\nRAW=$(touch example)\nEMPTY=").unwrap();
    assert_eq!(f["A"], "abc");
    assert_eq!(f["B"], "first\nsecond");
    assert_eq!(f["C"], "line one\nline two");
    assert_eq!(f["RAW"], "$(touch example)");
    assert_eq!(f["EMPTY"], "");
    assert!(lanyard_core::utilities::parse_env("A=1\nA=2").is_err());
    assert!(lanyard_core::utilities::parse_env("A='unclosed").is_err());
}
#[test]
fn key_formats_match_public_keys() {
    use ed25519_dalek::pkcs8::{DecodePrivateKey, DecodePublicKey};
    let f = lanyard_core::utilities::generate_keypair("ed25519").unwrap();
    let private = ed25519_dalek::SigningKey::from_pkcs8_pem(&f["private_key"]).unwrap();
    let public = ed25519_dalek::VerifyingKey::from_public_key_pem(&f["public_key"]).unwrap();
    assert_eq!(private.verifying_key(), public);
    let f = lanyard_core::utilities::generate_keypair("rsa-2048").unwrap();
    let private = rsa::RsaPrivateKey::from_pkcs8_pem(&f["private_key"]).unwrap();
    let public = rsa::RsaPublicKey::from_public_key_pem(&f["public_key"]).unwrap();
    assert_eq!(private.to_public_key(), public);
}
