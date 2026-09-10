//! Small startup diagnostic available before Tauri has initialized.
use std::{fs, io::Write, path::PathBuf, sync::{OnceLock, atomic::{AtomicBool, Ordering}}};

static LOG_PATH: OnceLock<PathBuf> = OnceLock::new();
static REPORTED: AtomicBool = AtomicBool::new(false);

pub fn initialize(directory: PathBuf) {
    if fs::create_dir_all(&directory).is_ok() {
        let path = directory.join("startup.log");
        if fs::metadata(&path).map(|m| m.len() > 256 * 1024).unwrap_or(false) {
            let previous = directory.join("startup.previous.log");
            let _ = fs::remove_file(&previous);
            let _ = fs::rename(&path, previous);
        }
        let _ = LOG_PATH.set(path);
    }
    stage("Starting Lanyard desktop");
    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        // Do not persist arbitrary panic payloads: they can contain user data.
        let location = info.location().map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_else(|| "unknown location".into());
        report(&format!("Lanyard encountered an internal error at {location}."));
        previous_hook(info);
    }));
}

pub fn stage(message: &str) {
    eprintln!("{message}");
    if let Some(path) = LOG_PATH.get() {
        if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(path) {
            let time = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs()).unwrap_or_default();
            let _ = writeln!(file, "{time} pid={} {message}", std::process::id());
            let _ = file.flush();
        }
    }
}

pub fn report(message: &str) {
    stage(message);
    if !REPORTED.swap(true, Ordering::SeqCst) {
        let detail = match LOG_PATH.get() {
            Some(path) => format!("{message}\n\nStartup log: {}", path.display()),
            None => format!("{message}\n\nThe startup log could not be created."),
        };
        show_error(&detail);
    }
}

#[cfg(windows)]
fn show_error(message: &str) {
    #[link(name = "user32")]
    extern "system" {
        fn MessageBoxW(hwnd: *mut std::ffi::c_void, text: *const u16, caption: *const u16, kind: u32) -> i32;
    }
    let text: Vec<u16> = message.encode_utf16().chain(Some(0)).collect();
    let caption: Vec<u16> = "Lanyard startup error".encode_utf16().chain(Some(0)).collect();
    // Buffers are NUL-terminated and live for the duration of the synchronous call.
    unsafe { MessageBoxW(std::ptr::null_mut(), text.as_ptr(), caption.as_ptr(), 0x10); }
}
#[cfg(not(windows))]
fn show_error(message: &str) { eprintln!("{message}"); }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn startup_milestones_are_written() {
        let root = std::env::temp_dir().join(format!("lanyard-startup-test-{}", std::process::id()));
        initialize(root.clone());
        stage("Test milestone");
        let log = fs::read_to_string(root.join("startup.log")).unwrap();
        assert!(log.contains("Starting Lanyard desktop"));
        assert!(log.contains("Test milestone"));
        fs::remove_dir_all(root).unwrap();
    }
}
