mod commands;
mod startup;
mod server;
mod state;
use fs2::FileExt;
use state::Desktop;
use std::{
    collections::VecDeque,
    fs,
    sync::{Arc, Mutex},
};
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager,
};
fn setup(app: &mut tauri::App) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let home = dirs::home_dir().ok_or("User home directory unavailable.")?;
    let discovery = home.join(".lanyard");
    fs::create_dir_all(&discovery)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&discovery, fs::Permissions::from_mode(0o700))?;
    }
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(discovery.join("client.lock"))?;
    lock.try_lock_exclusive()
        .map_err(|_| "Lanyard is already running. Open it from the system tray.")?;
    let data = dirs::data_local_dir()
        .ok_or("Local data directory unavailable.")?
        .join("ULIX/Lanyard");
    let vault =
        lanyard_core::Vault::open(data.join("vault.json"), Box::new(lanyard_core::NativeStore))?;
    let legacy = if cfg!(windows) {
        dirs::data_local_dir().map(|p| p.join("LanyardApp/Lanyard"))
    } else if cfg!(target_os = "macos") {
        Some(home.join("Library/Application Support/Lanyard"))
    } else {
        dirs::data_dir().map(|p| p.join("Lanyard"))
    }
    .filter(|p| p.join("meta.json").is_file() && p.join("config.json").is_file());
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    listener.set_nonblocking(true)?;
    let port = listener.local_addr()?.port();
    let instance = uuid::Uuid::new_v4().to_string();
    let desktop = Arc::new(Desktop {
        updating: std::sync::atomic::AtomicBool::new(false),
        vault: Arc::new(Mutex::new(vault)),
        pending: Mutex::new(VecDeque::new()),
        legacy,
        port,
        instance: instance.clone(),
        discovery: discovery.clone(),
        _lock: lock,
    });
    let mut endpoint = tempfile::NamedTempFile::new_in(&discovery)?;
    use std::io::Write;
    endpoint.write_all(&serde_json::to_vec(
        &serde_json::json!({"protocol":2,"port":port,"instance":instance}),
    )?)?;
    endpoint.as_file().sync_all()?;
    endpoint.persist(discovery.join("endpoint.json"))?;
    fs::write(discovery.join("port"), port.to_string())?;
    app.manage(desktop.clone());
    server::start(listener, desktop, app.handle().clone());
    let open = MenuItem::with_id(app, "open", "Open Lanyard", true, None::<&str>)?;
    let lock = MenuItem::with_id(app, "lock", "Lock vault", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Lanyard", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &lock, &quit])?;
    let mut tray = TrayIconBuilder::new()
        .tooltip("Lanyard")
        .menu(&menu)
        .on_menu_event(|app, e| match e.id.as_ref() {
            "open" => state::show(app),
            "lock" => {
                if let Some(s) = app.try_state::<Arc<Desktop>>() {
                    if let Ok(mut v) = s.vault.lock() {
                        v.lock()
                    }
                }
                state::changed(app)
            }
            "quit" => app.exit(0),
            _ => {}
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone())
    }
    tray.build(app)?;
    Ok(())
}
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    startup::initialize(dirs::data_local_dir().unwrap_or_else(std::env::temp_dir).join("ULIX/Lanyard/logs"));
    let config = match serde_json::from_str(include_str!("../update-config.json")) {
        Ok(c) => c,
        Err(e) => {
            startup::report(&format!("Invalid update configuration: {e}"));
            return;
        }
    };
    startup::stage("Initializing desktop plugins");
    let result = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            state::show(app)
        }))
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_ulix_update::native_updater())
        .plugin(tauri_plugin_ulix_update::init_with_hooks(
            config,
            |app| {
                if let Some(s) = app.try_state::<Arc<Desktop>>() {
                    let q = s
                        .pending
                        .lock()
                        .map_err(|_| update_error("Request queue unavailable."))?;
                    if q.iter()
                        .any(|p| p.deadline > std::time::Instant::now() && !p.sender.is_closed())
                    {
                        return Err(update_error(
                            "Resolve pending access requests before updating.",
                        ));
                    }
                    s.updating.store(true, std::sync::atomic::Ordering::SeqCst);
                    drop(q);
                    s.vault
                        .lock()
                        .map_err(|_| update_error("Vault unavailable."))?
                        .lock();
                    state::changed(app);
                }
                Ok(())
            },
            |app| {
                if let Some(s) = app.try_state::<Arc<Desktop>>() {
                    s.updating.store(false, std::sync::atomic::Ordering::SeqCst);
                }
            },
        ))
        .setup(|app| {
            startup::stage("Beginning Lanyard setup");
            if let Err(error) = setup(app) {
                startup::report(&format!("Lanyard setup failed: {error}"));
                return Err(error);
            }
            state::show(app.handle());
            startup::stage("Lanyard setup completed");
            Ok(())
        })
        .on_window_event(|w, e| {
            if w.label() == "main" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = e {
                    if let Some(s) = w.try_state::<Arc<Desktop>>() {
                        if let Ok(v) = s.vault.lock() {
                            if v.close_to_tray() {
                                api.prevent_close();
                                let _ = w.hide();
                            }
                        }
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::snapshot,
            commands::initialize,
            commands::unlock,
            commands::lock,
            commands::change_pin,
            commands::import_legacy,
            commands::save_item,
            commands::reveal,
            commands::delete_item,
            commands::save_project,
            commands::delete_project,
            commands::revoke,
            commands::set_close_to_tray,
            commands::requests,
            commands::respond,
            commands::parse_env,
            commands::generate_keypair,
            commands::copy_value,
            commands::hide_window,
            commands::quit
        ])
        .build(tauri::generate_context!());
    match result {
        Ok(app) => app.run(|app, e| {
            if let tauri::RunEvent::Exit = e {
                if let Some(s) = app.try_state::<Arc<Desktop>>() {
                    let _ = fs::remove_file(s.discovery.join("endpoint.json"));
                    let _ = fs::remove_file(s.discovery.join("port"));
                }
            }
        }),
        Err(e) => startup::report(&format!("Lanyard could not start: {e}")),
    }
}
fn update_error(s: &str) -> ulix_update_core::Error {
    ulix_update_core::Error::Configuration(s.into())
}
