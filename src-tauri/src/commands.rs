use crate::state::{changed, work, AccessReply, Desktop, RequestInfo};
use lanyard_core::{Error, Fields, ItemInput, Result, Snapshot};
use std::{sync::Arc, time::Instant};
use tauri::{AppHandle, State};
type AppState<'a> = State<'a, Arc<Desktop>>;
#[derive(serde::Serialize)]
pub struct AppSnapshot {
    #[serde(flatten)]
    vault: Snapshot,
    legacy_available: bool,
    port: u16,
}
#[tauri::command]
pub async fn snapshot(state: AppState<'_>) -> Result<AppSnapshot> {
    let vault = work(&state, |v| Ok(v.snapshot())).await?;
    Ok(AppSnapshot {
        vault,
        legacy_available: state.legacy.is_some(),
        port: state.port,
    })
}
#[tauri::command]
pub async fn initialize(app: AppHandle, state: AppState<'_>, pin: String) -> Result<()> {
    work(&state, move |v| v.initialize(&pin)).await?;
    changed(&app);
    Ok(())
}
#[tauri::command]
pub async fn unlock(app: AppHandle, state: AppState<'_>, pin: String) -> Result<()> {
    work(&state, move |v| v.unlock(&pin)).await?;
    changed(&app);
    Ok(())
}
#[tauri::command]
pub async fn lock(app: AppHandle, state: AppState<'_>) -> Result<()> {
    work(&state, |v| {
        v.lock();
        Ok(())
    })
    .await?;
    changed(&app);
    Ok(())
}
#[tauri::command]
pub async fn change_pin(state: AppState<'_>, current: String, next: String) -> Result<()> {
    work(&state, move |v| v.change_pin(&current, &next)).await
}
#[tauri::command]
pub async fn import_legacy(
    app: AppHandle,
    state: AppState<'_>,
    old_pin: String,
    new_pin: String,
    skip_missing: Option<bool>,
) -> Result<lanyard_core::legacy::ImportReport> {
    let path = state
        .legacy
        .clone()
        .ok_or_else(|| Error::Invalid("No legacy vault was found.".into()))?;
    let desktop = state.inner().clone();
    let n = work(&state, move |v| {
        if desktop.updating.load(std::sync::atomic::Ordering::Acquire) {
            return Err(Error::Invalid("An update is being installed. Try importing after restart.".into()));
        }
        v.import_legacy_with_options(
            &path,
            &old_pin,
            &new_pin,
            &lanyard_core::legacy::NativeLegacyReader,
            skip_missing.unwrap_or(false),
        )
    })
    .await?;
    changed(&app);
    Ok(n)
}
#[tauri::command]
pub async fn save_item(app: AppHandle, state: AppState<'_>, input: ItemInput) -> Result<String> {
    let id = work(&state, move |v| v.save_item(input)).await?;
    changed(&app);
    Ok(id)
}
#[tauri::command]
pub async fn reveal(state: AppState<'_>, id: String) -> Result<Fields> {
    work(&state, move |v| v.reveal(&id)).await
}
#[tauri::command]
pub async fn delete_item(app: AppHandle, state: AppState<'_>, id: String) -> Result<()> {
    work(&state, move |v| v.delete_item(&id)).await?;
    changed(&app);
    Ok(())
}
#[tauri::command]
pub async fn save_project(
    app: AppHandle,
    state: AppState<'_>,
    id: Option<String>,
    title: String,
    description: String,
) -> Result<String> {
    let id = work(&state, move |v| v.save_project(id, &title, &description)).await?;
    changed(&app);
    Ok(id)
}
#[tauri::command]
pub async fn delete_project(app: AppHandle, state: AppState<'_>, id: String) -> Result<()> {
    work(&state, move |v| v.delete_project(&id)).await?;
    changed(&app);
    Ok(())
}
#[tauri::command]
pub async fn revoke(
    app: AppHandle,
    state: AppState<'_>,
    client_id: String,
    item_id: Option<String>,
) -> Result<()> {
    work(&state, move |v| v.revoke(&client_id, item_id.as_deref())).await?;
    changed(&app);
    Ok(())
}
#[tauri::command]
pub async fn set_close_to_tray(app: AppHandle, state: AppState<'_>, value: bool) -> Result<()> {
    work(&state, move |v| v.set_close_to_tray(value)).await?;
    changed(&app);
    Ok(())
}
#[tauri::command]
pub fn requests(state: AppState<'_>) -> Result<Vec<RequestInfo>> {
    state.requests()
}
#[tauri::command]
pub async fn respond(
    app: AppHandle,
    state: AppState<'_>,
    request_id: String,
    target_id: Option<String>,
    always: bool,
) -> Result<()> {
    let p = {
        let mut q = state
            .pending
            .lock()
            .map_err(|_| Error::Storage("Request queue unavailable.".into()))?;
        let i = q
            .iter()
            .position(|p| p.id == request_id)
            .ok_or_else(|| Error::Invalid("This request has expired.".into()))?;
        q.remove(i).ok_or(Error::NotFound)?
    };
    if p.deadline <= Instant::now() || p.sender.is_closed() {
        changed(&app);
        return Err(Error::Invalid("This request has expired.".into()));
    }
    let result = if let Some(id) = target_id {
        let auth = p.input.auth;
        let expected = p.input.target_id;
        let category = p.input.category;
        let deadline = p.deadline;
        work(&state, move |v| {
            if Instant::now() >= deadline {
                return Err(Error::Denied);
            }
            if expected.as_ref().is_some_and(|t| t != &id) || !v.item_matches(&id, category)? {
                return Err(Error::Denied);
            }
            let data = v.reveal(&id)?;
            if Instant::now() >= deadline {
                return Err(Error::Denied);
            }
            v.approve(&auth, &id, always)?;
            Ok(AccessReply {
                status: "success",
                target_id: id,
                data,
            })
        })
        .await
    } else {
        Err(Error::Denied)
    };
    let error = match &result {
        Err(Error::Denied) => None,
        Err(e) => Some(e.to_string()),
        _ => None,
    };
    let _ = p.sender.send(result);
    changed(&app);
    if let Some(e) = error {
        return Err(Error::Invalid(e));
    }
    Ok(())
}
#[tauri::command]
pub async fn parse_env(state: AppState<'_>, text: String) -> Result<Fields> {
    work(&state, move |v| {
        if v.locked() {
            return Err(Error::Locked);
        }
        lanyard_core::utilities::parse_env(&text)
    })
    .await
}
#[tauri::command]
pub async fn generate_keypair(state: AppState<'_>, algorithm: String) -> Result<Fields> {
    work(&state, |v| {
        if v.locked() {
            Err(Error::Locked)
        } else {
            Ok(())
        }
    })
    .await?;
    let fields = tauri::async_runtime::spawn_blocking(move || {
        lanyard_core::utilities::generate_keypair(&algorithm)
    })
    .await
    .map_err(|e| Error::Storage(e.to_string()))??;
    work(&state, move |v| {
        if v.locked() {
            Err(Error::Locked)
        } else {
            Ok(fields)
        }
    })
    .await
}
#[tauri::command]
pub fn hide_window(app: AppHandle) -> Result<()> {
    use tauri::Manager;
    if let Some(w) = app.get_webview_window("main") {
        w.hide().map_err(|e| Error::Storage(e.to_string()))?
    }
    Ok(())
}
#[tauri::command]
pub fn quit(app: AppHandle) {
    app.exit(0)
}
#[tauri::command]
pub async fn copy_value(app: AppHandle, state: AppState<'_>, value: String) -> Result<()> {
    work(&state, |v| {
        if v.locked() {
            Err(Error::Locked)
        } else {
            Ok(())
        }
    })
    .await?;
    if value.len() > 1024 * 1024 {
        return Err(Error::Invalid("Clipboard value is too large.".into()));
    }
    use tauri_plugin_clipboard_manager::ClipboardExt;
    app.clipboard()
        .write_text(&value)
        .map_err(|e| Error::Storage(e.to_string()))?;
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(30)).await;
        if app.clipboard().read_text().ok().as_deref() == Some(value.as_str()) {
            let _ = app.clipboard().clear();
        }
    });
    Ok(())
}
