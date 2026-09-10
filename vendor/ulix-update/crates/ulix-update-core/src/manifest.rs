use crate::{Error, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub product: String,
    pub endpoint: String,
    pub manifest_public_key: String,
    #[serde(default)]
    pub artifact_public_key: String,
    pub channel: String,
    pub edition: String,
}
impl Config {
    pub fn url(&self, platform: &str, arch: &str, current: &str) -> Result<url::Url> {
        for value in [&self.product, &self.channel] {
            if value.is_empty()
                || value.len() > 64
                || !value
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            {
                return Err(Error::Configuration("Invalid product or channel.".into()));
            }
        }
        if !["installed", "portable"].contains(&self.edition.as_str())
            || !["windows", "darwin", "linux"].contains(&platform)
            || !["x86_64", "aarch64"].contains(&arch)
        {
            return Err(Error::Configuration("Unsupported release target.".into()));
        }
        semver::Version::parse(current).map_err(|e| Error::Configuration(e.to_string()))?;
        let mut url = secure_url(&self.endpoint)?;
        url.set_path(&format!(
            "{}/{}/{}/{}/{}",
            url.path().trim_end_matches('/'),
            self.product,
            platform,
            arch,
            current
        ));
        url.query_pairs_mut()
            .clear()
            .append_pair("channel", &self.channel)
            .append_pair("edition", &self.edition);
        Ok(url)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope {
    pub payload: String,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub id: String,
    pub version: String,
    pub url: String,
    pub sha256: String,
    pub signature: String,
    pub size: u64,
    pub kind: String,
    pub notes: String,
    pub published_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub schema: u32,
    pub product: String,
    pub channel: String,
    pub platform: String,
    pub arch: String,
    pub edition: String,
    pub current_version: String,
    pub expires_at: u64,
    pub update: Option<Artifact>,
}

pub fn secure_url(raw: &str) -> Result<url::Url> {
    let url = url::Url::parse(raw).map_err(|e| Error::Configuration(e.to_string()))?;
    if url.scheme() != "https"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || url.host_str().is_none()
    {
        return Err(Error::Verification(
            "HTTPS URLs without embedded credentials are required.".into(),
        ));
    }
    Ok(url)
}

pub fn verify(
    envelope: &Envelope,
    config: &Config,
    platform: &str,
    arch: &str,
    current: &str,
    now: u64,
) -> Result<Manifest> {
    if envelope.payload.len() > 128 * 1024 {
        return Err(Error::Verification("Manifest is too large.".into()));
    }
    let public = STANDARD
        .decode(&config.manifest_public_key)
        .map_err(|_| Error::Configuration("Invalid manifest public key.".into()))?;
    let key_bytes: [u8; 32] = public
        .try_into()
        .map_err(|_| Error::Configuration("Manifest public key must be 32 bytes.".into()))?;
    let key = VerifyingKey::from_bytes(&key_bytes)
        .map_err(|_| Error::Configuration("Invalid Ed25519 key.".into()))?;
    let signature = hex::decode(&envelope.signature)
        .map_err(|_| Error::Verification("Invalid signature encoding.".into()))?;
    let signature = Signature::from_slice(&signature)
        .map_err(|_| Error::Verification("Invalid signature length.".into()))?;
    key.verify_strict(envelope.payload.as_bytes(), &signature)
        .map_err(|_| Error::Verification("Manifest signature did not match.".into()))?;
    let manifest: Manifest = serde_json::from_str(&envelope.payload)?;
    if manifest.schema != 1
        || manifest.product != config.product
        || manifest.channel != config.channel
        || manifest.edition != config.edition
        || manifest.platform != platform
        || manifest.arch != arch
        || manifest.current_version != current
    {
        return Err(Error::Verification(
            "Manifest belongs to another application or target.".into(),
        ));
    }
    if manifest.expires_at <= now || manifest.expires_at > now.saturating_add(3600) {
        return Err(Error::Verification(
            "Manifest expired or the system clock is incorrect. Check again.".into(),
        ));
    }
    if let Some(update) = &manifest.update {
        let next = semver::Version::parse(&update.version)
            .map_err(|_| Error::Verification("Invalid release version.".into()))?;
        let current = semver::Version::parse(current)
            .map_err(|_| Error::Configuration("Invalid application version.".into()))?;
        if !next.cmp_precedence(&current).is_gt()
            || (config.channel == "stable" && !next.pre.is_empty())
        {
            return Err(Error::Verification(
                "Release is not a newer compatible version.".into(),
            ));
        }
        secure_url(&update.url)?;
        if update.size == 0
            || update.size > 4 * 1024 * 1024 * 1024
            || hex::decode(&update.sha256)
                .map(|b| b.len() != 32)
                .unwrap_or(true)
        {
            return Err(Error::Verification(
                "Invalid artifact size or digest.".into(),
            ));
        }
        if (config.edition == "portable" && update.kind != "portable-zip")
            || (config.edition == "installed"
                && (update.kind != "tauri" || update.signature.is_empty()))
        {
            return Err(Error::Verification(
                "Artifact format does not match this edition.".into(),
            ));
        }
    }
    Ok(manifest)
}
