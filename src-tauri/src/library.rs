use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::Manager;


use crate::{cv::*, files::*, trash::*};

// ---------------------------------------------------------------------------
// Local library: one versioned JSON file per project, atomic replacement
// ---------------------------------------------------------------------------

/// Project file format version. Documented in docs/project-schema.md.
pub(crate) const SCHEMA_VERSION: u32 = 1;

/// One independently editable CV. Structure is a presentation choice over the
/// same content; content language selects the PDF headings without translating
/// anything the user authored.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Project {
    pub version: u32,
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub structure: Structure,
    #[serde(default)]
    pub content_language: Language,
    #[serde(default)]
    pub updated_at: u64,
    #[serde(default)]
    pub cv: Cv,
}

#[derive(Serialize)]
pub struct ProjectSummary {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) name: String,
    pub(crate) updated_at: u64,
    /// Shown beside the label so CV copies for another language or structure
    /// are distinguishable; `None` when no revision of the file is readable.
    pub(crate) structure: Option<Structure>,
    pub(crate) content_language: Option<Language>,
    /// The current file is unreadable. The CV stays listed so opening it can
    /// recover the previous revision or report why it cannot.
    pub(crate) damaged: bool,
}

pub(crate) fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

pub(crate) fn projects_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("no application data directory: {e}"))?
        .join("projects");
    fs::create_dir_all(&dir).map_err(|e| format!("could not open the CV library: {e}"))?;
    Ok(dir)
}

/// One running application owns the library, so two windows cannot race two
/// writers against the same projects. The OS releases this lock on process exit,
/// so a crash never leaves a stale lock behind.
#[derive(Debug)]
pub(crate) struct InstanceLock(#[allow(dead_code)] fs::File);

pub(crate) fn instance_lock(dir: &Path) -> Result<InstanceLock, String> {
    let file = fs::File::create(dir.join("instance.lock"))
        .map_err(|e| format!("could not create the instance lock: {e}"))?;
    file.try_lock().map_err(|_| {
        "Another CV Generator window is already open. Close it before starting a second one: \
         only one application instance may edit the library."
            .to_string()
    })?;
    Ok(InstanceLock(file))
}

/// Ids are generated hex, never user input: no separators, no traversal.
pub(crate) fn is_valid_project_id(id: &str) -> bool {
    !id.is_empty() && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}

pub(crate) fn project_file(dir: &Path, id: &str, suffix: &str) -> Result<PathBuf, String> {
    if !is_valid_project_id(id) {
        return Err("invalid project id".into());
    }
    Ok(dir.join(format!("{id}{suffix}.json")))
}

pub(crate) fn project_path(dir: &Path, id: &str) -> Result<PathBuf, String> {
    project_file(dir, id, "")
}

/// The one rolling previous valid revision. Its dotted name is not a project id,
/// so the library listing never mistakes it (or a preserved damaged file) for a CV.
pub(crate) fn previous_path(dir: &Path, id: &str) -> Result<PathBuf, String> {
    project_file(dir, id, ".prev")
}

/// Test-only control over the replacement sequence. The running application
/// always passes the default; checks choose the failure they need.
#[derive(Clone, Copy, Default)]
pub(crate) struct Faults {
    /// The library refuses the write (read-only folder, full disk).
    pub(crate) deny_write: bool,
    /// The replacement stops after the previous revision was rotated in.
    pub(crate) interrupt_replace: bool,
}

/// A fresh identity for a new project or a duplicate. Lowercase hex only, as
/// documented: nanosecond clock concatenated with a process counter cannot
/// repeat inside one process; ids never come from user input.
pub(crate) fn new_project_id() -> String {
    format!("{:x}{:013x}", now_nanos(), NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
}

pub(crate) fn now_nanos() -> u128 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0)
}

pub(crate) static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

#[derive(Debug)]
pub(crate) enum ReadError {
    /// Missing or unparseable: recovery from the previous revision applies.
    Damaged(String),
    /// Readable but refused by this app: never rewritten or rolled back.
    Refused(Refusal),
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum Refusal {
    /// A project file this app version does not read.
    Version(u32),
    /// A project file belonging to another id.
    ForeignId,
}

/// English detail for non-interface callers (duplicate, diagnostics). The
/// interface formats its own localized message from the refusal.
impl std::fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Damaged(message) => f.write_str(message),
            Self::Refused(Refusal::Version(version)) => write!(
                f,
                "project version {version} is not supported (this app reads version {SCHEMA_VERSION})"
            ),
            Self::Refused(Refusal::ForeignId) => {
                f.write_str("project file does not belong to the requested project")
            }
        }
    }
}

