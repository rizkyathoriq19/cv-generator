use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::Manager;
use typst::diag::{FileError, FileResult, SourceDiagnostic};
use typst::foundations::{Bytes, Datetime, Duration};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};
use typst_layout::PagedDocument;

// ---------------------------------------------------------------------------
// CV data (plain data only; never re-parsed as template code)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Link {
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub url: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Experience {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub company: String,
    #[serde(default)]
    pub location: String,
    #[serde(default)]
    pub start: String,
    #[serde(default)]
    pub end: Option<String>,
    #[serde(default)]
    pub bullets: Vec<String>,
}

impl Experience {
    /// A draft entry nobody has filled in yet: it must not reach the PDF.
    fn is_blank(&self) -> bool {
        [&self.title, &self.company, &self.location, &self.start]
            .iter()
            .all(|text| text.trim().is_empty())
            && self.end.as_deref().unwrap_or("").trim().is_empty()
            && self.bullets.iter().all(|bullet| bullet.trim().is_empty())
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Cv {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub phone: String,
    #[serde(default)]
    pub location: String,
    #[serde(default)]
    pub links: Vec<Link>,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub experience: Vec<Experience>,
}

impl Cv {
    /// Export needs an owner name and one substantive non-contact section.
    fn export_problem(&self) -> Option<String> {
        if self.name.trim().is_empty() {
            return Some("Add the CV owner's full name before exporting.".into());
        }
        if self.summary.trim().is_empty() && !self.experience.iter().any(|e| !e.is_blank()) {
            return Some(
                "Add a professional summary or at least one work experience entry before exporting."
                    .into(),
            );
        }
        None
    }
}

// ---------------------------------------------------------------------------
// Embedded renderer: Typst with bundled fonts, one fixed template, no I/O
// ---------------------------------------------------------------------------

const TEMPLATE: &str = include_str!("../templates/cv.typ");
const DATA_PATH: &str = "cv.json";

static LIBRARY: OnceLock<LazyHash<Library>> = OnceLock::new();
static BOOK: OnceLock<LazyHash<FontBook>> = OnceLock::new();
static FONTS: OnceLock<Vec<Font>> = OnceLock::new();

static FONT_FILES: [&[u8]; 3] = [
    include_bytes!("../fonts/LiberationSerif-Regular.ttf"),
    include_bytes!("../fonts/LiberationSerif-Bold.ttf"),
    include_bytes!("../fonts/LiberationSerif-Italic.ttf"),
];

fn fonts() -> &'static Vec<Font> {
    FONTS.get_or_init(|| {
        FONT_FILES
            .iter()
            .filter_map(|data| Font::new(Bytes::new(*data), 0))
            .collect()
    })
}

fn library() -> &'static LazyHash<Library> {
    LIBRARY.get_or_init(|| LazyHash::new(Library::default()))
}

fn book() -> &'static LazyHash<FontBook> {
    BOOK.get_or_init(|| LazyHash::new(fonts().iter().fold(FontBook::new(), |mut book, font| {
        book.push(font.info().clone());
        book
    })))
}

struct CvWorld {
    main: Source,
    data_id: FileId,
    data: Bytes,
}

impl CvWorld {
    fn new(cv: &Cv) -> Result<Self, String> {
        // User text travels as JSON data, never as Typst source.
        let json = serde_json::to_string(cv).map_err(|e| e.to_string())?;
        let source = Source::new(file_id("main.typ"), TEMPLATE.to_string());
        Ok(Self { main: source, data_id: file_id(DATA_PATH), data: Bytes::from_string(json) })
    }
}

impl World for CvWorld {
    fn library(&self) -> &LazyHash<Library> {
        library()
    }
    fn book(&self) -> &LazyHash<FontBook> {
        book()
    }
    fn main(&self) -> FileId {
        self.main.id()
    }
    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.main.id() {
            Ok(self.main.clone())
        } else {
            Err(not_found(id))
        }
    }
    fn file(&self, id: FileId) -> FileResult<Bytes> {
        if id == self.data_id {
            Ok(self.data.clone())
        } else {
            // No arbitrary local file access from the compiler.
            Err(not_found(id))
        }
    }
    fn font(&self, index: usize) -> Option<Font> {
        fonts().get(index).cloned()
    }
    fn today(&self, _offset: Option<Duration>) -> Option<Datetime> {
        None
    }
}

