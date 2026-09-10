use crate::{Artifact, Config, Envelope, Error, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};

pub const MARKER: &str = "ulix-portable.json";
pub const RUN_LOCK: &str = ".ulix-running.lock";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    pub schema: u32,
    pub product: String,
    pub version: String,
    pub launch: String,
    pub managed: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Plan {
    pub root: PathBuf,
    pub archive: PathBuf,
    pub config: Config,
    pub platform: String,
    pub arch: String,
    pub current_version: String,
    pub envelope: Envelope,
}

fn bad(message: &str) -> Error {
    Error::Verification(message.into())
}

pub fn safe_relative(value: &str) -> Result<PathBuf> {
    let path = Path::new(value);
    if value.is_empty()
        || value.contains('\\')
        || value.contains(':')
        || value.contains('\0')
        || path.is_absolute()
    {
        return Err(bad("Unsafe archive path."));
    }
    for component in path.components() {
        let Component::Normal(part) = component else {
            return Err(bad("Archive path escapes the application."));
        };
        let name = part.to_string_lossy();
        if name.ends_with('.') || name.ends_with(' ') || name.starts_with('.') {
            return Err(bad("Reserved archive path."));
        }
        let stem = name.split('.').next().unwrap_or("").to_ascii_uppercase();
        if ["CON", "PRN", "AUX", "NUL"].contains(&stem.as_str())
            || (stem.len() == 4
                && (stem.starts_with("COM") || stem.starts_with("LPT"))
                && stem.as_bytes()[3].is_ascii_digit())
        {
            return Err(bad("Reserved Windows archive name."));
        }
    }
    Ok(path.to_path_buf())
}

pub fn read_package(root: &Path) -> Result<Package> {
    let file = fs::File::open(root.join(MARKER))?;
    let package: Package = serde_json::from_reader(file.take(64 * 1024))?;
    validate_package(&package)?;
    Ok(package)
}

fn validate_package(package: &Package) -> Result<()> {
    if package.schema != 1 || package.managed.is_empty() || package.managed.len() > 64 {
        return Err(bad("Invalid portable package descriptor."));
    }
    semver::Version::parse(&package.version).map_err(|_| bad("Invalid portable version."))?;
    let mut names = HashSet::new();
    for item in &package.managed {
        if safe_relative(item)?.components().count() != 1
            || ["models", "data", "userdata", MARKER].contains(&item.to_ascii_lowercase().as_str())
            || !names.insert(item.to_ascii_lowercase())
        {
            return Err(bad(
                "Package attempts to replace protected or duplicate content.",
            ));
        }
    }
    let launch = safe_relative(&package.launch)?;
    let first = launch
        .components()
        .next()
        .ok_or_else(|| bad("Missing launch path."))?
        .as_os_str()
        .to_string_lossy()
        .to_string();
    if !package.managed.contains(&first) {
        return Err(bad("Launch path is not managed by the package."));
    }
    Ok(())
}

pub fn verify_archive(archive: &Path, artifact: &Artifact) -> Result<()> {
    let mut file = fs::File::open(archive)?;
    if file.metadata()?.len() != artifact.size {
        return Err(bad("Portable archive size mismatch."));
    }
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let length = file.read(&mut buffer)?;
        if length == 0 {
            break;
        }
        hash.update(&buffer[..length]);
    }
    if hex::encode(hash.finalize()) != artifact.sha256.to_lowercase() {
        return Err(bad("Portable archive integrity check failed."));
    }
    Ok(())
}

fn extract(archive: &Path, destination: &Path) -> Result<Package> {
    let mut zip = zip::ZipArchive::new(fs::File::open(archive)?)?;
    if zip.len() > 30000 {
        return Err(bad("Portable archive has too many entries."));
    }
    let mut total = 0u64;
    let mut names = HashSet::new();
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index)?;
        let name = entry.name().trim_end_matches('/');
        let relative = safe_relative(name)?;
        if !names.insert(name.to_lowercase()) {
            return Err(bad("Duplicate portable archive entry."));
        }
        total = total
            .checked_add(entry.size())
            .ok_or_else(|| bad("Archive is too large."))?;
        if total > 8 * 1024 * 1024 * 1024
            || entry
                .unix_mode()
                .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            return Err(bad("Archive exceeds limits or contains symbolic links."));
        }
        let path = destination.join(&relative);
        if entry.is_dir() {
            fs::create_dir_all(&path)?;
            continue;
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        let count = std::io::copy(&mut entry, &mut file)?;
        if count != entry.size() {
            return Err(bad("Incomplete archive entry."));
        }
        file.flush()?;
        file.sync_all()?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(
                &path,
                fs::Permissions::from_mode(entry.unix_mode().unwrap_or(0o644) & 0o777),
            )?;
        }
    }
    let package = read_package(destination)?;
    for child in fs::read_dir(destination)? {
        let name = child?.file_name().to_string_lossy().to_string();
        if name != MARKER && !package.managed.contains(&name) {
            return Err(bad("Archive contains undeclared content."));
        }
    }
    for name in &package.managed {
        if !destination.join(name).exists() {
            return Err(bad("A managed package entry is missing."));
        }
    }
    if !destination.join(&package.launch).is_file() {
        return Err(bad("Portable executable is missing."));
    }
    Ok(package)
}

