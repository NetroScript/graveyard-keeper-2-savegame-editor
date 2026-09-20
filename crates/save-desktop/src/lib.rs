use gk2_save_core::{DocumentSummary, Workspace};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
};
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
        path: path.into(),
        name,
        metadata,
        metadata_error,
    }
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
        write_pair(&path, &bytes, metadata.as_deref(), expected, retention)?;
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
) -> Result<()> {
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
    let backup_root = path
        .parent()
        .unwrap()
        .join(".gk2-editor-backups")
        .join(path.file_name().unwrap());
    let replace = (|| {
        // Prepare and sync the retained backup before touching the destination.
        let backup = if retention > 0 && old.is_some() {
            fs::create_dir_all(&backup_root).map_err(err)?;
            let dir = tempfile::Builder::new()
                .prefix("save-")
                .tempdir_in(&backup_root)
                .map_err(err)?;
            atomic(&dir.path().join("save.dat"), old.as_ref().unwrap())?;
            if let Some(b) = &old_info {
                atomic(&dir.path().join("save.info"), b)?;
            }
            Some(dir)
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
        if let Some(backup) = backup {
            let _ = backup.keep();
        }
        Ok(())
    })();
    if let Err(e) = replace {
        recover(path)?;
        return Err(e);
    }
    // Cleanup failures leave recoverable data and do not misreport a completed save.
    let _ = fs::remove_dir_all(stage);
    if let Ok(entries) = fs::read_dir(backup_root) {
        let mut entries: Vec<_> = entries
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().starts_with("save-") && e.path().is_dir())
            .collect();
        entries.sort_by_key(|e| e.metadata().and_then(|m| m.modified()).ok());
        let count = entries.len().saturating_sub(retention as usize);
        for entry in entries.into_iter().take(count) {
            let _ = fs::remove_dir_all(entry.path());
        }
    }
    Ok(())
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
            assert_eq!(
                fs::read(entry.unwrap().path().join("save.info")).unwrap(),
                metadata
            );
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
}
