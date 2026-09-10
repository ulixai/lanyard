use crate::{Error, Fields, Result};
use rand::rngs::OsRng;
use rsa::pkcs8::{EncodePrivateKey, EncodePublicKey, LineEnding};
/// Parses data only: never expands variables, runs commands or modifies the environment.
pub fn parse_env(text: &str) -> Result<Fields> {
    if text.len() > 1024 * 1024 {
        return Err(Error::Invalid("Environment file exceeds 1 MB.".into()));
    }
    let mut fields = Fields::new();
    let mut lines = text.trim_start_matches('\u{feff}').lines().enumerate();
    while let Some((index, line)) = lines.next() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line
            .strip_prefix("export ")
            .map(str::trim_start)
            .unwrap_or(line);
        let (key, value) = line
            .split_once('=')
            .ok_or_else(|| Error::Invalid(format!("Expected NAME=value on line {}.", index + 1)))?;
        let key = key.trim();
        if key.is_empty()
            || !key
                .bytes()
                .enumerate()
                .all(|(i, b)| b == b'_' || b.is_ascii_alphabetic() || (i > 0 && b.is_ascii_digit()))
        {
            return Err(Error::Invalid(format!(
                "Invalid environment name on line {}.",
                index + 1
            )));
        }
        let value = value.trim();
        let quote = value.chars().next().filter(|c| *c == '\'' || *c == '"');
        let parsed = if let Some(quote) = quote {
            let mut raw = value[1..].to_string();
            let end = loop {
                let mut escape = false;
                let mut found = None;
                for (i, c) in raw.char_indices() {
                    if c == quote && !escape {
                        found = Some(i);
                        break;
                    }
                    escape = c == '\\' && !escape
                }
                if let Some(end) = found {
                    break end;
                }
                if let Some((_, next)) = lines.next() {
                    raw.push('\n');
                    raw.push_str(next)
                } else {
                    return Err(Error::Invalid(format!(
                        "Unclosed quote on line {}.",
                        index + 1
                    )));
                }
            };
            let trailing = raw[end + 1..].trim();
            if !trailing.is_empty() && !trailing.starts_with('#') {
                return Err(Error::Invalid(format!(
                    "Unexpected text after quoted value on line {}.",
                    index + 1
                )));
            }
            if quote == '"' {
                unescape(&raw[..end])
            } else {
                raw[..end].to_string()
            }
        } else {
            value
                .split_once(" #")
                .map(|(v, _)| v.trim_end())
                .unwrap_or(value)
                .to_string()
        };
        if fields.insert(key.into(), parsed).is_some() {
            return Err(Error::Invalid(format!(
                "Duplicate environment variable {key}."
            )));
        }
    }
    crate::items::validate_fields(&fields)?;
    Ok(fields)
}
fn unescape(raw: &str) -> String {
    let mut result = String::new();
    let mut chars = raw.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => result.push('\n'),
                Some('r') => result.push('\r'),
                Some('t') => result.push('\t'),
                Some('"') => result.push('"'),
                Some('\\') => result.push('\\'),
                Some(other) => {
                    result.push('\\');
                    result.push(other)
                }
                None => result.push('\\'),
            }
        } else {
            result.push(c)
        }
    }
    result
}
pub fn generate_keypair(algorithm: &str) -> Result<Fields> {
    let error = |e: Box<dyn std::error::Error>| Error::Invalid(e.to_string());
    let (private, public) = match algorithm {
        "ed25519" => {
            let key = ed25519_dalek::SigningKey::generate(&mut OsRng);
            (
                key.to_pkcs8_pem(LineEnding::LF)
                    .map_err(|e| error(Box::new(e)))?
                    .to_string(),
                key.verifying_key()
                    .to_public_key_pem(LineEnding::LF)
                    .map_err(|e| error(Box::new(e)))?,
            )
        }
        "rsa-2048" | "rsa-4096" => {
            let key = rsa::RsaPrivateKey::new(
                &mut OsRng,
                if algorithm == "rsa-2048" { 2048 } else { 4096 },
            )
            .map_err(|e| error(Box::new(e)))?;
            (
                key.to_pkcs8_pem(LineEnding::LF)
                    .map_err(|e| error(Box::new(e)))?
                    .to_string(),
                key.to_public_key()
                    .to_public_key_pem(LineEnding::LF)
                    .map_err(|e| error(Box::new(e)))?,
            )
        }
        _ => {
            return Err(Error::Invalid(
                "Choose Ed25519, RSA 2048 or RSA 4096.".into(),
            ))
        }
    };
    Ok(Fields::from([
        ("private_key".into(), private),
        ("public_key".into(), public),
    ]))
}
