use crate::{now, Candidate, Service, Staged};
use serde::Serialize;
use std::{fs, path::PathBuf};
use tauri::{AppHandle, Emitter, Manager, Runtime, State};
use tauri_plugin_updater::UpdaterExt;
use ulix_update_core::{portable::Plan, verify, Error, Result};

fn busy() -> Error {
    Error::Configuration("An update operation is already running.".into())
}
fn other(error: impl std::fmt::Display) -> Error {
    Error::Configuration(error.to_string())
}

#[derive(Serialize)]
pub struct Status {
    product: String,
    current_version: String,
    edition: String,
    update: Option<ulix_update_core::Artifact>,
}

#[tauri::command]
pub async fn check<R: Runtime>(
    _app: AppHandle<R>,
    service: State<'_, Service<R>>,
) -> Result<Status> {
    let mut state = service.state.try_lock().map_err(|_| busy())?;
    state.candidate = None;
    state.staged = None;
    let envelope = ulix_update_core::check(
        &service.config,
        &service.platform,
        &service.arch,
        &service.current,
    )
    .await?;
    let manifest = verify(
        &envelope,
        &service.config,
        &service.platform,
        &service.arch,
        &service.current,
        now()?,
    )?;
    let update = manifest.update;
    if let Some(artifact) = &update {
        state.candidate = Some(Candidate {
            envelope,
            artifact: artifact.clone(),
        });
    }
    Ok(Status {
        product: service.config.product.clone(),
        current_version: service.current.clone(),
        edition: service.config.edition.clone(),
        update,
    })
}

#[tauri::command]
pub async fn download<R: Runtime>(app: AppHandle<R>, service: State<'_, Service<R>>) -> Result<()> {
    let mut state = service.state.try_lock().map_err(|_| busy())?;
    state.staged = None;
    let candidate = state
        .candidate
        .as_ref()
        .ok_or_else(|| other("Check for an update first."))?;
    verify(
        &candidate.envelope,
        &service.config,
        &service.platform,
        &service.arch,
        &service.current,
        now()?,
    )?;
    let directory = tempfile::Builder::new().prefix("ulix-update-").tempdir()?;
    let path = directory.path().join("artifact");
    let update = if candidate.artifact.kind == "tauri" {
        if service.config.artifact_public_key.trim().is_empty() {
            return Err(other("This build has no artifact verification key. Configure release signing before building."));
        }
        let mut url = service
            .config
            .url(&service.platform, &service.arch, &service.current)?;
        url.query_pairs_mut()
            .append_pair("format", "tauri")
            .append_pair("release", &candidate.artifact.id);
        let updater = app
            .updater_builder()
            .pubkey(&service.config.artifact_public_key)
            .endpoints(vec![url])
            .map_err(other)?
            .build()
            .map_err(other)?;
        let update = updater
            .check()
            .await
            .map_err(other)?
            .ok_or_else(|| other("This release was withdrawn. Check again."))?;
        if update.version != candidate.artifact.version
            || update.download_url.as_str() != candidate.artifact.url
        {
            return Err(other("The release changed. Check again."));
        }
        Some(update)
    } else {
        None
    };
    ulix_update_core::download(&candidate.artifact, &path, |downloaded, total| {
        let _ = app.emit(
            "ulix-update-progress",
            serde_json::json!({ "downloaded": downloaded, "total": total }),
        );
    })
    .await?;
    if update.is_some() {
        let bytes = tokio::fs::read(&path).await?;
        ulix_update_core::verify_tauri_signature(
            &service.config.artifact_public_key,
            &candidate.artifact.signature,
            &bytes,
        )?;
    }
    state.staged = Some(Staged { directory, update });
    Ok(())
}

