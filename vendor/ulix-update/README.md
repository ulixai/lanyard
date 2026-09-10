# ULIX Update

A reusable updater for Tauri 2 desktop applications, with a framework-independent Rust verification/download core and a React UI. Installed updates use Tauri's native installer. Portable updates use a signed manifest, SHA-256 verified ZIP, and a helper that waits for the app to exit before replacing its managed files.

## Drop into a new application

1. Run `python tools/vendor.py /path/to/app` from this module. The app receives a self-contained `vendor/ulix-update` copy; keep this repository as the source of truth and rerun the command when upgrading it.
2. Add `tauri-plugin-ulix-update = { path = "../vendor/ulix-update/crates/tauri-plugin-ulix-update" }` and `ulix-update-core = { path = "../vendor/ulix-update/crates/ulix-update-core" }` to `src-tauri/Cargo.toml`.
3. Add `"@ulix/update-react": "file:vendor/ulix-update/packages/react"` to frontend dependencies and run `npm install`.
4. Add `ulix-update:default` to the main window's Tauri capability. Keep it off remote webviews.
5. Copy an app's `src-tauri/update-config.json`, change `product` to a permanent lowercase slug, and configure your public keys using `tools/configure.py`.
6. Register the native updater, then the ULIX plugin, once on the application builder, then render `<UpdateButton productName="Your app" />`.

```rust
let config = serde_json::from_str(include_str!("../update-config.json"))?;
let builder = tauri::Builder::default()
    .plugin(tauri_plugin_ulix_update::native_updater())
    .plugin(tauri_plugin_ulix_update::init_with_hooks(
        config,
        |app| { /* stop accepting work; flush durable state */ Ok(()) },
        |app| { /* resume if installation fails */ },
    ));
```

For unsigned local development, set `plugins.updater.pubkey` to `""` in
`tauri.conf.json` and leave `bundle.createUpdaterArtifacts` false. The native
plugin needs this configuration to initialize even before signing is configured.
Run `tools/configure.py` with real public keys before release; it enables signed
artifacts and configures both key locations. Never register a plugin from another
plugin's setup callback, which would recursively acquire Tauri's plugin lock.

`before_install` runs in Rust at the installation boundary. It must reject unsafe restart conditions and stop owned background processes. `on_install_error` releases the gate after a failed install. The React `beforeInstall` callback lets your app await draft/history saves; `blockedReason` explains temporary restrictions. These UI checks complement the native guard.

For small windows, call `openUpdateWindow()` and render `<UpdatePanel productName="Your app" />` when `location.search` contains `ulix-update`. Add a separate capability for the `ulix-update` window. Transfr demonstrates this pattern.

The UI supports checks, release notes, verified download progress, installation, and retry. Opening it checks immediately. Its colors use isolated `--uu-bg`, `--uu-fg`, `--uu-muted`, `--uu-accent`, and `--uu-border` variables.

## Protocol and installation

`GET /api/updates/{product}/{platform}/{arch}/{currentVersion}?channel=stable&edition=installed` returns an Ed25519 envelope. Verify the exact payload string before JSON parsing, then check expiry and the entire product/channel/platform/architecture/edition/current-version scope. HTTPS redirects stay HTTPS. A release must have a strictly newer SemVer precedence; build metadata alone is not an update.

The manifest contains artifact URL, byte count, SHA-256, notes, and a Tauri signature. Downloads are streamed to a private temporary file and checked before installation. Installed artifacts also pass Minisign verification. A fresh signed manifest must still authorize the same immutable release at installation time, so withdrawn releases cannot be installed from stale UI state.

Installed Windows uses NSIS; macOS uses a signed app update archive; Linux uses AppImage. Use `tools/install-appimage.sh` for a per-user installation that can update itself. DEB/RPM package-manager installations need their package manager and are outside this install adapter.

Portable installations contain `ulix-portable.json`, a matching helper executable, and named managed app/runtime files. `models`, `data`, and `userdata` are protected. The app and helper share an exclusive lock. Extraction rejects traversal, undeclared files, duplicate names, symlinks, and oversized archives. Files are staged on the same filesystem; the old app is moved into a retained backup. A durable journal supports interrupted-update recovery. If the helper cannot start the new executable, it restores and restarts the previous one.

After an interrupted handoff, close the application and run `ulix-update-helper --recover /path/to/portable-folder`. If the helper itself is in the retained backup, run that copy. Successful app startup retires the transaction; backups remain for manual recovery. Automatic health-based rollback after an app launches but later crashes is not implemented. Portable ZIPs must contain ordinary files, so flatten runtime library symlinks **before** signing the app bundle. Bundles requiring symlinked frameworks need a different packaging adapter.

The Rust core is usable outside Tauri; installed-app adapters and the React component are Tauri-specific. Other frameworks can adopt the signed protocol and provide their own install adapter without changing the website route.

See `RELEASES.md` for keys, website deployment, publication, and platform validation.