pub(crate) fn read_project_file(path: &Path) -> Result<Project, ReadError> {
    let text = fs::read_to_string(path)
        .map_err(|e| ReadError::Damaged(format!("could not open project: {e}")))?;
    let project: Project = serde_json::from_str(&text)
        .map_err(|e| ReadError::Damaged(format!("project file is unreadable: {e}")))?;
    if project.version != SCHEMA_VERSION {
        return Err(ReadError::Refused(Refusal::Version(project.version)));
    }
    Ok(project)
}

pub(crate) fn read_project(dir: &Path, id: &str) -> Result<Project, ReadError> {
    let path = project_path(dir, id).map_err(ReadError::Damaged)?;
    let project = read_project_file(&path)?;
    if project.id != id {
        return Err(ReadError::Refused(Refusal::ForeignId));
    }
    Ok(project)
}

/// Keep exactly one rolling valid revision: the current file becomes the
/// previous one just before it is replaced. The new content is already prepared,
/// so a failure here leaves the current file exactly as it was. A current file
/// that no longer parses is never rotated, because that would destroy the only
/// recoverable copy.
pub(crate) fn rotate_previous(current: &Path, previous: &Path) -> Result<(), String> {
    if !current.exists() || read_project_file(current).is_err() {
        return Ok(());
    }
    let bytes =
        fs::read(current).map_err(|e| format!("could not keep the previous revision: {e}"))?;
    // ponytail: project-save detail stays English OS text (ticket 05 limit); export localizes.
    save_atomically(previous, &bytes, Language::English)
}

/// Writes a project, returning the stored project and the preserved damaged copy
/// when the save found an unreadable current file.
pub(crate) fn write_project_in(
    dir: &Path,
    project: &Project,
    faults: Faults,
) -> Result<(Project, Option<PathBuf>), String> {
    if project.version != SCHEMA_VERSION {
        return Err(format!("refusing to write unsupported project version {}", project.version));
    }
    let mut stored = project.clone();
    stored.updated_at = now_secs();
    let json = serde_json::to_vec_pretty(&stored)
        .map_err(|e| format!("could not encode project: {e}"))?;
    if faults.deny_write {
        return Err("writing the project failed: Permission denied (os error 13)".into());
    }
    let current = project_path(dir, &stored.id)?;
    // Damage found while the editor was open is kept before the save replaces it.
    // A readable file this app refuses (newer version, another project's id) is
    // never overwritten by older content.
    let preserved = match read_project(dir, &stored.id) {
        Err(ReadError::Damaged(_)) => preserve_corrupt(dir, &stored.id)?,
        Err(ReadError::Refused(Refusal::Version(version))) => {
            return Err(format!(
                "refusing to overwrite project version {version} with version {SCHEMA_VERSION}"
            ));
        }
        Err(ReadError::Refused(Refusal::ForeignId)) => {
            return Err("refusing to overwrite a project file that belongs to another project".into());
        }
        Ok(_) => None,
    };
    // Rotate first, replace second: recoverability never depends on the new write.
    rotate_previous(&current, &previous_path(dir, &stored.id)?)?;
    if faults.interrupt_replace {
        return Err("replacing the project was interrupted".into());
    }
    save_atomically(&current, &json, Language::English)?;
    Ok((stored, preserved))
}

pub(crate) fn write_project(dir: &Path, project: &Project) -> Result<Project, String> {
    Ok(write_project_in(dir, project, Faults::default())?.0)
}

/// Interface notice for a save that kept a damaged file. Never silent: the
/// editor shows it beside the Saved state.
pub(crate) fn damage_notice(path: &Path, lang: Language) -> String {
    lang.pick(
        format!(
            "The saved file was damaged while you were editing. Your current edits were saved, \
             and the damaged file was kept at {}.",
            path.display()
        ),
        format!(
            "Berkas tersimpan rusak saat Anda mengedit. Editan Anda sudah disimpan, dan berkas \
             yang rusak disimpan di {}.",
            path.display()
        ),
    )
}


