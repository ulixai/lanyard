#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
use fs2::FileExt;
use std::{
    fs,
    io::Read,
    process::Command,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use ulix_update_core::{
    portable::{apply_archive, Plan, RUN_LOCK},
    verify, Error, Result,
};

fn run() -> Result<()> {
    let plan_path = std::env::args_os()
        .nth(1)
        .ok_or_else(|| Error::Configuration("An update plan is required.".into()))?;
    if plan_path == "--recover" {
        let root = std::path::PathBuf::from(
            std::env::args_os()
                .nth(2)
                .ok_or_else(|| Error::Configuration("Specify the portable folder.".into()))?,
        )
        .canonicalize()?;
        let lock = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(root.join(RUN_LOCK))?;
        lock.try_lock_exclusive()?;
        let launch = ulix_update_core::portable::recover(&root)?;
        FileExt::unlock(&lock)?;
        if let Some(launch) = launch {
            Command::new(launch).current_dir(root).spawn()?;
        }
        return Ok(());
    }
    let plan: Plan = serde_json::from_reader(fs::File::open(&plan_path)?.take(512 * 1024))?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| Error::Configuration(e.to_string()))?
        .as_secs();
    let manifest = verify(
        &plan.envelope,
        &plan.config,
        &plan.platform,
        &plan.arch,
        &plan.current_version,
        now,
    )?;
    let artifact = manifest
        .update
        .ok_or_else(|| Error::Verification("No update is authorized.".into()))?;
    let root = plan.root.canonicalize()?;
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(root.join(RUN_LOCK))?;
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        match lock.try_lock_exclusive() {
            Ok(()) => break,
            Err(error)
                if error.kind() == std::io::ErrorKind::WouldBlock && Instant::now() < deadline =>
            {
                std::thread::sleep(Duration::from_millis(100))
            }
            Err(error) => return Err(Error::Io(error)),
        }
    }
    let (launch, backup) = apply_archive(
        &root,
        &plan.archive,
        &plan.config.product,
        &plan.current_version,
        &artifact,
    )?;
    let _ = fs::write(
        root.join("ulix-update-result.json"),
        serde_json::to_vec(
            &serde_json::json!({"version":artifact.version,"backup":backup,"status":"installed"}),
        )?,
    );
    FileExt::unlock(&lock)?;
    if let Err(error) = Command::new(launch).current_dir(&root).spawn() {
        lock.try_lock_exclusive()?;
        let previous = ulix_update_core::portable::recover(&root)?;
        FileExt::unlock(&lock)?;
        if let Some(previous) = previous {
            let _ = Command::new(previous).current_dir(&root).spawn();
        }
        return Err(Error::Io(error));
    }
    let _ = fs::remove_file(plan.archive);
    let _ = fs::remove_file(plan_path);
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        // Preserve a diagnostic beside the plan for manual recovery.
        if let Some(path) = std::env::args_os().nth(1) {
            let path = std::path::PathBuf::from(path).with_extension("error.txt");
            let _ = fs::write(path, error.to_string());
        }
        eprintln!("{error}");
        std::process::exit(1);
    }
}
