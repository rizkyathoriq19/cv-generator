use std::fs;
use std::path::{Path, PathBuf};



use crate::{cv::*, library::*};

// ---------------------------------------------------------------------------
// Trash: reversible local deletion, explicit permanent removal
// ---------------------------------------------------------------------------

/// Trash lives inside the library folder. Its name is not a project id, and the
/// library listing only reads `*.json` files directly in `dir`, so trashed CVs
/// are never listed as active ones.
pub(crate) fn trash_dir(dir: &Path) -> PathBuf {
    dir.join("trash")
}

/// The saved file of a CV waiting in Trash.
pub(crate) fn trash_path(dir: &Path, id: &str) -> Result<PathBuf, String> {
    project_file(&trash_dir(dir), id, "")
}

/// Empty marker left after permanent deletion. It keeps a save that was already
/// in flight from recreating the project.
pub(crate) fn deleted_marker(dir: &Path, id: &str) -> Result<PathBuf, String> {
    project_file(&trash_dir(dir), id, ".deleted")
}

/// Every file that belongs to one CV: the current save, its one rolling previous
/// revision, and any damaged copies kept during recovery. Matching requires the
/// dotted suffix, so an id that is a prefix of another id never claims its files.
pub(crate) fn project_files(dir: &Path, id: &str) -> Result<Vec<PathBuf>, String> {
    if !is_valid_project_id(id) || !dir.exists() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    for entry in fs::read_dir(dir).map_err(|e| format!("could not read the CV library: {e}"))? {
        let path = entry.map_err(|e| format!("could not read the CV library: {e}"))?.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let Some(rest) = name.strip_prefix(id) else {
            continue;
        };
        let belongs = rest == ".json"
            || rest == ".prev.json"
            || (rest.starts_with(".corrupt-") && rest.ends_with(".json"));
        if belongs {
            files.push(path);
        }
    }
    Ok(files)
}

/// Rename each file of one CV between the library and Trash. Renames stay on the
/// same filesystem; a failure moves back what already moved, so a CV is never
/// split between the two folders.
pub(crate) fn move_project_files(from: &Path, to: &Path, id: &str, lang: Language) -> Result<(), String> {
    let files = project_files(from, id)?;
    fs::create_dir_all(to).map_err(|e| {
        lang.pick(
            format!("could not open Trash: {e}"),
            format!("tidak dapat membuka Sampah: {e}"),
        )
    })?;
    let mut moved: Vec<(PathBuf, PathBuf)> = Vec::new();
    for path in files {
        let target = to.join(path.file_name().unwrap_or_default());
        if let Err(e) = fs::rename(&path, &target) {
            for (from, to) in moved.iter().rev() {
                let _ = fs::rename(to, from);
            }
            return Err(lang.pick(
                format!("could not move the CV files: {e}"),
                format!("tidak dapat memindahkan berkas CV: {e}"),
            ));
        }
        moved.push((path, target));
    }
    Ok(())
}

/// Delete moves the whole CV — saved file, previous revision, kept damaged
/// copies — into Trash. Nothing is erased and no automatic cleanup runs.
pub(crate) fn delete_in(dir: &Path, id: &str, lang: Language) -> Result<(), String> {
    if !project_path(dir, id)?.exists() {
        return Err(lang.pick(
            "This CV is not in the library.",
            "CV ini tidak ada di pustaka.",
        ));
    }
    move_project_files(dir, &trash_dir(dir), id, lang)?;
    Ok(())
}

/// Restore refuses to overwrite anything already stored under the same identity:
/// a half-restored or duplicate CV is reported, never merged.
pub(crate) fn restore_in(dir: &Path, id: &str, lang: Language) -> Result<(), String> {
    let trash = trash_dir(dir);
    if !trash_path(dir, id)?.exists() {
        return Err(lang.pick(
            "This CV is no longer in Trash.",
            "CV ini tidak lagi ada di Sampah.",
        ));
    }
    if !project_files(dir, id)?.is_empty() {
        return Err(lang.pick(
            "A CV with the same saved identity is already in the library, so this one was not restored. Nothing was changed.",
            "CV dengan identitas tersimpan yang sama sudah ada di pustaka, jadi CV ini tidak dipulihkan. Tidak ada yang diubah.",
        ));
    }
    // Drop a leftover deletion marker first: if that fails nothing moved, and if
    // the move fails the CV is still in Trash, where saves stay refused anyway.
    let marker = deleted_marker(dir, id)?;
    if marker.exists() {
        fs::remove_file(&marker).map_err(|e| {
            lang.pick(
                format!("could not restore the CV: {e}"),
                format!("tidak dapat memulihkan CV: {e}"),
            )
        })?;
    }
    move_project_files(&trash, dir, id, lang)
}

/// Permanent deletion removes the selected CV's files only; exported PDFs and
/// backups live outside the library and are untouched. The empty `.deleted`
/// marker keeps an already-scheduled save from recreating the project.
pub(crate) fn delete_permanently_in(dir: &Path, id: &str, lang: Language) -> Result<(), String> {
    let trash = trash_dir(dir);
    let files = project_files(&trash, id)?;
    if files.is_empty() {
        return Err(lang.pick(
            "This CV is no longer in Trash.",
            "CV ini tidak lagi ada di Sampah.",
        ));
    }
    let marker = deleted_marker(dir, id)?;
    fs::write(&marker, b"").map_err(|e| {
        lang.pick(
            format!("could not finish deleting the CV: {e}"),
            format!("tidak dapat menyelesaikan penghapusan CV: {e}"),
        )
    })?;
    // Remove the recovery copies first and the listed saved file last: if a
    // removal fails, the row stays in Trash and permanent deletion can be retried.
    let current = trash_path(dir, id)?;
    let remove = |path: &Path| {
        fs::remove_file(path).map_err(|e| {
            lang.pick(
                format!("could not delete the CV: {e}"),
                format!("tidak dapat menghapus CV: {e}"),
            )
        })
    };
    for path in files.iter().filter(|path| **path != current) {
        remove(path)?;
    }
    if current.exists() {
        remove(&current)?;
    }
    Ok(())
}

#[tauri::command]
pub fn list_trash(app: tauri::AppHandle) -> Result<Vec<ProjectSummary>, String> {
    let trash = trash_dir(&projects_dir(&app)?);
    if !trash.exists() {
        return Ok(Vec::new());
    }
    summaries_in(&trash)
}

#[tauri::command]
pub fn delete_project(
    app: tauri::AppHandle,
    id: String,
    interface_language: Language,
) -> Result<(), String> {
    delete_in(&projects_dir(&app)?, &id, interface_language)
}

#[tauri::command]
pub fn restore_project(
    app: tauri::AppHandle,
    id: String,
    interface_language: Language,
) -> Result<(), String> {
    restore_in(&projects_dir(&app)?, &id, interface_language)
}

#[tauri::command]
pub fn delete_project_permanently(
    app: tauri::AppHandle,
    id: String,
    interface_language: Language,
) -> Result<(), String> {
    delete_permanently_in(&projects_dir(&app)?, &id, interface_language)
}