/// The library lists projects, never previous revisions or preserved damaged
/// files. A damaged current file is read through its previous revision so the
/// row stays identifiable and recoverable.
pub(crate) fn summaries_in(dir: &Path) -> Result<Vec<ProjectSummary>, String> {
    let mut out = Vec::new();
    for entry in fs::read_dir(dir).map_err(|e| format!("could not read the CV library: {e}"))? {
        let path = entry.map_err(|e| format!("could not read the CV library: {e}"))?.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let Some(id) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        if !is_valid_project_id(id) {
            continue;
        }
        let (label, damaged) = match read_project(dir, id) {
            Ok(project) => (Some(project), false),
            // The row stays so it can be opened and reported; its previous
            // revision supplies the label when that copy is readable.
            Err(error) => (
                read_project_file(&previous_path(dir, id)?).ok(),
                matches!(error, ReadError::Damaged(_)),
            ),
        };
        out.push(ProjectSummary {
            id: id.to_string(),
            title: label.as_ref().map(|p| p.title.clone()).unwrap_or_default(),
            name: label.as_ref().map(|p| p.cv.name.clone()).unwrap_or_default(),
            updated_at: label.as_ref().map_or(0, |p| p.updated_at),
            structure: label.as_ref().map(|p| p.structure),
            content_language: label.as_ref().map(|p| p.content_language),
            damaged,
        });
    }
    out.sort_by_key(|project| std::cmp::Reverse(project.updated_at));
    Ok(out)
}

#[tauri::command]
pub fn list_projects(app: tauri::AppHandle) -> Result<Vec<ProjectSummary>, String> {
    summaries_in(&projects_dir(&app)?)
}

#[tauri::command]
pub fn create_project(app: tauri::AppHandle) -> Result<Project, String> {
    let project = Project {
        version: SCHEMA_VERSION,
        id: new_project_id(),
        title: String::new(),
        structure: Structure::default(),
        content_language: Language::default(),
        updated_at: 0,
        cv: Cv::default(),
    };
    write_project(&projects_dir(&app)?, &project)
}

/// Duplicating copies every field — content, structure, and content language —
/// into a new project id. The two files are unrelated afterwards: each later
/// save writes only its own id.
pub(crate) fn duplicate_in(dir: &Path, id: &str) -> Result<Project, String> {
    let mut copy = read_project(dir, id).map_err(|e| e.to_string())?;
    copy.id = new_project_id();
    write_project(dir, &copy)
}

#[tauri::command]
pub fn duplicate_project(app: tauri::AppHandle, id: String) -> Result<Project, String> {
    duplicate_in(&projects_dir(&app)?, &id)
}

/// What opening a CV means when the saved file was damaged: the notice the
/// interface shows and where the unreadable file was kept for inspection.
#[derive(Debug, Serialize)]
pub struct Recovery {
    pub(crate) notice: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) preserved_as: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct OpenedProject {
    pub(crate) project: Project,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) recovery: Option<Recovery>,
}

/// Keep a copy of the unreadable current file before recovery or a save replaces
/// it. Copy, not rename: if the replacement write fails, the CV must stay at its
/// current path so the library still lists it. The name is not a project id, so
/// the library listing skips it.
pub(crate) fn preserve_corrupt(dir: &Path, id: &str) -> Result<Option<PathBuf>, String> {
    let current = project_path(dir, id)?;
    if !current.exists() {
        return Ok(None);
    }
    let kept = dir.join(format!("{id}.corrupt-{}.json", now_nanos()));
    fs::copy(&current, &kept)
        .map_err(|e| format!("could not keep the damaged file: {e}"))?;
    Ok(Some(kept))
}

