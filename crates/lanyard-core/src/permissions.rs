use crate::{Client, ClientAuth, Error, Grant, Result, Vault};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
fn hash(token: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(token.as_bytes()))
}
impl ClientAuth {
    pub fn validate(&self) -> Result<()> {
        if uuid::Uuid::parse_str(&self.client_id).is_err()
            || self.app_name.trim().is_empty()
            || self.app_name.len() > 128
            || self.app_name.chars().any(char::is_control)
            || URL_SAFE_NO_PAD
                .decode(&self.token)
                .map(|t| t.len() != 32)
                .unwrap_or(true)
        {
            return Err(Error::Invalid(
                "Invalid client identity or credential.".into(),
            ));
        }
        Ok(())
    }
}
impl Vault {
    pub fn authenticate(&self, auth: &ClientAuth) -> Result<bool> {
        auth.validate()?;
        match self.state.clients.iter().find(|c| c.id == auth.client_id) {
            Some(c) => {
                if c.name != auth.app_name
                    || !bool::from(c.token_hash.as_bytes().ct_eq(hash(&auth.token).as_bytes()))
                {
                    return Err(Error::Denied);
                }
                Ok(true)
            }
            None => Ok(false),
        }
    }
    pub fn has_grant(&self, auth: &ClientAuth, item: &str) -> Result<bool> {
        self.require_key()?;
        if !self.authenticate(auth)? {
            return Ok(false);
        }
        Ok(self
            .state
            .grants
            .iter()
            .any(|g| g.client_id == auth.client_id && g.item_id == item))
    }
    pub fn approve(&mut self, auth: &ClientAuth, item: &str, always: bool) -> Result<()> {
        self.require_key()?;
        if !self.state.items.iter().any(|i| i.id == item) {
            return Err(Error::NotFound);
        }
        let known = self.authenticate(auth)?;
        let mut next = self.state.clone();
        if !known {
            next.clients.push(Client {
                id: auth.client_id.clone(),
                name: auth.app_name.clone(),
                token_hash: hash(&auth.token),
            })
        }
        if always
            && !next
                .grants
                .iter()
                .any(|g| g.client_id == auth.client_id && g.item_id == item)
        {
            next.grants.push(Grant {
                client_id: auth.client_id.clone(),
                item_id: item.into(),
            })
        }
        self.commit(next)
    }
    pub fn revoke(&mut self, client_id: &str, item_id: Option<&str>) -> Result<()> {
        self.require_key()?;
        let mut next = self.state.clone();
        next.grants
            .retain(|g| g.client_id != client_id || item_id.is_some_and(|id| g.item_id != id));
        if item_id.is_none() {
            next.clients.retain(|c| c.id != client_id)
        }
        self.commit(next)
    }
}