fn not_found(id: FileId) -> FileError {
    FileError::NotFound(PathBuf::from(id.vpath().get_with_slash()))
}

fn file_id(path: &str) -> FileId {
    let vpath = VirtualPath::new(path).expect("static virtual path");
    RootedPath::new(VirtualRoot::Project, vpath).intern()
}

fn diagnostics_message(diagnostics: &[SourceDiagnostic]) -> String {
    diagnostics
        .iter()
        .map(|d| format!("{} @ {:?}", d.message, d.span))
        .collect::<Vec<_>>()
        .join("; ")
}

fn render_document(cv: &Cv) -> Result<PagedDocument, String> {
    let world = CvWorld::new(cv)?;
    typst::compile::<PagedDocument>(&world)
        .output
        .map_err(|diagnostics| format!("render failed: {}", diagnostics_message(&diagnostics)))
}

pub fn render(cv: &Cv) -> Result<Vec<u8>, String> {
    let document = render_document(cv)?;
    typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())
        .map_err(|diagnostics| format!("pdf export failed: {}", diagnostics_message(&diagnostics)))
}

// ---------------------------------------------------------------------------
// Local library: one versioned JSON file per project, atomic replacement
// ---------------------------------------------------------------------------

/// Project file format version. Documented in docs/project-schema.md.
const SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Structure {
    #[default]
    ReverseChronological,
    Functional,
    Combination,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ContentLanguage {
    #[default]
    English,
    Indonesian,
}

/// One independently editable CV. Structure and content language are stored
/// from the first schema version even while only one of each is offered.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Project {
    pub version: u32,
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub structure: Structure,
    #[serde(default)]
    pub content_language: ContentLanguage,
    #[serde(default)]
    pub updated_at: u64,
    #[serde(default)]
    pub cv: Cv,
}

#[derive(Serialize)]
pub struct ProjectSummary {
    id: String,
    title: String,
    name: String,
    updated_at: u64,
}

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn projects_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("no application data directory: {e}"))?
        .join("projects");
    fs::create_dir_all(&dir).map_err(|e| format!("could not open the CV library: {e}"))?;
    Ok(dir)
}

/// Ids are generated hex, never user input: no separators, no traversal.
fn project_path(dir: &Path, id: &str) -> Result<PathBuf, String> {
    if id.is_empty() || !id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') {
        return Err("invalid project id".into());
    }
    Ok(dir.join(format!("{id}.json")))
}

fn read_project(dir: &Path, id: &str) -> Result<Project, String> {
    let path = project_path(dir, id)?;
    let text = fs::read_to_string(&path).map_err(|e| format!("could not open project: {e}"))?;
    let project: Project =
        serde_json::from_str(&text).map_err(|e| format!("project file is unreadable: {e}"))?;
    if project.version != SCHEMA_VERSION {
        return Err(format!(
            "project version {} is not supported (this app reads version {SCHEMA_VERSION})",
            project.version
        ));
    }
    if project.id != id {
        return Err("project file does not belong to the requested project".into());
    }
    Ok(project)
}

fn write_project(dir: &Path, project: &Project) -> Result<Project, String> {
    if project.version != SCHEMA_VERSION {
        return Err(format!("refusing to write unsupported project version {}", project.version));
    }
    let mut stored = project.clone();
    stored.updated_at = now_secs();
    let json = serde_json::to_vec_pretty(&stored)
        .map_err(|e| format!("could not encode project: {e}"))?;
    // Temp write + rename: a failed write leaves the previous file intact.
    save_atomically(&project_path(dir, &stored.id)?, &json)?;
    Ok(stored)
}

