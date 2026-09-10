use crate::{Artifact, Config, Envelope, Error, Result};
use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use std::path::Path;
use std::time::Duration;
use tokio::io::AsyncWriteExt;

pub fn http_client() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .user_agent("ULIX-Update/1")
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(1800))
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() >= 5
                || attempt.url().scheme() != "https"
                || !attempt.url().username().is_empty()
                || attempt.url().password().is_some()
            {
                attempt.error("Unsafe or excessive update redirects")
            } else {
                attempt.follow()
            }
        }))
        .build()?)
}

pub async fn check(config: &Config, platform: &str, arch: &str, current: &str) -> Result<Envelope> {
    let mut response = http_client()?
        .get(config.url(platform, arch, current)?)
        .timeout(Duration::from_secs(30))
        .send()
        .await?
        .error_for_status()?;
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if bytes.len() + chunk.len() > 256 * 1024 {
            return Err(Error::Verification("Manifest is too large.".into()));
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(serde_json::from_slice(&bytes)?)
}

pub async fn download(
    artifact: &Artifact,
    path: &Path,
    mut progress: impl FnMut(u64, u64),
) -> Result<()> {
    let response = http_client()?
        .get(&artifact.url)
        .send()
        .await?
        .error_for_status()?;
    if response
        .content_length()
        .is_some_and(|length| length != artifact.size)
    {
        return Err(Error::Verification(
            "Download size differs from signed metadata.".into(),
        ));
    }
    let mut file = tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .await?;
    let mut stream = response.bytes_stream();
    let mut total = 0u64;
    let mut hash = Sha256::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        total = total
            .checked_add(chunk.len() as u64)
            .ok_or_else(|| Error::Verification("Download is too large.".into()))?;
        if total > artifact.size {
            return Err(Error::Verification(
                "Download exceeded its declared size.".into(),
            ));
        }
        hash.update(&chunk);
        file.write_all(&chunk).await?;
        progress(total, artifact.size);
    }
    file.sync_all().await?;
    if total != artifact.size || hex::encode(hash.finalize()) != artifact.sha256.to_lowercase() {
        return Err(Error::Verification(
            "Downloaded file failed integrity verification.".into(),
        ));
    }
    Ok(())
}
