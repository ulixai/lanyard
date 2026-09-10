use crate::{
    model::{State, Wrap},
    Error, Result,
};
use argon2::{Algorithm, Argon2, Params, Version};
use base64::{engine::general_purpose::STANDARD, Engine};
use chacha20poly1305::{
    aead::{Aead, Payload},
    KeyInit, XChaCha20Poly1305, XNonce,
};
use rand::{rngs::OsRng, RngCore};
use zeroize::Zeroizing;
pub(crate) fn random<const N: usize>() -> [u8; N] {
    let mut b = [0; N];
    OsRng.fill_bytes(&mut b);
    b
}
fn pin_key(pin: &str, salt: &[u8]) -> Result<Zeroizing<[u8; 32]>> {
    let params = Params::new(65536, 3, 1, Some(32)).map_err(|e| Error::Invalid(e.to_string()))?;
    let mut key = Zeroizing::new([0; 32]);
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password_into(pin.as_bytes(), salt, key.as_mut())
        .map_err(|_| Error::InvalidPin)?;
    Ok(key)
}
pub(crate) fn encrypt(key: &[u8; 32], bytes: &[u8], context: &[u8]) -> Result<(String, String)> {
    let nonce = random::<24>();
    let encrypted = XChaCha20Poly1305::new(key.into())
        .encrypt(
            XNonce::from_slice(&nonce),
            Payload {
                msg: bytes,
                aad: context,
            },
        )
        .map_err(|_| Error::Storage("Could not encrypt the vault item.".into()))?;
    Ok((STANDARD.encode(nonce), STANDARD.encode(encrypted)))
}
pub(crate) fn decrypt(
    key: &[u8; 32],
    nonce: &str,
    ciphertext: &str,
    context: &[u8],
) -> Result<Zeroizing<Vec<u8>>> {
    let nonce = STANDARD.decode(nonce).map_err(|_| Error::InvalidPin)?;
    let bytes = STANDARD.decode(ciphertext).map_err(|_| Error::InvalidPin)?;
    if nonce.len() != 24 {
        return Err(Error::InvalidPin);
    }
    XChaCha20Poly1305::new(key.into())
        .decrypt(
            XNonce::from_slice(&nonce),
            Payload {
                msg: &bytes,
                aad: context,
            },
        )
        .map(Zeroizing::new)
        .map_err(|_| Error::InvalidPin)
}
pub(crate) fn wrap(pin: &str, key: &[u8; 32]) -> Result<Wrap> {
    if pin.chars().count() < 4 || pin.len() > 256 {
        return Err(Error::Invalid(
            "Use a PIN or passphrase between 4 and 256 characters.".into(),
        ));
    }
    let salt = random::<16>();
    let derived = pin_key(pin, &salt)?;
    let (nonce, ciphertext) = encrypt(&derived, key, b"ULIX Lanyard master key v2")?;
    Ok(Wrap {
        salt: STANDARD.encode(salt),
        nonce,
        ciphertext,
    })
}
pub(crate) fn unwrap(pin: &str, wrap: &Wrap) -> Result<Zeroizing<[u8; 32]>> {
    if pin.len() > 256 {
        return Err(Error::InvalidPin);
    }
    let salt = STANDARD.decode(&wrap.salt).map_err(|_| Error::InvalidPin)?;
    if salt.len() != 16 {
        return Err(Error::InvalidPin);
    }
    let derived = pin_key(pin, &salt)?;
    let plain = decrypt(
        &derived,
        &wrap.nonce,
        &wrap.ciphertext,
        b"ULIX Lanyard master key v2",
    )?;
    Ok(Zeroizing::new(
        plain.as_slice().try_into().map_err(|_| Error::InvalidPin)?,
    ))
}
pub(crate) fn mac(state: &State, key: &[u8; 32]) -> Result<String> {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    let mut clean = state.clone();
    clean.mac.clear();
    let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(key).map_err(|_| Error::InvalidPin)?;
    mac.update(b"ULIX Lanyard metadata v2");
    mac.update(&serde_json::to_vec(&clean)?);
    Ok(STANDARD.encode(mac.finalize().into_bytes()))
}