#[tauri::command]
fn list_projects(app: tauri::AppHandle) -> Result<Vec<ProjectSummary>, String> {
    let dir = projects_dir(&app)?;
    let mut out = Vec::new();
    for entry in fs::read_dir(&dir).map_err(|e| format!("could not read the CV library: {e}"))? {
        let path = entry.map_err(|e| format!("could not read the CV library: {e}"))?.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let Some(id) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        // ponytail: unreadable files are skipped here; ticket 07 adds the recovery notice.
        if let Ok(project) = read_project(&dir, id) {
            out.push(ProjectSummary {
                id: project.id,
                title: project.title,
                name: project.cv.name,
                updated_at: project.updated_at,
            });
        }
    }
    out.sort_by_key(|project| std::cmp::Reverse(project.updated_at));
    Ok(out)
}

#[tauri::command]
fn create_project(app: tauri::AppHandle) -> Result<Project, String> {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let project = Project {
        version: SCHEMA_VERSION,
        id: format!("{nanos:x}"),
        title: String::new(),
        structure: Structure::default(),
        content_language: ContentLanguage::default(),
        updated_at: 0,
        cv: Cv::default(),
    };
    write_project(&projects_dir(&app)?, &project)
}

#[tauri::command]
fn open_project(app: tauri::AppHandle, id: String) -> Result<Project, String> {
    read_project(&projects_dir(&app)?, &id)
}

#[tauri::command]
fn save_project(app: tauri::AppHandle, project: Project) -> Result<Project, String> {
    write_project(&projects_dir(&app)?, &project)
}

// ---------------------------------------------------------------------------
// Commands: preview bytes are cached in Rust and reused verbatim for export
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct CachedRender {
    revision: u64,
    /// The exact content these bytes were rendered from: export re-checks it so the
    /// frontend cannot hand over a newer CV beside an older artifact.
    cv: Cv,
    bytes: Vec<u8>,
}

impl CachedRender {
    fn matches(&self, revision: u64, cv: &Cv) -> bool {
        self.revision == revision && &self.cv == cv
    }
}

#[derive(Default)]
struct RenderCache(Mutex<Option<CachedRender>>);

#[derive(Serialize)]
pub struct ExportAck {
    path: String,
    bytes: usize,
    sha256: String,
}

#[tauri::command]
fn render_pdf(cv: Cv, revision: u64, cache: tauri::State<RenderCache>) -> Result<tauri::ipc::Response, String> {
    let pdf = render(&cv)?;
    let mut cached = cache.0.lock().unwrap();
    // Concurrent invokes can finish out of order; keep the newest revision only.
    if cached.as_ref().is_some_and(|c| c.revision > revision) {
        return Err("stale render discarded".into());
    }
    *cached = Some(CachedRender { revision, cv, bytes: pdf.clone() });
    Ok(tauri::ipc::Response::new(pdf))
}

#[tauri::command]
fn export_pdf(
    revision: u64,
    path: String,
    cv: Cv,
    cache: tauri::State<RenderCache>,
) -> Result<ExportAck, String> {
    // Rust owns export validation: drafts may be incomplete, exports may not.
    if let Some(problem) = cv.export_problem() {
        return Err(problem);
    }
    let cached = cache
        .0
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| "nothing has been rendered yet".to_string())?;
    // Rust, not the frontend, proves these bytes belong to the validated CV.
    if !cached.matches(revision, &cv) {
        return Err("preview is out of date; render it again before exporting".into());
    }
    // The dialog is advisory: only a .pdf destination is ever written.
    if !path.to_ascii_lowercase().ends_with(".pdf") {
        return Err("destination must be a .pdf file".into());
    }
    save_atomically(Path::new(&path), &cached.bytes)?;
    Ok(ExportAck {
        path,
        bytes: cached.bytes.len(),
        sha256: hex(&cached.bytes),
    })
}

