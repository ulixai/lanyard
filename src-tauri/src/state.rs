use lanyard_core::{Category, ClientAuth, Fields, Result, Vault};
use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::Instant,
};
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::oneshot;
#[derive(Deserialize)]
pub struct RequestInput {
    #[serde(flatten)]
    pub auth: ClientAuth,
    pub target_id: Option<String>,
    pub category: Option<Category>,
    pub reason: Option<String>,
    #[serde(default = "timeout")]
    pub timeout: u64,
}
fn timeout() -> u64 {
    300
}
#[derive(Serialize, Clone)]
pub struct AccessReply {
    pub status: &'static str,
    pub target_id: String,
    pub data: Fields,
}
#[derive(Serialize, Clone)]
pub struct RequestInfo {
    pub id: String,
    pub client_id: String,
    pub app_name: String,
    pub target_id: Option<String>,
    pub category: Option<Category>,
    pub reason: Option<String>,
    pub paired: bool,
    pub expires_in: u64,
}
pub struct Pending {
    pub id: String,
    pub input: RequestInput,
    pub deadline: Instant,
    pub paired: bool,
    pub sender: oneshot::Sender<Result<AccessReply>>,
}
pub struct Desktop {
    pub updating: std::sync::atomic::AtomicBool,
    pub vault: Arc<Mutex<Vault>>,
    pub pending: Mutex<VecDeque<Pending>>,
    pub legacy: Option<std::path::PathBuf>,
    pub port: u16,
    pub instance: String,
    pub discovery: std::path::PathBuf,
    pub _lock: std::fs::File,
}
impl Desktop {
    pub fn requests(&self) -> Result<Vec<RequestInfo>> {
        let mut queue = self
            .pending
            .lock()
            .map_err(|_| lanyard_core::Error::Storage("Request queue unavailable.".into()))?;
        queue.retain(|p| !p.sender.is_closed() && p.deadline > Instant::now());
        Ok(queue
            .iter()
            .map(|p| RequestInfo {
                id: p.id.clone(),
                client_id: p.input.auth.client_id.clone(),
                app_name: p.input.auth.app_name.clone(),
                target_id: p.input.target_id.clone(),
                category: p.input.category,
                reason: p.input.reason.clone(),
                paired: p.paired,
                expires_in: p
                    .deadline
                    .saturating_duration_since(Instant::now())
                    .as_secs(),
            })
            .collect())
    }
}
pub fn changed(app: &AppHandle) {
    let _ = app.emit_to("main", "lanyard-changed", ());
}
pub fn show(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}
pub async fn work<T: Send + 'static>(
    desktop: &Arc<Desktop>,
    f: impl FnOnce(&mut Vault) -> Result<T> + Send + 'static,
) -> Result<T> {
    let vault = desktop.vault.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut vault = vault.lock().map_err(|_| {
            lanyard_core::Error::Storage("Vault is unavailable. Restart Lanyard.".into())
        })?;
        f(&mut vault)
    })
    .await
    .map_err(|e| lanyard_core::Error::Storage(e.to_string()))?
}
