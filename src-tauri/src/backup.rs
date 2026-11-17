use std::fs;
use std::io::Read;
use std::path::Path;

use serde::{Deserialize, Serialize};


use crate::{cv::*, files::*, library::*, render::*};

// ---------------------------------------------------------------------------
// Project backups: portable, unencrypted snapshots that import as a new project
// ---------------------------------------------------------------------------

pub(crate) const BACKUP_FORMAT: &str = "cv-generator-backup";
pub(crate) const BACKUP_VERSION: u32 = 1;
/// A CV is kilobytes of text. This bounds the read and parse before any decoding;
/// export refuses to write a file import would refuse to read.
pub(crate) const MAX_BACKUP_BYTES: usize = 2 * 1024 * 1024;

/// A backup carries no id, path, or timestamp: identity belongs to the library.
/// `deny_unknown_fields` rejects a file that tries to bring one.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Backup {
    pub(crate) format: String,
    pub(crate) version: u32,
    #[serde(default)]
    pub(crate) title: String,
    #[serde(default)]
    pub(crate) structure: Structure,
    #[serde(default)]
    pub(crate) content_language: Language,
    /// Required: a file without content is not a snapshot of a project.
    pub(crate) cv: Cv,
}

/// Both directions refuse the same size; export says so before writing anything.
pub(crate) fn within_backup_limit(len: usize, lang: Language) -> Result<(), String> {
    if len <= MAX_BACKUP_BYTES {
        return Ok(());
    }
    let limit = MAX_BACKUP_BYTES;
    Err(lang.pick(
        format!("The backup is larger than the {limit}-byte limit."),
        format!("Cadangan melebihi batas {limit} byte."),
    ))
}

pub(crate) fn encode_backup(backup: &Backup, lang: Language) -> Result<Vec<u8>, String> {
    let json = serde_json::to_vec_pretty(backup).map_err(|e| format!("could not encode backup: {e}"))?;
    within_backup_limit(json.len(), lang)?;
    Ok(json)
}

/// Everything is checked here, before the library is touched.
pub(crate) fn parse_backup(bytes: &[u8], lang: Language) -> Result<Backup, String> {
    let invalid = |detail: String| {
        lang.pick(
            format!("This is not a valid CV Generator backup: {detail}"),
            format!("Ini bukan cadangan CV Generator yang valid: {detail}"),
        )
    };
    let value: serde_json::Value = serde_json::from_slice(bytes).map_err(|e| invalid(e.to_string()))?;
    if value.get("format").and_then(|f| f.as_str()) != Some(BACKUP_FORMAT) {
        return Err(invalid(lang.pick("backup format marker missing", "penanda format cadangan tidak ada")));
    }
    // A newer file is refused by name before its (possibly different) shape is read.
    if let Some(version) = value.get("version").and_then(|v| v.as_u64()).filter(|v| *v > BACKUP_VERSION as u64) {
        return Err(lang.pick(
            format!("This backup uses format version {version}; this app reads version {BACKUP_VERSION}. Update the app to open it."),
            format!("Cadangan ini memakai format versi {version}; aplikasi ini membaca versi {BACKUP_VERSION}. Perbarui aplikasi untuk membukanya."),
        ));
    }
    let backup: Backup = serde_json::from_value(value).map_err(|e| invalid(e.to_string()))?;
    if backup.version != BACKUP_VERSION {
        return Err(invalid(lang.pick(
            format!("unsupported format version {}", backup.version),
            format!("versi format {} tidak didukung", backup.version),
        )));
    }
    if let Some(problem) = backup.cv.date_shape_problem(lang) {
        return Err(invalid(problem));
    }
    Ok(backup)
}

/// Import always mints a new id, so a repeated or look-alike backup can never
/// replace an existing project. The only write is one new file in `dir`.
pub(crate) fn import_backup_in(dir: &Path, path: &Path, lang: Language) -> Result<Project, String> {
    let mut bytes = Vec::new();
    fs::File::open(path)
        .and_then(|file| file.take(MAX_BACKUP_BYTES as u64 + 1).read_to_end(&mut bytes))
        .map_err(|e| lang.pick(format!("could not read the backup: {e}"), format!("tidak dapat membaca cadangan: {e}")))?;
    within_backup_limit(bytes.len(), lang)?;
    let backup = parse_backup(&bytes, lang)?;
    write_project(
        dir,
        &Project {
            version: SCHEMA_VERSION,
            id: new_project_id(),
            title: backup.title,
            structure: backup.structure,
            content_language: backup.content_language,
            updated_at: 0,
            cv: backup.cv,
        },
    )
}

#[tauri::command]
pub fn import_backup(app: tauri::AppHandle, path: String, interface_language: Language) -> Result<Project, String> {
    import_backup_in(&projects_dir(&app)?, Path::new(&path), interface_language)
}

/// Snapshot of the editor's in-memory CV. The save dialog confirms replacing a
/// file; `save_atomically` guarantees a failed write leaves the old one intact.
#[tauri::command]
pub fn export_backup(
    path: String,
    title: String,
    structure: Structure,
    content_language: Language,
    cv: Cv,
    interface_language: Language,
) -> Result<ExportAck, String> {
    if !path.to_ascii_lowercase().ends_with(".json") {
        return Err(interface_language.pick(
            "destination must be a .json file",
            "tujuan harus berupa berkas .json",
        ));
    }
    // Same preconditions import enforces, so a backup is always importable.
    if let Some(problem) = cv.date_shape_problem(interface_language) {
        return Err(problem);
    }
    let backup = Backup { format: BACKUP_FORMAT.into(), version: BACKUP_VERSION, title, structure, content_language, cv };
    let json = encode_backup(&backup, interface_language)?;
    save_atomically(Path::new(&path), &json, interface_language)?;
    Ok(ExportAck { path, bytes: json.len(), sha256: hex(&json), warnings: Vec::new() })
}
