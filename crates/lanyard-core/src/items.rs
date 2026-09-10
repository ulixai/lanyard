use crate::{crypto, storage, Error, Fields, Item, ItemInput, Project, Result, Vault};
use zeroize::Zeroizing;
fn title(value: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty() || value.len() > 256 {
        return Err(Error::Invalid("Enter a title of 1–256 characters.".into()));
    }
    Ok(value.into())
}
pub(crate) fn validate_fields(fields: &Fields) -> Result<()> {
    if fields.is_empty() || fields.len() > 256 || serde_json::to_vec(fields)?.len() > 1024 * 1024 {
        return Err(Error::Invalid(
            "Use 1–256 fields totaling at most 1 MB.".into(),
        ));
    }
    if fields
        .iter()
        .any(|(k, v)| k.trim().is_empty() || k.len() > 256 || v.len() > 512 * 1024)
    {
        return Err(Error::Invalid("A field exceeds the size limit.".into()));
    }
    Ok(())
}
impl Vault {
    pub fn save_project(
        &mut self,
        id: Option<String>,
        name: &str,
        description: &str,
    ) -> Result<String> {
        self.require_key()?;
        if description.len() > 8192 {
            return Err(Error::Invalid("Project description is too long.".into()));
        }
        let name = title(name)?;
        let mut next = self.state.clone();
        let id = match id {
            Some(id) => {
                let p = next
                    .projects
                    .iter_mut()
                    .find(|p| p.id == id)
                    .ok_or(Error::NotFound)?;
                p.title = name;
                p.description = description.into();
                id
            }
            None => {
                let id = uuid::Uuid::new_v4().to_string();
                next.projects.push(Project {
                    id: id.clone(),
                    title: name,
                    description: description.into(),
                });
                id
            }
        };
        self.commit(next)?;
        Ok(id)
    }
    pub fn delete_project(&mut self, id: &str) -> Result<()> {
        self.require_key()?;
        if !self.state.projects.iter().any(|p| p.id == id) {
            return Err(Error::NotFound);
        }
        let deleted: Vec<_> = self
            .state
            .items
            .iter()
            .filter(|i| i.project_id.as_deref() == Some(id))
            .cloned()
            .collect();
        let mut next = self.state.clone();
        next.projects.retain(|p| p.id != id);
        next.items.retain(|i| i.project_id.as_deref() != Some(id));
        next.grants
            .retain(|g| !deleted.iter().any(|i| i.id == g.item_id));
        self.commit(next)?;
        for item in deleted {
            storage::remove(self.store.as_mut(), &item.secret)
        }
        Ok(())
    }
    pub fn save_item(&mut self, input: ItemInput) -> Result<String> {
        self.require_key()?;
        validate_fields(&input.fields)?;
        let name = title(&input.title)?;
        if let Some(project) = &input.project_id {
            if !self.state.projects.iter().any(|p| &p.id == project) {
                return Err(Error::NotFound);
            }
        }
        let old = match &input.id {
            Some(id) => Some(
                self.state
                    .items
                    .iter()
                    .find(|i| &i.id == id)
                    .cloned()
                    .ok_or(Error::NotFound)?,
            ),
            None => None,
        };
        let id = input.id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let plain = Zeroizing::new(serde_json::to_vec(&input.fields)?);
        let (nonce, cipher) = crypto::encrypt(self.require_key()?, &plain, id.as_bytes())?;
        let secret = storage::store(self.store.as_mut(), &format!("{nonce}.{cipher}"))?;
        let item = Item {
            id: id.clone(),
            title: name,
            category: input.category,
            project_id: input.project_id,
            fields: input.fields.keys().cloned().collect(),
            secret: secret.clone(),
        };
        let mut next = self.state.clone();
        if let Some(index) = next.items.iter().position(|i| i.id == id) {
            next.items[index] = item
        } else {
            next.items.push(item)
        }
        if let Err(e) = self.commit(next) {
            storage::remove(self.store.as_mut(), &secret);
            return Err(e);
        }
        if let Some(old) = old {
            storage::remove(self.store.as_mut(), &old.secret)
        }
        Ok(id)
    }
    pub fn reveal(&self, id: &str) -> Result<Fields> {
        let key = self.require_key()?;
        let item = self
            .state
            .items
            .iter()
            .find(|i| i.id == id)
            .ok_or(Error::NotFound)?;
        let raw = storage::read(self.store.as_ref(), &item.secret)?;
        let (nonce, cipher) = raw
            .split_once('.')
            .ok_or_else(|| Error::Storage("Credential is damaged.".into()))?;
        Ok(serde_json::from_slice(&crypto::decrypt(
            key,
            nonce,
            cipher,
            id.as_bytes(),
        )?)?)
    }
    pub fn delete_item(&mut self, id: &str) -> Result<()> {
        self.require_key()?;
        let item = self
            .state
            .items
            .iter()
            .find(|i| i.id == id)
            .cloned()
            .ok_or(Error::NotFound)?;
        let mut next = self.state.clone();
        next.items.retain(|i| i.id != id);
        next.grants.retain(|g| g.item_id != id);
        self.commit(next)?;
        storage::remove(self.store.as_mut(), &item.secret);
        Ok(())
    }
    pub fn item_matches(&self, id: &str, category: Option<crate::Category>) -> Result<bool> {
        self.require_key()?;
        let item = self
            .state
            .items
            .iter()
            .find(|i| i.id == id)
            .ok_or(Error::NotFound)?;
        Ok(category.is_none_or(|c| c == item.category))
    }
}
