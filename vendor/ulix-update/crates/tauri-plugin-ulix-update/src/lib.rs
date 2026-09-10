mod commands;
use std::{fs, sync::Arc};
use tauri::{
    plugin::{Builder, TauriPlugin},
    AppHandle, Manager, Runtime,
};
use ulix_update_core::{Artifact, Config, Envelope, Error, Result};

pub use ulix_update_core::Config as UpdateConfig;
pub type BeforeInstall<R> = Arc<dyn Fn(&AppHandle<R>) -> Result<()> + Send + Sync>;

pub(crate) struct Candidate {
    envelope: Envelope,
    artifact: Artifact,
}
pub(crate) struct Staged {
    directory: tempfile::TempDir,
    update: Option<tauri_plugin_updater::Update>,
}
pub(crate) struct Coordinator {
    candidate: Option<Candidate>,
    staged: Option<Staged>,
}
pub(crate) struct Service<R: Runtime> {
    config: Config,
    current: String,
    platform: String,
    arch: String,
    portable_root: Option<std::path::PathBuf>,
    _portable_lock: Option<fs::File>,
    before_install: BeforeInstall<R>,
    on_install_error: Arc<dyn Fn(&AppHandle<R>) + Send + Sync>,
    state: tokio::sync::Mutex<Coordinator>,
}

pub fn portable_root() -> Result<Option<std::path::PathBuf>> {
    let executable = std::env::var_os("APPIMAGE")
        .map(std::path::PathBuf::from)
        .map(Ok)
        .unwrap_or_else(std::env::current_exe)?;
    for parent in executable.ancestors().skip(1).take(5) {
        if parent.join(ulix_update_core::portable::MARKER).is_file() {
            return Ok(Some(parent.canonicalize()?));
        }
    }
    Ok(None)
}

/// Register this on the app builder BEFORE the ULIX plugin.
/// Never register plugins from a plugin setup callback: Tauri holds its plugin lock.
pub fn native_updater<R: Runtime>() -> TauriPlugin<R, tauri_plugin_updater::Config> {
    tauri_plugin_updater::Builder::new().build()
}

pub fn init<R: Runtime>(
    config: Config,
    before_install: impl Fn(&AppHandle<R>) -> Result<()> + Send + Sync + 'static,
) -> TauriPlugin<R> {
    init_with_hooks(config, before_install, |_| {})
}
pub fn init_with_hooks<R: Runtime>(
    mut config: Config,
    before_install: impl Fn(&AppHandle<R>) -> Result<()> + Send + Sync + 'static,
    on_install_error: impl Fn(&AppHandle<R>) + Send + Sync + 'static,
) -> TauriPlugin<R> {
    Builder::new("ulix-update")
        .setup(move |app, _| {
            let root = portable_root()?;
            let lock = if let Some(root) = &root {
                use fs2::FileExt;
                let package = ulix_update_core::portable::read_package(root)?;
                if package.product != config.product
                    || package.version != app.package_info().version.to_string()
                {
                    return Err(Box::new(Error::Configuration(
                        "Portable package does not match this application.".into(),
                    )));
                }
                config.edition = "portable".into();
                let file = fs::OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create(true)
                    .truncate(false)
                    .open(root.join(ulix_update_core::portable::RUN_LOCK))?;
                file.try_lock_exclusive().map_err(|_| {
                    Error::Configuration(
                        "This portable application is already running or updating.".into(),
                    )
                })?;
                ulix_update_core::portable::finish_update(
                    root,
                    &config.product,
                    &app.package_info().version.to_string(),
                )?;
                Some(file)
            } else {
                None
            };
            app.manage(Service {
                config,
                current: app.package_info().version.to_string(),
                platform: if cfg!(target_os = "macos") {
                    "darwin"
                } else {
                    std::env::consts::OS
                }
                .into(),
                arch: std::env::consts::ARCH.into(),
                portable_root: root,
                _portable_lock: lock,
                before_install: Arc::new(before_install),
                on_install_error: Arc::new(on_install_error),
                state: tokio::sync::Mutex::new(Coordinator {
                    candidate: None,
                    staged: None,
                }),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::check,
            commands::download,
            commands::install,
            commands::discard,
            commands::open_window
        ])
        .build()
}

pub(crate) fn now() -> Result<u64> {
    Ok(std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| Error::Configuration(e.to_string()))?
        .as_secs())
}

#[cfg(test)]
mod startup_tests {
    use super::*;
    use std::{sync::mpsc, time::Duration};
    use tauri_plugin_updater::UpdaterExt;

    #[test]
    fn plugins_initialize_without_recursive_registration() {
        let (send, receive) = mpsc::channel();
        std::thread::spawn(move || {
            let mut context = tauri::test::mock_context(tauri::test::noop_assets());
            context.config_mut().plugins.0.insert("updater".into(), serde_json::json!({"pubkey": ""}));
            let config: Config = serde_json::from_value(serde_json::json!({
                "product": "startup-test", "endpoint": "https://www.ulix.ai/api/updates",
                "manifest_public_key": "", "artifact_public_key": "",
                "channel": "stable", "edition": "installed"
            })).unwrap();
            let result = tauri::test::mock_builder()
                .plugin(native_updater())
                .plugin(init(config, |_| Ok(())))
                .build(context)
                .map(|app| { let _ = app.updater_builder(); })
                .map_err(|error| error.to_string());
            let _ = send.send(result);
        });
        receive.recv_timeout(Duration::from_secs(10))
            .expect("Plugin initialization stalled before application setup")
            .expect("Plugins should initialize for an unsigned local build");
    }
}
