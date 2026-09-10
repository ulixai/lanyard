use crate::{crypto, model::State, ClientInfo, Error, Result, SecretStore, Snapshot};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use subtle::ConstantTimeEq;
use zeroize::Zeroizing;
pub struct Vault {
    pub(crate) path: PathBuf,
    pub(crate) state: State,
    pub(crate) key: Option<Zeroizing<[u8; 32]>>,
    pub(crate) store: Box<dyn SecretStore>,
    failures: u32,
    retry_at: Instant,
}
impl Vault {
    pub fn open(path: impl AsRef<Path>, store: Box<dyn SecretStore>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let state: State = if path.exists() {
            serde_json::from_reader(fs::File::open(&path)?.take(16 * 1024 * 1024))?
        } else {
            State::default()
        };
        if state.schema != 2
            || (state.wrap.is_none() && (!state.items.is_empty() || !state.projects.is_empty()))
        {
            return Err(Error::Invalid(
                "Unsupported or damaged vault. The original file has been preserved.".into(),
            ));
        }
        Ok(Self {
            path,
            state,
            key: None,
            store,
            failures: 0,
            retry_at: Instant::now(),
        })
    }
    pub fn initialized(&self) -> bool {
        self.state.wrap.is_some()
    }
    pub fn locked(&self) -> bool {
        self.key.is_none()
    }
    pub fn lock(&mut self) {
        self.key = None
    }
    pub fn initialize(&mut self, pin: &str) -> Result<()> {
        if self.initialized() {
            return Err(Error::Invalid("This vault is already initialized.".into()));
        }
        let key = Zeroizing::new(crypto::random::<32>());
        let mut next = self.state.clone();
        next.wrap = Some(crypto::wrap(pin, &key)?);
        self.key = Some(key);
        if let Err(e) = self.commit(next) {
            self.key = None;
            return Err(e);
        }
        Ok(())
    }
    pub fn unlock(&mut self, pin: &str) -> Result<()> {
        if Instant::now() < self.retry_at {
            return Err(Error::Invalid(
                "Too many attempts. Wait a moment before trying again.".into(),
            ));
        }
        let wrap = self
            .state
            .wrap
            .as_ref()
            .ok_or_else(|| Error::Invalid("Create your vault first.".into()))?;
        let result = (|| {
            let key = crypto::unwrap(pin, wrap)?;
            let mac = crypto::mac(&self.state, &key)?;
            if !bool::from(mac.as_bytes().ct_eq(self.state.mac.as_bytes())) {
                return Err(Error::InvalidPin);
            }
            Ok(key)
        })();
        match result {
            Ok(key) => {
                self.key = Some(key);
                self.failures = 0;
                Ok(())
            }
            Err(e) => {
                self.key = None;
                self.failures = self.failures.saturating_add(1);
                if self.failures >= 3 {
                    self.retry_at =
                        Instant::now() + Duration::from_secs(2u64.pow(self.failures.min(5)))
                }
                Err(e)
            }
        }
    }
    pub fn change_pin(&mut self, current: &str, next: &str) -> Result<()> {
        self.require_key()?;
        let old = self.state.wrap.as_ref().ok_or(Error::InvalidPin)?;
        let verified = crypto::unwrap(current, old)?;
        if !bool::from(verified.as_ref().ct_eq(self.require_key()?.as_ref())) {
            return Err(Error::InvalidPin);
        }
        let mut state = self.state.clone();
        state.wrap = Some(crypto::wrap(next, self.require_key()?)?);
        self.commit(state)
    }
    pub fn set_close_to_tray(&mut self, value: bool) -> Result<()> {
        self.require_key()?;
        let mut next = self.state.clone();
        next.close_to_tray = value;
        self.commit(next)
    }
    pub fn close_to_tray(&self) -> bool {
        self.state.close_to_tray
    }
    pub fn snapshot(&self) -> Snapshot {
        let open = !self.locked();
        Snapshot {
            initialized: self.initialized(),
            locked: !open,
            projects: if open {
                self.state.projects.clone()
            } else {
                vec![]
            },
            items: if open {
                self.state.items.clone()
            } else {
                vec![]
            },
            clients: if open {
                self.state
                    .clients
                    .iter()
                    .map(|c| ClientInfo {
                        id: c.id.clone(),
                        name: c.name.clone(),
                    })
                    .collect()
            } else {
                vec![]
            },
            grants: if open {
                self.state.grants.clone()
            } else {
                vec![]
            },
            close_to_tray: self.state.close_to_tray,
        }
    }
    pub(crate) fn require_key(&self) -> Result<&[u8; 32]> {
        self.key.as_deref().ok_or(Error::Locked)
    }
    pub(crate) fn commit(&mut self, mut next: State) -> Result<()> {
        next.mac = crypto::mac(&next, self.require_key()?)?;
        let parent = self
            .path
            .parent()
            .ok_or_else(|| Error::Invalid("Invalid vault path.".into()))?;
        fs::create_dir_all(parent)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(parent, fs::Permissions::from_mode(0o700))?;
        }
        let mut temp = tempfile::NamedTempFile::new_in(parent)?;
        temp.write_all(&serde_json::to_vec_pretty(&next)?)?;
        temp.as_file().sync_all()?;
        temp.persist(&self.path).map_err(|e| Error::Io(e.error))?;
        // Rename commits ownership of the new credentials; never roll them back after this point.
        #[cfg(unix)]
        {
            if let Ok(dir) = fs::File::open(parent) {
                let _ = dir.sync_all();
            }
        }
        self.state = next;
        Ok(())
    }
}