/// The current file did not parse. Recover the one previous revision, keep the
/// damaged file, and say so. With no valid previous revision nothing is written
/// or blanked: the damaged file is reported where it lies.
pub(crate) fn recover_in(dir: &Path, id: &str, lang: Language) -> Result<OpenedProject, String> {
    let current = project_path(dir, id)?;
    let previous = previous_path(dir, id)?;
    let recovered = read_project_file(&previous).ok().filter(|project| project.id == id);
    let Some(previous_project) = recovered else {
        return Err(lang.pick(
            format!(
                "This CV could not be opened: its saved file is missing or damaged and no valid \
                 previous revision exists. The damaged file was left unchanged at {}.",
                current.display()
            ),
            format!(
                "CV ini tidak dapat dibuka: berkas tersimpannya hilang atau rusak dan tidak ada \
                 revisi sebelumnya yang valid. Berkas yang rusak dibiarkan tidak berubah di {}.",
                current.display()
            ),
        ));
    };
    let preserved = preserve_corrupt(dir, id)?;
    // Restore the previous revision's own bytes: its content and timestamp are
    // the recovered save, not a new edit. The next start then opens it normally.
    let bytes = fs::read(&previous).map_err(|e| format!("could not restore the project: {e}"))?;
    save_atomically(&current, &bytes, Language::English)?;
    let kept = preserved.as_ref().map(|path| {
        lang.pick(
            format!("The damaged file was kept at {}.", path.display()),
            format!("Berkas yang rusak disimpan di {}.", path.display()),
        )
    });
    let notice = lang.pick(
        format!(
            "Your saved file was damaged, so the previous saved revision was recovered. This \
             application keeps one previous revision, not full history. Review the CV after \
             opening it. {}",
            kept.clone().unwrap_or_default()
        ),
        format!(
            "Berkas tersimpan Anda rusak, sehingga revisi tersimpan sebelumnya dipulihkan. \
             Aplikasi ini hanya menyimpan satu revisi sebelumnya, bukan seluruh riwayat. \
             Periksa kembali CV setelah dibuka. {}",
            kept.clone().unwrap_or_default()
        ),
    );
    Ok(OpenedProject {
        project: previous_project,
        recovery: Some(Recovery { notice: notice.trim_end().to_string(), preserved_as: preserved.map(|path| path.display().to_string()) }),
    })
}

pub(crate) fn open_in(dir: &Path, id: &str, lang: Language) -> Result<OpenedProject, String> {
    match read_project(dir, id) {
        Ok(project) => Ok(OpenedProject { project, recovery: None }),
        // Only a missing or unparseable file is recoverable. A readable project
        // this app refuses keeps its explicit error and its bytes.
        Err(ReadError::Damaged(_)) => recover_in(dir, id, lang),
        Err(ReadError::Refused(Refusal::Version(version))) => Err(lang.pick(
            format!(
                "This CV uses project version {version}; this app reads version {SCHEMA_VERSION} \
                 and will not rewrite it. Update the app to open it."
            ),
            format!(
                "CV ini memakai versi proyek {version}; aplikasi ini membaca versi \
                 {SCHEMA_VERSION} dan tidak akan menulis ulang. Perbarui aplikasi untuk membukanya."
            ),
        )),
        Err(ReadError::Refused(Refusal::ForeignId)) => Err(lang.pick(
            "This CV's saved file belongs to another project and was not opened.",
            "Berkas tersimpan CV ini milik proyek lain dan tidak dibuka.",
        )),
    }
}

#[tauri::command]
pub fn open_project(
    app: tauri::AppHandle,
    id: String,
    interface_language: Language,
) -> Result<OpenedProject, String> {
    open_in(&projects_dir(&app)?, &id, interface_language)
}

#[derive(Debug, Serialize)]
pub struct SaveResult {
    pub(crate) project: Project,
    /// Present when the save kept a damaged file it found while writing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) notice: Option<String>,
}

/// The editor's save path adds the Trash guards: a CV waiting in Trash, or one
/// already permanently deleted, must not be recreated by a write that was
/// already in flight when the user deleted it.
pub(crate) fn save_in(dir: &Path, project: &Project, lang: Language) -> Result<SaveResult, String> {
    if deleted_marker(dir, &project.id)?.exists() {
        return Err(lang.pick(
            "This CV was permanently deleted and was not saved again.",
            "CV ini sudah dihapus permanen dan tidak disimpan lagi.",
        ));
    }
    if trash_path(dir, &project.id)?.exists() {
        return Err(lang.pick(
            "This CV is in Trash and was not saved. Restore it before editing.",
            "CV ini ada di Sampah dan tidak disimpan. Pulihkan dulu sebelum mengedit.",
        ));
    }
    let (stored, preserved) = write_project_in(dir, project, Faults::default())?;
    Ok(SaveResult {
        project: stored,
        notice: preserved.map(|path| damage_notice(&path, lang)),
    })
}

#[tauri::command]
pub fn save_project(
    app: tauri::AppHandle,
    project: Project,
    interface_language: Language,
) -> Result<SaveResult, String> {
    save_in(&projects_dir(&app)?, &project, interface_language)
}
