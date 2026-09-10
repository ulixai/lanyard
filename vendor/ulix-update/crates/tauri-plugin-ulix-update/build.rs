fn main() {
    tauri_plugin::Builder::new(&["check", "download", "install", "discard", "open_window"]).build();
}