/// Write next to the destination, then rename so a failed write leaves the
/// existing file untouched.
fn save_atomically(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let dir = path.parent().filter(|p| !p.as_os_str().is_empty()).ok_or("invalid destination path")?;
    let name = path.file_name().ok_or("invalid destination path")?.to_string_lossy();
    let tmp = dir.join(format!(".{}.tmp-{}", name, std::process::id()));
    fs::write(&tmp, bytes).map_err(|e| format!("write failed: {e}"))?;
    if let Err(e) = fs::rename(&tmp, path) {
        let _ = fs::remove_file(&tmp);
        return Err(format!("could not replace destination: {e}"));
    }
    Ok(())
}

fn hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(RenderCache::default())
        .invoke_handler(tauri::generate_handler![
            render_pdf,
            export_pdf,
            list_projects,
            create_project,
            open_project,
            save_project
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn main() {
    run();
}

// ---------------------------------------------------------------------------
// Checks: A4 pages, text survives as extractable text, export is byte-identical
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    // Progressive setup reads better than one giant struct literal in these checks.
    #![allow(clippy::field_reassign_with_default)]

    use super::*;

    fn fixture(bullets: usize, bullet_len: usize) -> Cv {
        Cv {
            name: "Ada Lovelace".into(),
            email: "ada@example.invalid".into(),
            phone: "".into(),
            location: "London".into(),
            links: vec![Link { label: "Web".into(), url: "https://example.invalid/portfolio".into() }],
            summary: "Analytical engine programmer.".into(),
            experience: (0..6)
                .map(|i| Experience {
                    title: format!("Engineer {i}"),
                    company: "Analytical Engines Ltd".into(),
                    location: "London".into(),
                    start: "Jan 2020".into(),
                    end: Some("Dec 2021".into()),
                    bullets: (0..bullets).map(|b| format!("Achievement {b} {}", "x".repeat(bullet_len))).collect(),
                })
                .collect(),
        }
    }

    /// Writes the synthetic fixtures for external inspection (pdfinfo/pdftotext).
    #[test]
    fn writes_evidence_pdfs() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.scratch/evidence");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("short.pdf"), render(&fixture(3, 60)).unwrap()).unwrap();
        fs::write(dir.join("multipage.pdf"), render(&fixture(12, 140)).unwrap()).unwrap();
        let mut narrow = fixture(1, 0);
        narrow.summary = format!("Long token: {} and {}", "y".repeat(200), "https://example.invalid/".to_string() + &"segment/".repeat(30));
        fs::write(dir.join("longtoken.pdf"), render(&narrow).unwrap()).unwrap();
    }

    #[test]
    fn renders_a4_multipage_pdf() {
        let document = render_document(&fixture(12, 140)).unwrap();
        assert!(
            document.pages().len() > 1,
            "expected multipage output, got {}",
            document.pages().len()
        );
        let size = document.pages()[0].frame.size();
        assert!((size.x.to_pt() - 595.276).abs() < 0.1, "width {}", size.x.to_pt());
        assert!((size.y.to_pt() - 841.89).abs() < 0.1, "height {}", size.y.to_pt());
        assert!(render(&fixture(12, 140)).unwrap().starts_with(b"%PDF"));
    }

    #[test]
    fn different_content_renders_different_documents() {
        let a = render(&fixture(3, 60)).unwrap();
        let b = render(&fixture(12, 140)).unwrap();
        assert_ne!(a, b, "renderer returned a cached document for different content");
    }

    fn document_text(document: &PagedDocument) -> String {
        fn walk(frame: &typst::layout::Frame, out: &mut String) {
            for (_, item) in frame.items() {
                match item {
                    typst::layout::FrameItem::Text(text) => out.push_str(&text.text),
                    typst::layout::FrameItem::Group(group) => walk(&group.frame, out),
                    _ => {}
                }
            }
        }
        let mut out = String::new();
        for page in document.pages() {
            walk(&page.frame, &mut out);
            out.push('\n');
        }
        out
    }

    #[test]
    fn rendered_text_stays_extractable() {
        let text = document_text(&render_document(&fixture(1, 10)).unwrap());
        assert!(text.contains("Ada Lovelace"), "name missing from rendered text");
        assert!(text.contains("PROFESSIONAL SUMMARY"), "heading missing");
        assert!(text.contains("Achievement 0"), "bullet missing");
    }

    #[test]
    fn template_looking_text_stays_literal() {
        let mut cv = fixture(1, 4);
        cv.name = r#"#eval("read(\"/etc/passwd\")") *bold*"#.into();
        cv.summary = r#"#include "secret.json""#.into();
        let text = document_text(&render_document(&cv).unwrap());
        assert!(text.contains("#eval("), "authored text missing from output: {text}");
        assert!(text.contains("#include"), "authored include text missing");
        assert!(!text.contains("root:"), "user text was evaluated as code");
    }


    #[test]
    fn world_serves_only_its_declared_files() {
        let world = CvWorld::new(&fixture(1, 1)).unwrap();
        assert!(world.file(world.data_id).is_ok(), "template data file must load");
        let foreign = file_id("/etc/passwd");
        assert!(world.file(foreign).is_err(), "compiler must not read arbitrary files");
        assert!(world.source(foreign).is_err(), "compiler must not source arbitrary files");
    }

    #[test]
    fn export_writes_exact_cached_bytes() {
        let dir = std::env::temp_dir().join(format!("cvgen-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let target = dir.join("cv.pdf");
        fs::write(&target, b"older content").unwrap();

        let pdf = render(&fixture(1, 10)).unwrap();
        save_atomically(&target, &pdf).unwrap();
        assert_eq!(fs::read(&target).unwrap(), pdf, "exported bytes differ from rendered bytes");
        assert_eq!(hex(&pdf), hex(&fs::read(&target).unwrap()));

        let _ = fs::remove_dir_all(&dir);
    }

    // --- ticket 02: local library, autosave durability, export validation ---

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("cvgen-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn project() -> Project {
        Project {
            version: SCHEMA_VERSION,
            id: "0a1b2c".into(),
            title: String::new(),
            structure: Structure::default(),
            content_language: ContentLanguage::default(),
            updated_at: 0,
            cv: fixture(1, 6),
        }
    }

    #[test]
    fn project_file_round_trips_with_its_schema_version() {
        let dir = temp_dir("library");
        let stored = write_project(&dir, &project()).unwrap();
        assert_eq!(stored.version, SCHEMA_VERSION);
        assert!(stored.updated_at > 0, "a saved project records when it changed");

        let json = fs::read_to_string(dir.join("0a1b2c.json")).unwrap();
        for expected in [
            "\"version\": 1",
            "\"structure\": \"reverse-chronological\"",
            "\"content_language\": \"english\"",
        ] {
            assert!(json.contains(expected), "missing {expected} in {json}");
        }

        // Reopening reads the same content back: durability, not in-memory state.
        let reopened = read_project(&dir, "0a1b2c").unwrap();
        assert_eq!(reopened.cv.name, "Ada Lovelace");
        assert_eq!(reopened.cv.experience.len(), 6);
        assert_eq!(reopened.cv.experience[0].bullets.len(), 1);
        assert!(reopened.cv.experience[0].bullets[0].starts_with("Achievement 0"));

        let leftovers: Vec<String> = fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.contains(".tmp-"))
            .collect();
        assert!(leftovers.is_empty(), "temp files left behind: {leftovers:?}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn autosave_replaces_the_existing_project_file() {
        let dir = temp_dir("autosave");
        let mut project = project();
        write_project(&dir, &project).unwrap();
        project.cv.summary = "Second autosave.".into();
        write_project(&dir, &project).unwrap();

        assert_eq!(read_project(&dir, &project.id).unwrap().cv.summary, "Second autosave.");
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 1, "one file per project");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn unsupported_or_mismatched_project_files_are_refused() {
        let dir = temp_dir("versions");

        let mut newer = project();
        newer.version = SCHEMA_VERSION + 1;
        assert!(write_project(&dir, &newer).unwrap_err().contains("unsupported"));

        let future = r#"{"version":99,"id":"future","cv":{"name":"Ada"}}"#;
        fs::write(dir.join("future.json"), future).unwrap();
        assert!(read_project(&dir, "future").unwrap_err().contains("not supported"));
        assert_eq!(fs::read_to_string(dir.join("future.json")).unwrap(), future, "refused file rewritten");

        fs::write(dir.join("other.json"), r#"{"version":1,"id":"someone-else","cv":{}}"#).unwrap();
        assert!(read_project(&dir, "other").unwrap_err().contains("does not belong"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn project_ids_never_escape_the_library_folder() {
        let dir = temp_dir("ids");
        for id in ["", "../escape", "sub/dir", "a.json"] {
            assert!(project_path(&dir, id).is_err(), "{id:?} must be rejected");
        }
        assert!(project_path(&dir, "0a1b2c").is_ok());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn cached_bytes_only_match_the_content_they_were_rendered_from() {
        let cached = CachedRender { revision: 4, cv: fixture(1, 6), bytes: b"%PDF".to_vec() };
        assert!(cached.matches(4, &fixture(1, 6)), "same revision and content must match");

        let mut edited = fixture(1, 6);
        edited.summary = "Edited after the preview was rendered.".into();
        assert!(!cached.matches(4, &edited), "newer content must not be exported with older bytes");
        assert!(!cached.matches(5, &fixture(1, 6)), "a different revision must not match");
    }

    #[test]
    fn export_requires_a_name_and_a_non_contact_section() {
        let mut cv = Cv::default();
        assert!(cv.export_problem().unwrap().contains("full name"));

        cv.name = "Ada Lovelace".into();
        assert!(cv.export_problem().unwrap().contains("summary or"));

        cv.summary = "   ".into();
        assert!(cv.export_problem().is_some(), "whitespace is not content");

        cv.experience = vec![Experience::default()];
        assert!(cv.export_problem().is_some(), "blank entries are not content");

        cv.experience[0].bullets = vec!["  ".into()];
        assert!(cv.export_problem().is_some(), "blank bullets are not content");

        cv.summary = "Analytical engine programmer.".into();
        assert!(cv.export_problem().is_none(), "a summary alone is enough");
    }

    #[test]
    fn blank_draft_entries_and_sections_do_not_render() {
        let mut cv = Cv::default();
        cv.name = "Ada Lovelace".into();
        cv.summary = "   ".into();
        cv.experience = vec![
            Experience::default(),
            Experience { bullets: vec!["".into(), "  ".into()], ..Default::default() },
        ];

        let text = document_text(&render_document(&cv).unwrap());
        assert!(text.contains("Ada Lovelace"), "the name still renders: {text:?}");
        assert!(!text.contains("WORK EXPERIENCE"), "empty section rendered: {text:?}");
        assert!(!text.contains("PROFESSIONAL SUMMARY"), "blank summary rendered: {text:?}");
        assert!(!text.contains("Present"), "blank entry invented an ongoing role: {text:?}");

        cv.summary = "Analytical engine programmer.".into();
        cv.experience[1].title = "Engineer".into();
        let text = document_text(&render_document(&cv).unwrap());
        assert!(text.contains("WORK EXPERIENCE"));
        assert!(text.contains("PROFESSIONAL SUMMARY"));
        assert_eq!(text.matches("Engineer").count(), 1, "one row, not one per draft entry");
    }

    #[test]
    fn a_missing_end_date_is_not_reported_as_present() {
        let mut cv = Cv::default();
        cv.name = "Ada Lovelace".into();
        cv.summary = "Analytical engine programmer.".into();
        cv.experience = vec![Experience {
            title: "Engineer".into(),
            start: "2020".into(),
            end: Some(String::new()),
            ..Default::default()
        }];
        // An empty end string means "no end recorded", not an ongoing role.
        let text = document_text(&render_document(&cv).unwrap());
        assert!(text.contains("2020"), "{text:?}");
        assert!(!text.contains("Present"), "empty end became Present: {text:?}");
        assert!(!text.contains("2020 – "), "trailing separator rendered: {text:?}");

        // Only an explicit ongoing role (no end value at all) says Present.
        cv.experience[0].end = None;
        let text = document_text(&render_document(&cv).unwrap());
        assert!(text.contains("Present"), "an ongoing role still says Present: {text:?}");
    }
}