#[tauri::command]
pub async fn install<R: Runtime>(app: AppHandle<R>, service: State<'_, Service<R>>) -> Result<()> {
    let mut prepared = false;
    let result = install_inner(app.clone(), &service, &mut prepared).await;
    if result.is_err() && prepared {
        (service.on_install_error)(&app);
    }
    result
}
async fn install_inner<R: Runtime>(app: AppHandle<R>, service: &Service<R>, prepared: &mut bool) -> Result<()> {
    let mut state = service.state.try_lock().map_err(|_| busy())?;
    let candidate = state
        .candidate
        .as_ref()
        .ok_or_else(|| other("Check for an update first."))?;
    let envelope = ulix_update_core::check(
        &service.config,
        &service.platform,
        &service.arch,
        &service.current,
    )
    .await?;
    let fresh = verify(
        &envelope,
        &service.config,
        &service.platform,
        &service.arch,
        &service.current,
        now()?,
    )?;
    let artifact = fresh
        .update
        .ok_or_else(|| other("This release was withdrawn. Check again."))?;
    if artifact.id != candidate.artifact.id
        || artifact.sha256 != candidate.artifact.sha256
        || artifact.version != candidate.artifact.version
    {
        return Err(other(
            "The available release changed. Check again before installing.",
        ));
    }
    let staged = state
        .staged
        .as_ref()
        .ok_or_else(|| other("Download the update first."))?;
    let artifact_path = staged.directory.path().join("artifact");
    // Recheck the staged bytes at the installation boundary.
    ulix_update_core::portable::verify_archive(&artifact_path, &candidate.artifact)?;
    if let Some(update) = &staged.update {
        let bytes = tokio::fs::read(&artifact_path).await?;
        ulix_update_core::verify_tauri_signature(
            &service.config.artifact_public_key,
            &candidate.artifact.signature,
            &bytes,
        )?;
        *prepared = true;
        (service.before_install)(&app)?;
        update.install(bytes).map_err(other)?;
        app.restart();
    } else {
        let root = service
            .portable_root
            .as_ref()
            .ok_or_else(|| other("Portable installation root is missing."))?;
        let helper_name = if cfg!(windows) {
            "ulix-update-helper.exe"
        } else {
            "ulix-update-helper"
        };
        let helper_source = root.join(helper_name);
        if !helper_source.is_file() {
            return Err(other(
                "The portable update helper is missing. Restore it from the original package.",
            ));
        }
        let helper_path = staged.directory.path().join(helper_name);
        fs::copy(&helper_source, &helper_path)?;
        let plan = Plan {
            root: root.clone(),
            archive: artifact_path,
            config: service.config.clone(),
            platform: service.platform.clone(),
            arch: service.arch.clone(),
            current_version: service.current.clone(),
            envelope,
        };
        let plan_path = staged.directory.path().join("plan.json");
        fs::write(&plan_path, serde_json::to_vec(&plan)?)?;
        *prepared = true;
        (service.before_install)(&app)?;
        std::process::Command::new(helper_path)
            .arg(plan_path)
            .spawn()?;
        if let Some(staged) = state.staged.take() {
            let _ = staged.directory.keep();
        }
        app.exit(0);
    }
    Ok(())
}

#[tauri::command]
pub async fn discard<R: Runtime>(_app: AppHandle<R>, service: State<'_, Service<R>>) -> Result<()> {
    let mut state = service.state.try_lock().map_err(|_| busy())?;
    state.staged = None;
    state.candidate = None;
    Ok(())
}

#[tauri::command]
pub fn open_window<R: Runtime>(app: AppHandle<R>) -> Result<()> {
    if let Some(window) = app.get_webview_window("ulix-update") {
        window.show().map_err(other)?;
        window.set_focus().map_err(other)?;
        return Ok(());
    }
    tauri::WebviewWindowBuilder::new(
        &app,
        "ulix-update",
        tauri::WebviewUrl::App(PathBuf::from("index.html?ulix-update=1")),
    )
    .title("Software updates")
    .inner_size(540.0, 560.0)
    .min_inner_size(400.0, 400.0)
    .build()
    .map_err(other)?;
    Ok(())
}
