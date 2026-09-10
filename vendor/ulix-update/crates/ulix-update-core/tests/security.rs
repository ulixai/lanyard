use base64::{engine::general_purpose::STANDARD, Engine};
use ed25519_dalek::{Signer, SigningKey};
use sha2::{Digest, Sha256};
use std::{fs, io::Write, path::Path};
use ulix_update_core::{portable::*, *};
fn config() -> (Config, SigningKey) {
    let key = SigningKey::from_bytes(&[7; 32]);
    (
        Config {
            product: "ulysses".into(),
            endpoint: "https://www.ulix.ai/api/updates".into(),
            manifest_public_key: STANDARD.encode(key.verifying_key().as_bytes()),
            artifact_public_key: String::new(),
            channel: "stable".into(),
            edition: "portable".into(),
        },
        key,
    )
}
fn artifact() -> Artifact {
    Artifact {
        id: "release-id".into(),
        version: "1.1.0".into(),
        url: "https://www.ulix.ai/download.zip".into(),
        sha256: "00".repeat(32),
        signature: String::new(),
        size: 100,
        kind: "portable-zip".into(),
        notes: "Update".into(),
        published_at: "2026-09-08T00:00:00Z".into(),
    }
}
fn manifest() -> Manifest {
    Manifest {
        schema: 1,
        product: "ulysses".into(),
        channel: "stable".into(),
        platform: "linux".into(),
        arch: "x86_64".into(),
        edition: "portable".into(),
        current_version: "1.0.0".into(),
        expires_at: 1500,
        update: Some(artifact()),
    }
}
fn envelope(m: &Manifest, key: &SigningKey) -> Envelope {
    let payload = serde_json::to_string(m).unwrap();
    let signature = hex::encode(key.sign(payload.as_bytes()).to_bytes());
    Envelope { payload, signature }
}
#[test]
fn signature_target_and_expiry_are_bound() {
    let (c, k) = config();
    let m = manifest();
    let mut e = envelope(&m, &k);
    assert!(verify(&e, &c, "linux", "x86_64", "1.0.0", 1000).is_ok());
    assert!(verify(&e, &c, "windows", "x86_64", "1.0.0", 1000).is_err());
    assert!(verify(&e, &c, "linux", "x86_64", "1.0.0", 1500).is_err());
    e.payload.push(' ');
    assert!(verify(&e, &c, "linux", "x86_64", "1.0.0", 1000).is_err());
}
#[test]
fn downgrade_prerelease_and_build_metadata_are_not_updates() {
    let (c, k) = config();
    for version in ["0.9.0", "1.0.0", "1.0.0+different", "1.1.0-beta.1"] {
        let mut m = manifest();
        m.update.as_mut().unwrap().version = version.into();
        assert!(
            verify(&envelope(&m, &k), &c, "linux", "x86_64", "1.0.0", 1000).is_err(),
            "{version}"
        )
    }
}
#[test]
fn urls_and_archive_paths_fail_closed() {
    let (c, _) = config();
    assert!(c.url("linux", "x86_64", "1.0.0").is_ok());
    for path in [
        "../secret",
        "/secret",
        "C:/Windows",
        "app/../data",
        "app\\data",
        ".hidden",
        "CON.txt",
        "app/trailing.",
    ] {
        assert!(safe_relative(path).is_err(), "{path}")
    }
    for url in [
        "http://ulix.ai/a",
        "https://user:pass@ulix.ai/a",
        "file:///tmp/app",
        "https://ulix.ai/a#fragment",
    ] {
        let mut cfg = c.clone();
        cfg.endpoint = url.into();
        assert!(cfg.url("linux", "x86_64", "1.0.0").is_err())
    }
}
fn package(version: &str) -> Package {
    Package {
        schema: 1,
        product: "ulysses".into(),
        version: version.into(),
        launch: "app/run".into(),
        managed: vec!["app".into(), "ulix-update-helper".into()],
    }
}
fn installation(root: &Path) {
    fs::create_dir_all(root.join("app")).unwrap();
    fs::create_dir(root.join("models")).unwrap();
    fs::create_dir(root.join("data")).unwrap();
    fs::write(root.join("app/run"), b"old").unwrap();
    fs::write(root.join("ulix-update-helper"), b"old helper").unwrap();
    fs::write(root.join("models/model.gguf"), b"huge model").unwrap();
    fs::write(root.join("data/workspace.json"), b"chat history").unwrap();
    fs::write(
        root.join(MARKER),
        serde_json::to_vec(&package("1.0.0")).unwrap(),
    )
    .unwrap();
}
fn archive(path: &Path, extra: Option<&str>) -> Artifact {
    let mut zip = zip::ZipWriter::new(fs::File::create(path).unwrap());
    let options = zip::write::SimpleFileOptions::default().unix_permissions(0o755);
    for (name, bytes) in [
        (MARKER, serde_json::to_vec(&package("1.1.0")).unwrap()),
        ("app/run", b"new".to_vec()),
        ("ulix-update-helper", b"new helper".to_vec()),
    ] {
        zip.start_file(name, options).unwrap();
        zip.write_all(&bytes).unwrap()
    }
    if let Some(name) = extra {
        zip.start_file(name, options).unwrap();
        zip.write_all(b"malicious").unwrap()
    }
    zip.finish().unwrap();
    let data = fs::read(path).unwrap();
    let mut a = artifact();
    a.size = data.len() as u64;
    a.sha256 = hex::encode(Sha256::digest(data));
    a
}
#[test]
fn portable_update_preserves_user_data_and_keeps_backup() {
    let t = tempfile::tempdir().unwrap();
    installation(t.path());
    let zip = t.path().join("release.zip");
    let a = archive(&zip, None);
    let (_, backup) = apply_archive(t.path(), &zip, "ulysses", "1.0.0", &a).unwrap();
    assert_eq!(fs::read(t.path().join("app/run")).unwrap(), b"new");
    assert_eq!(fs::read(backup.join("app/run")).unwrap(), b"old");
    assert_eq!(
        fs::read(t.path().join("models/model.gguf")).unwrap(),
        b"huge model"
    );
    assert_eq!(
        fs::read(t.path().join("data/workspace.json")).unwrap(),
        b"chat history"
    );
    finish_update(t.path(), "ulysses", "1.1.0").unwrap();
    assert!(!t.path().join(JOURNAL).exists());
    assert!(backup.exists());
}
#[test]
fn interrupted_handoff_rolls_back_idempotently() {
    let t = tempfile::tempdir().unwrap();
    installation(t.path());
    let zip = t.path().join("release.zip");
    let a = archive(&zip, None);
    apply_archive(t.path(), &zip, "ulysses", "1.0.0", &a).unwrap();
    assert!(recover(t.path()).unwrap().is_some());
    assert_eq!(fs::read(t.path().join("app/run")).unwrap(), b"old");
    assert_eq!(read_package(t.path()).unwrap().version, "1.0.0");
    assert!(recover(t.path()).unwrap().is_none());
}
#[test]
fn malformed_archive_preserves_original() {
    for extra in ["../escape", "data/workspace.json", "undeclared"] {
        let t = tempfile::tempdir().unwrap();
        installation(t.path());
        let zip = t.path().join("release.zip");
        let a = archive(&zip, Some(extra));
        assert!(apply_archive(t.path(), &zip, "ulysses", "1.0.0", &a).is_err());
        assert_eq!(fs::read(t.path().join("app/run")).unwrap(), b"old");
        assert_eq!(
            fs::read(t.path().join("data/workspace.json")).unwrap(),
            b"chat history"
        );
    }
}
#[test]
fn digest_is_rechecked_before_replacing_files() {
    let t = tempfile::tempdir().unwrap();
    installation(t.path());
    let zip = t.path().join("release.zip");
    let mut a = archive(&zip, None);
    a.sha256 = "ff".repeat(32);
    assert!(apply_archive(t.path(), &zip, "ulysses", "1.0.0", &a).is_err());
    assert_eq!(fs::read(t.path().join("app/run")).unwrap(), b"old");
}
