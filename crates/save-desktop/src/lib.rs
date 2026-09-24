use gk2_save_core::{DocumentSummary, Workspace};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};
use zip::{write::SimpleFileOptions, CompressionMethod, ZipArchive, ZipWriter};
type Result<T> = std::result::Result<T, String>;
fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub out_of_bounds_edits: bool,
    pub backup_retention: u8,
    pub custom_directories: Vec<PathBuf>,
    pub interface_scale: f64,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            out_of_bounds_edits: false,
            backup_retention: 5,
            custom_directories: vec![],
            interface_scale: 1.0,
        }
    }
}
impl Settings {
    pub fn validate(&self) -> Result<()> {
        if self.backup_retention > 50
            || !self.interface_scale.is_finite()
            || !(0.75..=1.5).contains(&self.interface_scale)
        {
            return Err("Invalid settings".into());
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub path: PathBuf,
    pub name: String,
    pub metadata: Option<Value>,
    pub metadata_error: Option<String>,
    pub backup: bool,
    pub editor_backups: bool,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupPreview {
    pub name: String,
    pub modified: u64,
    pub bytes: u64,
    pub metadata: Option<Value>,
    pub metadata_error: Option<String>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreResult {
    pub restored: bool,
    pub backup_created: bool,
}
fn info(path: &Path) -> PathBuf {
    path.with_extension("info")
}
fn optional(path: &Path) -> Result<Option<Vec<u8>>> {
    match fs::read(path) {
        Ok(b) => Ok(Some(b)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(err(e)),
    }
}
fn fingerprint(path: &Path) -> Result<Vec<u8>> {
    let mut h = Sha256::new();
    h.update(fs::read(path).map_err(err)?);
    match optional(&info(path))? {
        Some(b) => {
            h.update([1]);
            h.update(b)
        }
        None => h.update([0]),
    }
    Ok(h.finalize().to_vec())
}
pub fn preview(path: &Path) -> Preview {
    let (metadata, metadata_error) = match optional(&info(path)) {
        Ok(Some(b)) => match serde_json::from_slice(&b) {
            Ok(v) => (Some(v), None),
            Err(e) => (None, Some(err(e))),
        },
        Ok(None) => (None, None),
        Err(e) => (None, Some(e)),
    };
    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    Preview {
        backup: name.contains("_backup_"),
        editor_backups: has_editor_backups(path),
        path: path.into(),
        name,
        metadata,
        metadata_error,
    }
}

fn has_editor_backups(path: &Path) -> bool {
    fs::read_dir(backup_root(path)).is_ok_and(|entries| {
        entries.flatten().any(|entry| {
            let path = entry.path();
            path.is_file()
                && path
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"))
        })
    })
}
pub fn list(directories: &[PathBuf], include_backups: bool) -> Vec<Preview> {
    let mut found = vec![];
    for dir in directories {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file() && p.extension().is_some_and(|e| e.eq_ignore_ascii_case("dat")) {
                    let item = preview(&p);
                    if include_backups || !item.backup {
                        found.push(item)
                    }
                }
            }
        }
    }
    found.sort_by(|a, b| a.path.cmp(&b.path));
    found.dedup_by(|a, b| a.path == b.path);
    found
}
/// Environment and platform are inputs so discovery can be tested without the host machine.
pub fn discover(platform: &str, env: &HashMap<String, String>, custom: &[PathBuf]) -> Vec<PathBuf> {
    let home = env
        .get(if platform == "windows" {
            "USERPROFILE"
        } else {
            "HOME"
        })
        .or_else(|| env.get("HOME"))
        .map(PathBuf::from)
        .unwrap_or_default();
    let products = ["Graveyard Keeper 2 Demo", "Graveyard Keeper 2"];
    let mut candidates = custom.to_vec();
    let base = match platform {
        "windows" => home.join("AppData/LocalLow"),
        "macos" => home.join("Library/Application Support"),
        _ => env
            .get("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".config"))
            .join("unity3d"),
    };
    for product in products {
        candidates.push(base.join("Lazy Bear Games").join(product));
    }
    if platform == "linux" {
        let mut libraries = vec![
            home.join(".steam/steam"),
            home.join(".local/share/Steam"),
            home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam"),
        ];
        if let Some(data) = env.get("XDG_DATA_HOME") {
            libraries.push(PathBuf::from(data).join("Steam"));
        }
        for library in libraries.clone() {
            if let Ok(vdf) = fs::read_to_string(library.join("steamapps/libraryfolders.vdf")) {
                let tokens = vdf_tokens(&vdf);
                for pair in tokens.windows(2) {
                    if pair[0] == "path" {
                        libraries.push(PathBuf::from(&pair[1]));
                    }
                }
            }
        }
        for library in libraries {
            for app in ["5075680", "4358690"] {
                let users = library
                    .join("steamapps/compatdata")
                    .join(app)
                    .join("pfx/drive_c/users");
                if let Ok(entries) = fs::read_dir(users) {
                    for user in entries.flatten() {
                        for product in products {
                            candidates.push(
                                user.path()
                                    .join("AppData/LocalLow/Lazy Bear Games")
                                    .join(product),
                            );
                        }
                    }
                }
            }
        }
    }
    candidates.retain(|p| p.is_dir());
    let mut unique = vec![];
    for p in candidates {
        let p = fs::canonicalize(p).unwrap_or_default();
        if !unique.contains(&p) {
            unique.push(p)
        }
    }
    unique
}
fn vdf_tokens(input: &str) -> Vec<String> {
    let mut tokens = vec![];
    let mut chars = input.chars();
    while let Some(c) = chars.next() {
        if c == '"' {
            let mut token = String::new();
            while let Some(c) = chars.next() {
                if c == '"' {
                    break;
                }
                if c == '\\' {
                    if let Some(c) = chars.next() {
                        token.push(c)
                    }
                } else {
                    token.push(c)
                }
            }
            tokens.push(token)
        }
    }
    tokens
}
struct Source {
    path: PathBuf,
    fingerprint: Vec<u8>,
    metadata: Option<Vec<u8>>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Opened {
    pub summary: DocumentSummary,
    pub preview: Preview,
}
#[derive(Default)]
pub struct DesktopWorkspace {
    pub workspace: Workspace,
    sources: HashMap<u32, Source>,
    pub recent_directory: Option<PathBuf>,
}
impl DesktopWorkspace {
    pub fn open_path(&mut self, path: &Path) -> Result<Opened> {
        recover(path)?;
        let path = fs::canonicalize(path).map_err(err)?;
        if let Some((&id, _)) = self.sources.iter().find(|(_, s)| s.path == path) {
            return Ok(Opened {
                summary: self.workspace.summary(id).map_err(err)?,
                preview: preview(&path),
            });
        }
        recover(&path)?;
        let before = fingerprint(&path)?;
        let bytes = fs::read(&path).map_err(err)?;
        let metadata = optional(&info(&path))?;
        if fingerprint(&path)? != before {
            return Err("File changed while opening; try again".into());
        }
        let summary = self.workspace.open(&bytes).map_err(err)?;
        self.recent_directory = path.parent().map(Path::to_path_buf);
        self.sources.insert(
            summary.document_id,
            Source {
                path: path.clone(),
                fingerprint: before,
                metadata,
            },
        );
        Ok(Opened {
            summary,
            preview: preview(&path),
        })
    }
    pub fn close(&mut self, id: u32) {
        self.sources.remove(&id);
    }
    pub fn path(&self, id: u32) -> Option<PathBuf> {
        self.sources.get(&id).map(|s| s.path.clone())
    }
    pub fn save(
        &mut self,
        id: u32,
        revision: u32,
        destination: Option<PathBuf>,
        retention: u8,
    ) -> Result<Opened> {
        if retention > 50 {
            return Err("Backup retention must be 0–50".into());
        }
        if self.workspace.summary(id).map_err(err)?.revision != revision {
            return Err("Stale revision".into());
        }
        let source = self.sources.get(&id);
        let path = destination
            .or_else(|| source.map(|s| s.path.clone()))
            .ok_or("Choose a destination")?;
        let parent = fs::canonicalize(path.parent().ok_or("Invalid destination")?).map_err(err)?;
        let path = parent.join(path.file_name().ok_or("Invalid filename")?);
        if self
            .sources
            .iter()
            .any(|(other, s)| *other != id && s.path == path)
        {
            return Err("Destination is already open in another document".into());
        }
        recover(&path)?;
        if let Some(source) = source {
            if source.path == path && fingerprint(&path).ok().as_ref() != Some(&source.fingerprint)
            {
                return Err("EXTERNAL_CHANGE".into());
            }
        }
        let expected = if path.exists() {
            Some(fingerprint(&path)?)
        } else {
            None
        };
        let bytes = self.workspace.export(id).map_err(err)?;
        let metadata = source.and_then(|s| s.metadata.clone());
        write_pair(
            &path,
            &bytes,
            metadata.as_deref(),
            expected,
            retention,
            false,
        )?;
        let summary = self.workspace.mark_saved(id, revision).map_err(err)?;
        self.sources.insert(
            id,
            Source {
                fingerprint: fingerprint(&path)?,
                path: path.clone(),
                metadata,
            },
        );
        self.recent_directory = Some(parent);
        Ok(Opened {
            summary,
            preview: preview(&path),
        })
    }

    pub fn backups(&self, path: &Path) -> Result<Vec<BackupPreview>> {
        let path = fs::canonicalize(path).map_err(err)?;
        let root = backup_root(&path);
        let mut backups = vec![];
        let entries = match fs::read_dir(root) {
            Ok(entries) => entries,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(backups),
            Err(e) => return Err(err(e)),
        };
        for entry in entries.flatten() {
            let archive_path = entry.path();
            if !archive_path.is_file()
                || archive_path
                    .extension()
                    .is_none_or(|extension| !extension.eq_ignore_ascii_case("zip"))
            {
                continue;
            }
            let metadata_on_disk = entry.metadata().map_err(err)?;
            let (metadata, metadata_error) = backup_metadata(&archive_path, &path);
            backups.push(BackupPreview {
                name: entry.file_name().to_string_lossy().into_owned(),
                modified: metadata_on_disk
                    .modified()
                    .ok()
                    .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                    .map_or(0, |duration| duration.as_secs()),
                bytes: metadata_on_disk.len(),
                metadata,
                metadata_error,
            });
        }
        backups.sort_by(|a, b| {
            b.modified
                .cmp(&a.modified)
                .then_with(|| b.name.cmp(&a.name))
        });
        Ok(backups)
    }

    pub fn restore_backup(
        &mut self,
        path: &Path,
        backup: &str,
        retention: u8,
    ) -> Result<RestoreResult> {
        if retention > 50 {
            return Err("Backup retention must be 0–50".into());
        }
        let path = fs::canonicalize(path).map_err(err)?;
        if self.sources.values().any(|source| source.path == path) {
            return Err("Close this save before restoring a backup".into());
        }
        if backup.is_empty()
            || backup
                != Path::new(backup)
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
            || !backup.to_ascii_lowercase().ends_with(".zip")
        {
            return Err("Invalid backup name".into());
        }
        let archive_path = backup_root(&path).join(backup);
        let (data, metadata) = read_backup(&archive_path, &path)?;
        let expected = Some(fingerprint(&path)?);
        if optional(&path)?.as_deref() == Some(data.as_slice())
            && optional(&info(&path))?.as_deref() == metadata.as_deref()
        {
            if Some(fingerprint(&path)?) != expected {
                return Err("EXTERNAL_CHANGE".into());
            }
            return Ok(RestoreResult {
                restored: false,
                backup_created: false,
            });
        }
        let backup_created =
            write_pair(&path, &data, metadata.as_deref(), expected, retention, true)?;
        Ok(RestoreResult {
            restored: true,
            backup_created,
        })
    }
}
fn journal(path: &Path) -> PathBuf {
    path.with_file_name(format!(
        ".{}.editor-recovery",
        path.file_name().unwrap_or_default().to_string_lossy()
    ))
}
fn atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut tmp =
        tempfile::NamedTempFile::new_in(path.parent().ok_or("Missing parent")?).map_err(err)?;
    tmp.write_all(bytes).map_err(err)?;
    tmp.as_file().sync_all().map_err(err)?;
    tmp.persist(path).map_err(err)?;
    Ok(())
}
fn backup_archive(
    root: &Path,
    path: &Path,
    data: &[u8],
    metadata: Option<&[u8]>,
) -> Result<tempfile::NamedTempFile> {
    let mut file = tempfile::Builder::new()
        .prefix("save-")
        .suffix(".zip")
        .tempfile_in(root)
        .map_err(err)?;
    {
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .compression_level(Some(6));
        let mut archive = ZipWriter::new(file.as_file_mut());
        archive
            .start_file(
                path.file_name().unwrap_or_default().to_string_lossy(),
                options,
            )
            .map_err(err)?;
        archive.write_all(data).map_err(err)?;
        if let Some(metadata) = metadata {
            archive
                .start_file(
                    info(path).file_name().unwrap_or_default().to_string_lossy(),
                    options,
                )
                .map_err(err)?;
            archive.write_all(metadata).map_err(err)?;
        }
        archive.finish().map_err(err)?;
    }
    file.as_file().sync_all().map_err(err)?;
    Ok(file)
}

fn backup_root(path: &Path) -> PathBuf {
    path.parent()
        .unwrap_or_else(|| Path::new("."))
        .join(".gk2-editor-backups")
        .join(path.file_name().unwrap_or_default())
}

fn backup_contains_pair(
    root: &Path,
    path: &Path,
    data: &[u8],
    metadata: Option<&[u8]>,
) -> Result<bool> {
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(err(e)),
    };
    for entry in entries {
        let entry = entry.map_err(err)?;
        if !entry.path().is_file()
            || entry
                .path()
                .extension()
                .is_none_or(|extension| !extension.eq_ignore_ascii_case("zip"))
        {
            continue;
        }
        if let Ok((saved, saved_metadata)) = read_backup(&entry.path(), path) {
            if saved == data && saved_metadata.as_deref() == metadata {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn read_backup(path: &Path, destination: &Path) -> Result<(Vec<u8>, Option<Vec<u8>>)> {
    const MAX_ENTRY_BYTES: u64 = 512 * 1024 * 1024;
    let file = fs::File::open(path).map_err(err)?;
    let mut archive = ZipArchive::new(file).map_err(err)?;
    let data_name = destination
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let info_name = info(destination)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let read = |archive: &mut ZipArchive<fs::File>, name: &str| -> Result<Option<Vec<u8>>> {
        let mut entry = match archive.by_name(name) {
            Ok(entry) => entry,
            Err(zip::result::ZipError::FileNotFound) => return Ok(None),
            Err(e) => return Err(err(e)),
        };
        if !entry.is_file() || entry.size() > MAX_ENTRY_BYTES {
            return Err("Invalid backup entry".into());
        }
        let mut bytes = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut bytes).map_err(err)?;
        Ok(Some(bytes))
    };
    let data = read(&mut archive, &data_name)?.ok_or("Backup does not contain the save file")?;
    let metadata = read(&mut archive, &info_name)?;
    Ok((data, metadata))
}

fn backup_metadata(path: &Path, destination: &Path) -> (Option<Value>, Option<String>) {
    const MAX_METADATA_BYTES: u64 = 16 * 1024 * 1024;
    let result = (|| -> Result<Option<Vec<u8>>> {
        let file = fs::File::open(path).map_err(err)?;
        let mut archive = ZipArchive::new(file).map_err(err)?;
        let name = info(destination)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let mut entry = match archive.by_name(&name) {
            Ok(entry) => entry,
            Err(zip::result::ZipError::FileNotFound) => return Ok(None),
            Err(e) => return Err(err(e)),
        };
        if !entry.is_file() || entry.size() > MAX_METADATA_BYTES {
            return Err("Invalid backup metadata entry".into());
        }
        let mut bytes = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut bytes).map_err(err)?;
        Ok(Some(bytes))
    })();
    match result {
        Ok(Some(bytes)) => match serde_json::from_slice(&bytes) {
            Ok(value) => (Some(value), None),
            Err(e) => (None, Some(err(e))),
        },
        Ok(None) => (None, None),
        Err(e) => (None, Some(e)),
    }
}
/// A journal retains the complete previous pair until both replacements have succeeded.
pub fn recover(path: &Path) -> Result<()> {
    let dir = journal(path);
    if !dir.exists() {
        return Ok(());
    }
    if !dir.join("ready").exists() || dir.join("committed").exists() {
        fs::remove_dir_all(dir).map_err(err)?;
        return Ok(());
    }
    for (old, dest) in [("old.dat", path.to_path_buf()), ("old.info", info(path))] {
        if dir.join(old).exists() {
            atomic(&dest, &fs::read(dir.join(old)).map_err(err)?)?
        } else if dest.exists() {
            fs::remove_file(dest).map_err(err)?
        }
    }
    fs::remove_dir_all(dir).map_err(err)
}
fn write_pair(
    path: &Path,
    bytes: &[u8],
    metadata: Option<&[u8]>,
    expected: Option<Vec<u8>>,
    retention: u8,
    avoid_duplicate_backup: bool,
) -> Result<bool> {
    let current = if path.exists() {
        Some(fingerprint(path)?)
    } else {
        None
    };
    if current != expected {
        return Err("EXTERNAL_CHANGE".into());
    }
    let old = optional(path)?;
    let old_info = optional(&info(path))?;
    let stage = journal(path);
    fs::create_dir(&stage).map_err(err)?;
    let prepare = (|| {
        if let Some(b) = &old {
            atomic(&stage.join("old.dat"), b)?
        }
        if let Some(b) = &old_info {
            atomic(&stage.join("old.info"), b)?
        }
        atomic(&stage.join("new.dat"), bytes)?;
        if let Some(b) = metadata {
            atomic(&stage.join("new.info"), b)?
        }
        atomic(&stage.join("ready"), b"1")
    })();
    if let Err(e) = prepare {
        let _ = fs::remove_dir_all(&stage);
        return Err(e);
    }
    let current = if path.exists() {
        Some(fingerprint(path)?)
    } else {
        None
    };
    if current != expected {
        fs::remove_dir_all(stage).map_err(err)?;
        return Err("EXTERNAL_CHANGE".into());
    }
    let backup_root = backup_root(path);
    let replace = (|| {
        // Prepare and sync the retained backup before touching the destination.
        let already_archived = if avoid_duplicate_backup && retention > 0 {
            match old.as_ref() {
                Some(old) => backup_contains_pair(&backup_root, path, old, old_info.as_deref())?,
                None => false,
            }
        } else {
            false
        };
        let backup = if retention > 0 && old.is_some() && !already_archived {
            fs::create_dir_all(&backup_root).map_err(err)?;
            Some(backup_archive(
                &backup_root,
                path,
                old.as_ref().unwrap(),
                old_info.as_deref(),
            )?)
        } else {
            None
        };
        atomic(path, bytes)?;
        if let Some(b) = metadata {
            atomic(&info(path), b)?;
        } else if info(path).exists() {
            fs::remove_file(info(path)).map_err(err)?;
        }
        // Recovery must not roll back a completed save if cleanup is interrupted.
        atomic(&stage.join("committed"), b"1")?;
        let backup_created = backup.is_some();
        if let Some(backup) = backup {
            let _ = backup.keep();
        }
        Ok(backup_created)
    })();
    let backup_created = match replace {
        Ok(created) => created,
        Err(e) => {
            recover(path)?;
            return Err(e);
        }
    };
    // Cleanup failures leave recoverable data and do not misreport a completed save.
    let _ = fs::remove_dir_all(stage);
    if let Ok(entries) = fs::read_dir(backup_root) {
        let mut entries: Vec<_> = entries
            .flatten()
            .filter(|e| {
                e.path().is_file()
                    && e.file_name().to_string_lossy().starts_with("save-")
                    && e.path()
                        .extension()
                        .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"))
            })
            .collect();
        entries.sort_by_key(|e| e.metadata().and_then(|m| m.modified()).ok());
        let count = entries.len().saturating_sub(retention as usize);
        for entry in entries.into_iter().take(count) {
            let _ = fs::remove_file(entry.path());
        }
    }
    Ok(backup_created)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn synthetic_native_proton_and_sidecars() {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path();
        let native = home.join("custom-config/unity3d/Lazy Bear Games/Graveyard Keeper 2 Demo");
        fs::create_dir_all(&native).unwrap();
        let steam = home.join(".local/share/Steam/steamapps");
        fs::create_dir_all(&steam).unwrap();
        let library = home.join("Games Library");
        fs::write(
            steam.join("libraryfolders.vdf"),
            format!(
                "\"libraryfolders\" {{ \"1\" {{ \"path\" \"{}\" }} }}",
                library.display().to_string().replace('\\', "\\\\")
            ),
        )
        .unwrap();
        let proton=library.join("steamapps/compatdata/4358690/pfx/drive_c/users/steamuser/AppData/LocalLow/Lazy Bear Games/Graveyard Keeper 2");
        fs::create_dir_all(&proton).unwrap();
        let env = HashMap::from([
            ("HOME".into(), home.display().to_string()),
            (
                "XDG_CONFIG_HOME".into(),
                home.join("custom-config").display().to_string(),
            ),
        ]);
        let dirs = discover("linux", &env, &[]);
        assert_eq!(dirs.len(), 2);
        fs::write(native.join("slot.dat"), [255]).unwrap();
        fs::write(native.join("slot.info"), b"broken").unwrap();
        fs::write(native.join("slot_backup_1.dat"), []).unwrap();
        assert_eq!(list(&dirs, false).len(), 1);
        assert!(list(&dirs, false)[0].metadata_error.is_some());
        assert_eq!(list(&dirs, true).len(), 2);
        for (platform, relative) in [
            ("windows", "AppData/LocalLow"),
            ("macos", "Library/Application Support"),
        ] {
            let p = home
                .join(relative)
                .join("Lazy Bear Games/Graveyard Keeper 2");
            fs::create_dir_all(&p).unwrap();
            assert_eq!(discover(platform, &env, &[]).len(), 1);
        }
    }
    #[test]
    fn save_as_backups_external_changes_and_recovery() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("slot.dat");
        let bytes = [4, 46, 44, 0, 5];
        let metadata = b"{\"day\":3}";
        fs::write(&path, bytes).unwrap();
        fs::write(info(&path), metadata).unwrap();
        let mut w = DesktopWorkspace::default();
        let first = w.open_path(&path).unwrap();
        let id = first.summary.document_id;
        assert_eq!(w.open_path(&path).unwrap().summary.document_id, id);
        for _ in 0..4 {
            w.save(id, 1, None, 2).unwrap();
        }
        let backups = temp.path().join(".gk2-editor-backups/slot.dat");
        assert_eq!(fs::read_dir(&backups).unwrap().count(), 2);
        for entry in fs::read_dir(&backups).unwrap() {
            let file = fs::File::open(entry.unwrap().path()).unwrap();
            let mut archive = zip::ZipArchive::new(file).unwrap();
            let mut saved_data = Vec::new();
            let mut data_entry = archive.by_name("slot.dat").unwrap();
            assert_eq!(data_entry.compression(), CompressionMethod::Deflated);
            std::io::Read::read_to_end(&mut data_entry, &mut saved_data).unwrap();
            drop(data_entry);
            assert_eq!(saved_data, bytes);
            let mut saved_metadata = Vec::new();
            std::io::Read::read_to_end(
                &mut archive.by_name("slot.info").unwrap(),
                &mut saved_metadata,
            )
            .unwrap();
            assert_eq!(saved_metadata, metadata);
        }
        let dest = temp.path().join("copy.dat");
        w.save(id, 1, Some(dest.clone()), 2).unwrap();
        assert_eq!(fs::read(info(&dest)).unwrap(), metadata);
        fs::write(&dest, [4, 46, 44, 1, 5]).unwrap();
        assert_eq!(w.save(id, 1, None, 2).unwrap_err(), "EXTERNAL_CHANGE");
        assert_eq!(fs::read(&dest).unwrap(), [4, 46, 44, 1, 5]);
        let stage = journal(&path);
        fs::create_dir(&stage).unwrap();
        fs::write(stage.join("old.dat"), bytes).unwrap();
        fs::write(stage.join("old.info"), metadata).unwrap();
        fs::write(stage.join("ready"), b"1").unwrap();
        fs::write(&path, b"partial").unwrap();
        recover(&path).unwrap();
        assert_eq!(fs::read(&path).unwrap(), bytes);
        assert_eq!(fs::read(info(&path)).unwrap(), metadata);
        assert!(!stage.exists());
    }

    #[test]
    fn lists_and_restores_paired_archives() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("slot.dat");
        let original = b"original save";
        let original_info = b"{\"day\":7,\"gameSaveVersion\":\"1.2.3\"}";
        fs::write(&path, original).unwrap();
        fs::write(info(&path), original_info).unwrap();
        let root = backup_root(&path);
        assert!(!preview(&path).editor_backups);
        fs::create_dir_all(&root).unwrap();
        let archive = backup_archive(&root, &path, original, Some(original_info)).unwrap();
        let (_, archive_path) = archive.keep().unwrap();
        assert!(preview(&path).editor_backups);
        fs::write(&path, b"current save").unwrap();
        fs::write(info(&path), b"{\"day\":8}").unwrap();

        let mut workspace = DesktopWorkspace::default();
        let backups = workspace.backups(&path).unwrap();
        assert_eq!(backups.len(), 1);
        assert_eq!(backups[0].metadata.as_ref().unwrap()["day"], 7);
        assert_eq!(
            backups[0].name,
            archive_path.file_name().unwrap().to_string_lossy()
        );
        let result = workspace
            .restore_backup(&path, &backups[0].name, 3)
            .unwrap();
        assert!(result.restored && result.backup_created);

        assert_eq!(fs::read(&path).unwrap(), original);
        assert_eq!(fs::read(info(&path)).unwrap(), original_info);
        let after_restore = workspace.backups(&path).unwrap();
        assert_eq!(after_restore.len(), 2);
        assert!(
            !workspace
                .restore_backup(&path, &backups[0].name, 3)
                .unwrap()
                .restored
        );
        let other = after_restore
            .iter()
            .find(|item| item.name != backups[0].name)
            .unwrap();
        for (name, expected) in [
            (&other.name, b"current save".as_slice()),
            (&backups[0].name, original.as_slice()),
            (&other.name, b"current save".as_slice()),
            (&backups[0].name, original.as_slice()),
        ] {
            let result = workspace.restore_backup(&path, name, 3).unwrap();
            assert!(result.restored);
            assert!(!result.backup_created);
            assert_eq!(fs::read(&path).unwrap(), expected);
        }
        assert_eq!(
            workspace
                .backups(&path)
                .unwrap()
                .iter()
                .map(|backup| &backup.name)
                .collect::<Vec<_>>(),
            after_restore
                .iter()
                .map(|backup| &backup.name)
                .collect::<Vec<_>>()
        );
        fs::write(info(&path), b"changed metadata").unwrap();
        let result = workspace
            .restore_backup(&path, &backups[0].name, 3)
            .unwrap();
        assert!(result.restored && result.backup_created);
        assert_eq!(fs::read(info(&path)).unwrap(), original_info);
    }
}