pub const JOURNAL: &str = ".ulix-update-journal.json";
#[derive(Serialize, Deserialize)]
struct Journal {
    schema: u32,
    stage: String,
    backup: String,
    old: Package,
    new: Package,
}
fn write_journal(root: &Path, journal: &Journal) -> Result<()> {
    let mut temp = tempfile::NamedTempFile::new_in(root)?;
    temp.write_all(&serde_json::to_vec(journal)?)?;
    temp.as_file().sync_all()?;
    temp.persist(root.join(JOURNAL))
        .map_err(|e| Error::Io(e.error))?;
    #[cfg(unix)]
    fs::File::open(root)?.sync_all()?;
    Ok(())
}
fn read_journal(root: &Path) -> Result<Journal> {
    let j: Journal = serde_json::from_reader(fs::File::open(root.join(JOURNAL))?.take(256 * 1024))?;
    validate_package(&j.old)?;
    validate_package(&j.new)?;
    for (name, prefix) in [(&j.stage, ".ulix-stage-"), (&j.backup, ".ulix-backup-")] {
        if !name.starts_with(prefix) || name.contains(['/', '\\', ':']) || name.len() > 100 {
            return Err(bad("Invalid recovery directory."));
        }
        let metadata = fs::symlink_metadata(root.join(name))?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(bad("Invalid recovery directory."));
        }
    }
    if j.schema != 1 || j.old.product != j.new.product {
        return Err(bad("Invalid recovery journal."));
    }
    Ok(j)
}
/// Undo an interrupted update under RUN_LOCK. Moving replacements back to staging
/// makes recovery idempotent if recovery itself is interrupted.
pub fn recover(root: &Path) -> Result<Option<PathBuf>> {
    let root = root.canonicalize()?;
    if !root.join(JOURNAL).exists() {
        return Ok(None);
    }
    let j = read_journal(&root)?;
    let stage = root.join(&j.stage);
    let backup = root.join(&j.backup);
    for name in j
        .new
        .managed
        .iter()
        .chain(std::iter::once(&MARKER.to_string()))
    {
        if !stage.join(name).try_exists()? && root.join(name).try_exists()? {
            fs::rename(root.join(name), stage.join(name))?;
        }
    }
    for name in j
        .old
        .managed
        .iter()
        .chain(std::iter::once(&MARKER.to_string()))
    {
        if backup.join(name).try_exists()? {
            fs::rename(backup.join(name), root.join(name))?;
        }
    }
    fs::remove_file(root.join(JOURNAL))?;
    let _ = fs::remove_dir_all(stage);
    let _ = fs::remove_dir(backup);
    Ok(Some(root.join(j.old.launch)))
}
/// The newly started app retires its journal after taking RUN_LOCK; backup stays.
pub fn finish_update(root: &Path, product: &str, version: &str) -> Result<()> {
    if !root.join(JOURNAL).exists() {
        return Ok(());
    }
    let j = read_journal(root)?;
    if j.new.product != product || j.new.version != version {
        return Err(bad("An interrupted update requires recovery. Run ulix-update-helper --recover with the portable folder."));
    }
    let actual = read_package(root)?;
    if actual.version != version || actual.product != product {
        return Err(bad("Portable recovery identity mismatch."));
    }
    fs::remove_file(root.join(JOURNAL))?;
    let _ = fs::remove_dir_all(root.join(j.stage));
    Ok(())
}
/// Caller must hold RUN_LOCK. Returns new executable and retained backup.
pub fn apply_archive(
    root: &Path,
    archive: &Path,
    product: &str,
    current: &str,
    artifact: &Artifact,
) -> Result<(PathBuf, PathBuf)> {
    let root = root.canonicalize()?;
    if root.join(JOURNAL).exists() {
        recover(&root)?;
    }
    let old = read_package(&root)?;
    if old.product != product || old.version != current {
        return Err(bad("Portable installation changed since the update check."));
    }
    verify_archive(archive, artifact)?;
    let stage = tempfile::Builder::new()
        .prefix(".ulix-stage-")
        .tempdir_in(&root)?;
    let new = extract(archive, stage.path())?;
    if new.product != product || new.version != artifact.version {
        return Err(bad(
            "Portable package identity does not match the signed release.",
        ));
    }
    for name in &new.managed {
        if root.join(name).try_exists()? && !old.managed.contains(name) {
            return Err(bad(
                "New package conflicts with a user-owned file or directory.",
            ));
        }
    }
    let backup = tempfile::Builder::new()
        .prefix(".ulix-backup-")
        .tempdir_in(&root)?;
    let j = Journal {
        schema: 1,
        stage: stage
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned(),
        backup: backup
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned(),
        old: old.clone(),
        new: new.clone(),
    };
    write_journal(&root, &j)?;
    let stage = stage.keep();
    let backup = backup.keep();
    let operation = (|| -> Result<()> {
        for name in old
            .managed
            .iter()
            .chain(std::iter::once(&MARKER.to_string()))
        {
            if root.join(name).try_exists()? {
                fs::rename(root.join(name), backup.join(name))?;
            }
        }
        for name in new
            .managed
            .iter()
            .chain(std::iter::once(&MARKER.to_string()))
        {
            fs::rename(stage.join(name), root.join(name))?;
        }
        #[cfg(unix)]
        fs::File::open(&root)?.sync_all()?;
        Ok(())
    })();
    if let Err(error) = operation {
        if let Err(recovery) = recover(&root) {
            return Err(Error::Io(std::io::Error::other(format!(
                "{error}; recovery required from {}: {recovery}",
                backup.display()
            ))));
        }
        return Err(error);
    }
    Ok((root.join(new.launch), backup))
}
