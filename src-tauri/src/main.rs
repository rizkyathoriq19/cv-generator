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

/// A career date with month/year precision, year-only precision, or no value.
/// `month` is only ever written when the user selected one: a missing month is
/// never materialized, so year-only records stay year-only.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CvDate {
    #[serde(default)]
    pub year: Option<u16>,
    #[serde(default)]
    pub month: Option<u8>,
}

impl CvDate {
    fn is_empty(&self) -> bool {
        self.year.is_none()
    }

    /// Only provable contradictions: both years are known, or the same year with
    /// both months known. A year-only side never proves a month is wrong.
    fn contradicts(&self, end: &CvDate) -> bool {
        let (Some(start_year), Some(end_year)) = (self.year, end.year) else {
            return false;
        };
        if start_year != end_year {
            return start_year > end_year;
        }
        matches!((self.month, end.month), (Some(start), Some(end)) if start > end)
    }

    /// The documented contract is 1-12 and never a month without a year. A
    /// hand-edited file must fail with a message, not inside the template.
    fn shape_is_valid(&self) -> bool {
        self.month
            .is_none_or(|month| (1..=12).contains(&month) && self.year.is_some())
    }
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
    pub start: CvDate,
    /// `None` means an ongoing role (Present); an empty date means no end recorded.
    #[serde(default)]
    pub end: Option<CvDate>,
    #[serde(default)]
    pub bullets: Vec<String>,
}

impl Experience {
    /// A draft entry nobody has filled in yet: it must not reach the PDF.
    fn is_blank(&self) -> bool {
        [&self.title, &self.company, &self.location]
            .iter()
            .all(|text| text.trim().is_empty())
            && self.start.is_empty()
            && self.end.is_none_or(|end| end.is_empty())
            && self.bullets.iter().all(|bullet| bullet.trim().is_empty())
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Education {
    #[serde(default)]
    pub school: String,
    #[serde(default)]
    pub degree: String,
    #[serde(default)]
    pub location: String,
    #[serde(default)]
    pub start: CvDate,
    #[serde(default)]
    pub end: Option<CvDate>,
    #[serde(default)]
    pub bullets: Vec<String>,
}

impl Education {
    fn is_blank(&self) -> bool {
        [&self.school, &self.degree, &self.location]
            .iter()
            .all(|text| text.trim().is_empty())
            && self.start.is_empty()
            && self.end.is_none_or(|end| end.is_empty())
            && self.bullets.iter().all(|bullet| bullet.trim().is_empty())
    }
}

/// A named skill group. Reverse chronological CVs use it as supporting content;
/// the group's achievements lead in functional and combination structures.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SkillGroup {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub bullets: Vec<String>,
}

impl SkillGroup {
    fn is_blank(&self) -> bool {
        self.name.trim().is_empty() && self.bullets.iter().all(|bullet| bullet.trim().is_empty())
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ProjectEntry {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub detail: String,
    #[serde(default)]
    pub bullets: Vec<String>,
}

impl ProjectEntry {
    fn is_blank(&self) -> bool {
        [&self.name, &self.detail].iter().all(|text| text.trim().is_empty())
            && self.bullets.iter().all(|bullet| bullet.trim().is_empty())
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Certification {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub issuer: String,
    #[serde(default)]
    pub date: CvDate,
}

impl Certification {
    fn is_blank(&self) -> bool {
        [&self.name, &self.issuer].iter().all(|text| text.trim().is_empty())
            && self.date.is_empty()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Language {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub level: String,
}

impl Language {
    fn is_blank(&self) -> bool {
        self.name.trim().is_empty() && self.level.trim().is_empty()
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
    #[serde(default)]
    pub education: Vec<Education>,
    #[serde(default)]
    pub skills: Vec<SkillGroup>,
    #[serde(default)]
    pub projects: Vec<ProjectEntry>,
    #[serde(default)]
    pub certifications: Vec<Certification>,
    #[serde(default)]
    pub languages: Vec<Language>,
}

impl Cv {
    /// Export needs an owner name and one substantive non-contact section.
    fn export_problem(&self) -> Option<String> {
        if self.name.trim().is_empty() {
            return Some("Add the CV owner's full name before exporting.".into());
        }
        let substantive = !self.summary.trim().is_empty()
            || self.experience.iter().any(|e| !e.is_blank())
            || self.education.iter().any(|e| !e.is_blank())
            || self.skills.iter().any(|s| !s.is_blank())
            || self.projects.iter().any(|p| !p.is_blank())
            || self.certifications.iter().any(|c| !c.is_blank())
            || self.languages.iter().any(|l| !l.is_blank());
        if !substantive {
            return Some(
                "Add a professional summary or at least one work experience, education, skill, \
                 project, certification, or language entry before exporting."
                    .into(),
            );
        }
        if !self.email.trim().is_empty() && !valid_email(&self.email) {
            return Some("The email address is not valid. Fix it or clear it before exporting.".into());
        }
        if let Some(link) = self.links.iter().find(|l| !l.url.trim().is_empty() && !valid_web_link(&l.url)) {
            let label = if link.label.trim().is_empty() {
                "a link".to_string()
            } else {
                format!("\"{}\"", link.label.trim())
            };
            return Some(format!(
                "The web link for {label} is not valid: it must start with http:// or https:// and \
                 contain no spaces. Fix it or clear it before exporting."
            ));
        }
        if let Some(problem) = self.date_shape_problem() {
            return Some(problem);
        }
        self.date_problem()
    }

    /// At least one skill group carries a user-authored achievement bullet.
    /// A named group with no evidence is not a functional/combination lead.
    fn has_skill_achievements(&self) -> bool {
        self.skills
            .iter()
            .any(|group| group.bullets.iter().any(|bullet| !bullet.trim().is_empty()))
    }

    /// Warnings never block export: they name optional content the PDF will lack.
    fn export_warnings(&self, structure: Structure) -> Vec<String> {
        let mut warnings = Vec::new();
        if matches!(structure, Structure::Functional | Structure::Combination)
            && !self.has_skill_achievements()
        {
            let name = if structure == Structure::Functional {
                "functional"
            } else {
                "combination"
            };
            warnings.push(format!(
                "The {name} structure relies on skill-group achievements, but no skill group has \
                 an achievement bullet yet. Add achievements or choose another structure."
            ));
        }
        if self.email.trim().is_empty() && self.phone.trim().is_empty() {
            warnings.push(
                "No email or phone number is included, so the PDF gives employers no way to contact you."
                    .into(),
            );
        }
        let undated = self
            .experience
            .iter()
            .filter(|entry| !entry.is_blank())
            .map(|entry| entry.start.year.is_none())
            .chain(
                self.education
                    .iter()
                    .filter(|entry| !entry.is_blank())
                    .map(|entry| entry.start.year.is_none()),
            )
            .filter(|missing| *missing)
            .count();
        if undated > 0 {
            warnings.push(
                "Some entries have no start date, so they are listed after dated entries.".into(),
            );
        }
        warnings
    }

    /// A month outside 1-12, or a month without a year, is corrupt data rather
    /// than user input; refuse it before it reaches the template.
    fn date_shape_problem(&self) -> Option<String> {
        let invalid = self
            .experience
            .iter()
            .map(|entry| ("work experience", &entry.start, entry.end.as_ref()))
            .chain(self.education.iter().map(|entry| ("education", &entry.start, entry.end.as_ref())))
            .chain(self.certifications.iter().map(|entry| ("certification", &entry.date, None)))
            .find(|(_, start, end)| {
                !start.shape_is_valid() || end.is_some_and(|end| !end.shape_is_valid())
            });
        invalid.map(|(section, _, _)| {
            format!("Fix the dates for the {section}: a month must be between 1 and 12 and needs a year.")
        })
    }

    fn date_problem(&self) -> Option<String> {
        let histories = self
            .experience
            .iter()
            .map(|entry| ("Work experience", entry.title.as_str(), &entry.start, entry.end.as_ref()))
            .chain(self.education.iter().map(|entry| {
                ("Education", entry.degree.as_str(), &entry.start, entry.end.as_ref())
            }));
        for (section, label, start, end) in histories {
            let Some(end) = end else { continue };
            if start.contradicts(end) {
                let name = if label.trim().is_empty() {
                    "entry".to_string()
                } else {
                    format!("\"{}\"", label.trim())
                };
                return Some(format!(
                    "Fix the dates for {section} {name}: the start date is after the end date."
                ));
            }
        }
        None
    }

    /// The PDF presents dated histories chronologically; project files keep the
    /// user's manual order so ties and imprecise dates are never re-ordered.
    fn render_view(&self) -> Cv {
        let mut view = self.clone();
        view.experience = ordered_histories(&self.experience, |e| &e.start, |e| e.end.is_none());
        view.education = ordered_histories(&self.education, |e| &e.start, |e| e.end.is_none());
        view
    }
}

fn valid_email(email: &str) -> bool {
    let email = email.trim();
    if email.chars().any(char::is_whitespace) {
        return false;
    }
    let mut parts = email.split('@');
    match (parts.next(), parts.next(), parts.next()) {
        (Some(local), Some(domain), None) => {
            !local.is_empty()
                && !domain.is_empty()
                && domain.contains('.')
                && !domain.starts_with('.')
                && !domain.ends_with('.')
        }
        _ => false,
    }
}

fn valid_web_link(url: &str) -> bool {
    let url = url.trim();
    if url.chars().any(char::is_whitespace) {
        return false;
    }
    url.strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .is_some_and(|rest| !rest.is_empty() && !rest.starts_with('/'))
}

/// Ongoing entries with a known start first, then newest known start year, then
/// undated entries (ongoing or not) last. Within a year, months only establish
/// an order when every entry has one; otherwise the user's manual order is kept.
/// This avoids a comparator that pretends a year-only date is newer or older
/// than a month-precision one.
fn ordered_histories<T: Clone>(
    entries: &[T],
    start: impl Fn(&T) -> &CvDate,
    ongoing: impl Fn(&T) -> bool,
) -> Vec<T> {
    let mut out: Vec<T> = entries
        .iter()
        .filter(|entry| ongoing(entry) && start(entry).year.is_some())
        .cloned()
        .collect();

    let mut years: Vec<u16> = entries
        .iter()
        .filter(|entry| !ongoing(entry))
        .filter_map(|entry| start(entry).year)
        .collect();
    years.sort_unstable_by(|a, b| b.cmp(a));
    years.dedup();
    for year in years {
        let mut group: Vec<T> = entries
            .iter()
            .filter(|entry| !ongoing(entry) && start(entry).year == Some(year))
            .cloned()
            .collect();
        if group.iter().all(|entry| start(entry).month.is_some()) {
            group.sort_by_key(|entry| std::cmp::Reverse(start(entry).month));
        }
        out.extend(group);
    }

    out.extend(entries.iter().filter(|entry| start(entry).year.is_none()).cloned());
    out
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

/// What the fixed template receives: one CV and the selected structure.
/// Both are data; the template only projects sections, it never mutates content.
#[derive(Serialize)]
struct RenderInput<'a> {
    structure: Structure,
    cv: &'a Cv,
}

impl CvWorld {
    fn new(cv: &Cv, structure: Structure) -> Result<Self, String> {
        // User text travels as JSON data, never as Typst source.
        let json = serde_json::to_string(&RenderInput { structure, cv }).map_err(|e| e.to_string())?;
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

fn render_document(cv: &Cv, structure: Structure) -> Result<PagedDocument, String> {
    if let Some(problem) = cv.date_shape_problem() {
        return Err(problem);
    }
    let view = cv.render_view();
    let world = CvWorld::new(&view, structure)?;
    typst::compile::<PagedDocument>(&world)
        .output
        .map_err(|diagnostics| format!("render failed: {}", diagnostics_message(&diagnostics)))
}

pub fn render(cv: &Cv, structure: Structure) -> Result<Vec<u8>, String> {
    let document = render_document(cv, structure)?;
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

/// One independently editable CV. Structure is a presentation choice over the
/// same content; content language is stored from the first schema version even
/// while only English headings are offered.
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
    /// The exact input these bytes were rendered from: export re-checks it so the
    /// frontend cannot hand over newer content or another structure beside older
    /// artifacts. Same CV + different structure is a different document.
    structure: Structure,
    cv: Cv,
    bytes: Vec<u8>,
}

impl CachedRender {
    fn matches(&self, revision: u64, structure: Structure, cv: &Cv) -> bool {
        self.revision == revision && self.structure == structure && &self.cv == cv
    }
}

#[derive(Default)]
struct RenderCache(Mutex<Option<CachedRender>>);

#[derive(Serialize)]
pub struct ExportAck {
    path: String,
    bytes: usize,
    sha256: String,
    warnings: Vec<String>,
}

#[tauri::command]
fn render_pdf(
    cv: Cv,
    structure: Structure,
    revision: u64,
    cache: tauri::State<RenderCache>,
) -> Result<tauri::ipc::Response, String> {
    let pdf = render(&cv, structure)?;
    let mut cached = cache.0.lock().unwrap();
    // Concurrent invokes can finish out of order; keep the newest revision only.
    if cached.as_ref().is_some_and(|c| c.revision > revision) {
        return Err("stale render discarded".into());
    }
    *cached = Some(CachedRender { revision, structure, cv, bytes: pdf.clone() });
    Ok(tauri::ipc::Response::new(pdf))
}

#[tauri::command]
fn export_pdf(
    revision: u64,
    path: String,
    cv: Cv,
    structure: Structure,
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
    // Rust, not the frontend, proves these bytes belong to the validated CV and structure.
    if !cached.matches(revision, structure, &cv) {
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
        warnings: cv.export_warnings(structure),
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

    fn date(year: u16, month: Option<u8>) -> CvDate {
        CvDate { year: Some(year), month }
    }

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
                    start: date(2020, Some(1)),
                    end: Some(date(2021, Some(12))),
                    bullets: (0..bullets).map(|b| format!("Achievement {b} {}", "x".repeat(bullet_len))).collect(),
                })
                .collect(),
            ..Cv::default()
        }
    }

    /// Writes the synthetic fixtures for external inspection (pdfinfo/pdftotext).
    #[test]
    fn writes_evidence_pdfs() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.scratch/evidence");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("short.pdf"), render(&fixture(3, 60), Structure::ReverseChronological).unwrap()).unwrap();
        fs::write(dir.join("multipage.pdf"), render(&fixture(12, 140), Structure::ReverseChronological).unwrap()).unwrap();
        let mut narrow = fixture(1, 0);
        narrow.summary = format!("Long token: {} and {}", "y".repeat(200), "https://example.invalid/".to_string() + &"segment/".repeat(30));
        fs::write(dir.join("longtoken.pdf"), render(&narrow, Structure::ReverseChronological).unwrap()).unwrap();
    }

    #[test]
    fn renders_a4_multipage_pdf() {
        let document = render_document(&fixture(12, 140), Structure::ReverseChronological).unwrap();
        assert!(
            document.pages().len() > 1,
            "expected multipage output, got {}",
            document.pages().len()
        );
        let size = document.pages()[0].frame.size();
        assert!((size.x.to_pt() - 595.276).abs() < 0.1, "width {}", size.x.to_pt());
        assert!((size.y.to_pt() - 841.89).abs() < 0.1, "height {}", size.y.to_pt());
        assert!(render(&fixture(12, 140), Structure::ReverseChronological).unwrap().starts_with(b"%PDF"));
    }

    #[test]
    fn different_content_renders_different_documents() {
        let a = render(&fixture(3, 60), Structure::ReverseChronological).unwrap();
        let b = render(&fixture(12, 140), Structure::ReverseChronological).unwrap();
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
        let text = document_text(&render_document(&fixture(1, 10), Structure::ReverseChronological).unwrap());
        assert!(text.contains("Ada Lovelace"), "name missing from rendered text");
        assert!(text.contains("PROFESSIONAL SUMMARY"), "heading missing");
        assert!(text.contains("Achievement 0"), "bullet missing");
    }

    #[test]
    fn template_looking_text_stays_literal() {
        let mut cv = fixture(1, 4);
        cv.name = r#"#eval("read(\"/etc/passwd\")") *bold*"#.into();
        cv.summary = r#"#include "secret.json""#.into();
        let text = document_text(&render_document(&cv, Structure::ReverseChronological).unwrap());
        assert!(text.contains("#eval("), "authored text missing from output: {text}");
        assert!(text.contains("#include"), "authored include text missing");
        assert!(!text.contains("root:"), "user text was evaluated as code");
    }


    #[test]
    fn world_serves_only_its_declared_files() {
        let world = CvWorld::new(&fixture(1, 1), Structure::ReverseChronological).unwrap();
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

        let pdf = render(&fixture(1, 10), Structure::ReverseChronological).unwrap();
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
    fn cached_bytes_only_match_the_content_and_structure_they_were_rendered_from() {
        let cached = CachedRender {
            revision: 4,
            structure: Structure::Functional,
            cv: fixture(1, 6),
            bytes: b"%PDF".to_vec(),
        };
        assert!(
            cached.matches(4, Structure::Functional, &fixture(1, 6)),
            "same revision, structure and content must match"
        );

        let mut edited = fixture(1, 6);
        edited.summary = "Edited after the preview was rendered.".into();
        assert!(!cached.matches(4, Structure::Functional, &edited), "newer content must not be exported with older bytes");
        assert!(!cached.matches(5, Structure::Functional, &fixture(1, 6)), "a different revision must not match");
        assert!(
            !cached.matches(4, Structure::Combination, &fixture(1, 6)),
            "a structure change must not reuse the old preview bytes"
        );
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

        let text = document_text(&render_document(&cv, Structure::ReverseChronological).unwrap());
        assert!(text.contains("Ada Lovelace"), "the name still renders: {text:?}");
        assert!(!text.contains("WORK EXPERIENCE"), "empty section rendered: {text:?}");
        assert!(!text.contains("PROFESSIONAL SUMMARY"), "blank summary rendered: {text:?}");
        assert!(!text.contains("Present"), "blank entry invented an ongoing role: {text:?}");

        cv.summary = "Analytical engine programmer.".into();
        cv.experience[1].title = "Engineer".into();
        let text = document_text(&render_document(&cv, Structure::ReverseChronological).unwrap());
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
            start: date(2020, None),
            end: Some(CvDate::default()),
            ..Default::default()
        }];
        // An empty end date means "no end recorded", not an ongoing role.
        let text = document_text(&render_document(&cv, Structure::ReverseChronological).unwrap());
        assert!(text.contains("2020"), "{text:?}");
        assert!(!text.contains("Present"), "empty end became Present: {text:?}");
        assert!(!text.contains("2020 – "), "trailing separator rendered: {text:?}");

        // Only an explicit ongoing role (no end value at all) says Present.
        cv.experience[0].end = None;
        let text = document_text(&render_document(&cv, Structure::ReverseChronological).unwrap());
        assert!(text.contains("Present"), "an ongoing role still says Present: {text:?}");
    }

    // --- ticket 03: all career sections, date precision, ordering, validation ---

    fn entry(title: &str, start: CvDate, end: Option<CvDate>) -> Experience {
        Experience { title: title.into(), start, end, ..Default::default() }
    }

    #[test]
    fn mixed_date_precision_never_invents_a_month() {
        let mut cv = Cv::default();
        cv.name = "Ada Lovelace".into();
        cv.summary = "Analytical engine programmer.".into();
        cv.experience = vec![
            Experience {
                title: "Month role".into(),
                start: date(2021, Some(3)),
                end: Some(date(2022, Some(6))),
                ..Default::default()
            },
            Experience {
                title: "Year role".into(),
                start: date(2019, None),
                end: Some(CvDate::default()),
                ..Default::default()
            },
        ];

        let text = document_text(&render_document(&cv, Structure::ReverseChronological).unwrap());
        assert!(text.contains("Mar 2021 – Jun 2022"), "month precision lost: {text:?}");
        assert!(text.contains("(2019)"), "year-only start missing: {text:?}");
        assert!(!text.contains("Jan 2019"), "a missing month was materialized: {text:?}");
        assert!(!text.contains("2019 – "), "an empty end rendered a separator: {text:?}");
    }

    #[test]
    fn ongoing_entries_lead_then_newest_start_then_undated() {
        let roles = vec![
            entry("Older current", date(2018, None), None),
            entry("Undated", CvDate::default(), Some(CvDate::default())),
            entry("Newest", date(2022, Some(5)), Some(date(2022, Some(6)))),
            entry("Current", date(2019, Some(2)), None),
            entry("Older", date(2020, None), Some(date(2021, None))),
            entry("Ongoing undated", CvDate::default(), None),
        ];
        let ordered = ordered_histories(&roles, |e| &e.start, |e| e.end.is_none());
        let titles: Vec<&str> = ordered.iter().map(|e| e.title.as_str()).collect();
        // An ongoing entry with no start is still undated, so it lands last
        // where export_warnings says undated entries go.
        assert_eq!(
            titles,
            ["Older current", "Current", "Newest", "Older", "Undated", "Ongoing undated"]
        );
    }

    #[test]
    fn equal_or_imprecise_dates_keep_the_manual_order() {
        // All months known: newest month first, equal months keep manual order.
        let closed = Some(CvDate::default());
        let dated = vec![
            entry("January", date(2021, Some(1)), closed),
            entry("March", date(2021, Some(3)), closed),
            entry("Also March", date(2021, Some(3)), closed),
        ];
        let titles = |entries: Vec<Experience>| -> Vec<String> {
            entries.into_iter().map(|entry| entry.title).collect()
        };
        assert_eq!(
            titles(ordered_histories(&dated, |e| &e.start, |e| e.end.is_none())),
            ["March", "Also March", "January"]
        );

        // One year-only entry in the year: the month cannot prove an order for
        // the whole group, so every entry in that year keeps its manual order.
        let mixed = vec![
            entry("January", date(2021, Some(1)), closed),
            entry("Year only", date(2021, None), closed),
            entry("March", date(2021, Some(3)), closed),
        ];
        assert_eq!(
            titles(ordered_histories(&mixed, |e| &e.start, |e| e.end.is_none())),
            ["January", "Year only", "March"]
        );

        // Undated entries keep their relative manual order at the end.
        let undated = vec![
            entry("First undated", CvDate::default(), closed),
            entry("Second undated", CvDate::default(), closed),
        ];
        assert_eq!(
            titles(ordered_histories(&undated, |e| &e.start, |e| e.end.is_none())),
            ["First undated", "Second undated"]
        );
    }

    #[test]
    fn contradictory_ranges_block_export_but_imprecise_ones_do_not() {
        let mut cv = fixture(1, 6);
        cv.experience[0].start = date(2022, Some(1));
        cv.experience[0].end = Some(date(2021, Some(12)));
        assert!(
            cv.export_problem().unwrap().contains("start date is after the end date"),
            "a later start year must be rejected"
        );

        cv.experience[0].start = date(2021, Some(6));
        cv.experience[0].end = Some(date(2021, Some(3)));
        assert!(cv.export_problem().unwrap().contains("start date is after the end date"));

        // A year-only end cannot prove a month contradiction.
        cv.experience[0].end = Some(date(2021, None));
        assert!(cv.export_problem().is_none(), "year-only end was treated as a contradiction");

        // The same year with no months is a tie, not a contradiction.
        cv.experience[0].start = date(2021, None);
        cv.experience[0].end = Some(date(2021, None));
        assert!(cv.export_problem().is_none());

        // Ongoing roles never contradict themselves.
        cv.experience[0].start = date(2024, Some(2));
        cv.experience[0].end = None;
        assert!(cv.export_problem().is_none());
    }

    #[test]
    fn malformed_supplied_email_or_links_block_export() {
        let mut cv = fixture(1, 6);

        cv.email = "ada@".into();
        assert!(cv.export_problem().unwrap().contains("email address is not valid"));
        cv.email = "ada lovelace@example.invalid".into();
        assert!(cv.export_problem().unwrap().contains("email address is not valid"));
        cv.email = "ada@example".into();
        assert!(cv.export_problem().unwrap().contains("email address is not valid"));
        cv.email = "ada@example.invalid".into();
        assert!(cv.export_problem().is_none(), "a structurally valid email was rejected");

        // Clearing an optional email is allowed: the draft stays incomplete.
        cv.email = "   ".into();
        assert!(cv.export_problem().is_none(), "empty email must not block export");
        cv.email = "ada@example.invalid".into();

        cv.links = vec![Link { label: "Portfolio".into(), url: "example.invalid".into() }];
        assert!(cv.export_problem().unwrap().contains("must start with http:// or https://"));
        cv.links[0].url = "https://example.invalid/a b".into();
        assert!(cv.export_problem().unwrap().contains("must start with http:// or https://"));
        cv.links[0].url = "https://example.invalid/portfolio".into();
        assert!(cv.export_problem().is_none(), "a valid link was rejected");
    }

    #[test]
    fn every_career_section_reaches_the_pdf_in_order() {
        let mut cv = fixture(1, 6);
        cv.education = vec![Education {
            school: "Test University".into(),
            degree: "BSc Testing".into(),
            location: "Testville".into(),
            start: date(2016, Some(8)),
            end: Some(date(2019, Some(5))),
            bullets: vec!["Graduated with honours.".into()],
        }];
        cv.skills = vec![SkillGroup {
            name: "Test automation".into(),
            bullets: vec!["Wrote regression checks.".into()],
        }];
        cv.projects = vec![ProjectEntry {
            name: "Synthetic project".into(),
            detail: "Local tooling".into(),
            bullets: vec!["Built a local renderer.".into()],
        }];
        cv.certifications = vec![Certification {
            name: "Certified tester".into(),
            issuer: "Test board".into(),
            date: date(2023, None),
        }];
        cv.languages = vec![Language { name: "Indonesian".into(), level: "Native".into() }];

        let text = document_text(&render_document(&cv, Structure::ReverseChronological).unwrap());
        for expected in [
            "PROFESSIONAL SUMMARY",
            "WORK EXPERIENCE",
            "EDUCATION",
            "BSc Testing",
            "Test University",
            "Aug 2016 – May 2019",
            "Graduated with honours.",
            "SKILLS",
            "Test automation",
            "Wrote regression checks.",
            "PROJECTS",
            "Synthetic project",
            "Built a local renderer.",
            "CERTIFICATIONS",
            "Certified tester",
            "Test board",
            "(2023)",
            "LANGUAGES",
            "Indonesian — Native",
        ] {
            assert!(text.contains(expected), "missing {expected:?} in rendered text: {text:?}");
        }

        let mut position = 0;
        for heading in [
            "PROFESSIONAL SUMMARY",
            "WORK EXPERIENCE",
            "EDUCATION",
            "SKILLS",
            "PROJECTS",
            "CERTIFICATIONS",
            "LANGUAGES",
        ] {
            let found = text.find(heading).unwrap_or_else(|| panic!("{heading} missing"));
            assert!(found >= position, "{heading} is out of order");
            position = found;
        }
    }

    #[test]
    fn blank_new_section_entries_do_not_render() {
        let mut cv = Cv::default();
        cv.name = "Ada Lovelace".into();
        cv.summary = "Analytical engine programmer.".into();
        cv.education = vec![Education::default()];
        cv.skills = vec![SkillGroup::default()];
        cv.projects = vec![ProjectEntry::default()];
        cv.certifications = vec![Certification::default()];
        cv.languages = vec![Language::default(), Language { level: "  ".into(), ..Default::default() }];

        let text = document_text(&render_document(&cv, Structure::ReverseChronological).unwrap());
        for heading in ["EDUCATION", "SKILLS", "PROJECTS", "CERTIFICATIONS", "LANGUAGES"] {
            assert!(!text.contains(heading), "blank {heading} section rendered: {text:?}");
        }

        cv.skills[0].name = "Test automation".into();
        let text = document_text(&render_document(&cv, Structure::ReverseChronological).unwrap());
        assert!(text.contains("SKILLS") && text.contains("Test automation"));
        assert!(!text.contains("EDUCATION"), "still-blank sections stayed blank");
    }

    #[test]
    fn all_career_sections_survive_a_saved_project_file() {
        let dir = temp_dir("sections");
        let mut project = project();
        project.cv.education = vec![Education {
            school: "Test University".into(),
            degree: "BSc Testing".into(),
            location: String::new(),
            start: date(2016, Some(8)),
            end: Some(date(2019, None)),
            bullets: vec!["Graduated with honours.".into()],
        }];
        project.cv.skills = vec![SkillGroup { name: "Test automation".into(), bullets: vec!["Checks.".into()] }];
        project.cv.projects = vec![ProjectEntry { name: "Synthetic project".into(), detail: String::new(), bullets: vec![] }];
        project.cv.certifications = vec![Certification { name: "Certified tester".into(), issuer: String::new(), date: date(2023, None) }];
        project.cv.languages = vec![Language { name: "Indonesian".into(), level: "Native".into() }];
        project.cv.links = vec![Link { label: "Portfolio".into(), url: "https://example.invalid/portfolio".into() }];

        write_project(&dir, &project).unwrap();
        let reopened = read_project(&dir, &project.id).unwrap();

        assert_eq!(reopened.cv.education[0].start, date(2016, Some(8)), "month precision lost on disk");
        assert_eq!(reopened.cv.education[0].end, Some(date(2019, None)), "year-only end gained a month");
        assert_eq!(reopened.cv.skills[0].bullets, ["Checks."]);
        assert_eq!(reopened.cv.projects[0].name, "Synthetic project");
        assert_eq!(reopened.cv.certifications[0].date, date(2023, None));
        assert_eq!(reopened.cv.languages[0].level, "Native");
        assert_eq!(reopened.cv.links[0].url, "https://example.invalid/portfolio");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn invalid_date_shapes_are_refused_before_rendering() {
        let mut cv = fixture(1, 6);

        // Hand-edited or imported data: a month outside 1-12.
        cv.experience[0].start = CvDate { year: Some(2020), month: Some(13) };
        assert!(
            cv.export_problem().unwrap().contains("between 1 and 12"),
            "an out-of-range month must block export"
        );
        assert!(
            render_document(&cv, Structure::ReverseChronological).is_err(),
            "an out-of-range month must not reach the template"
        );

        // A month without a year cannot be rendered meaningfully either.
        cv.experience[0].start = CvDate { year: None, month: Some(5) };
        assert!(cv.export_problem().unwrap().contains("between 1 and 12"));
        assert!(render_document(&cv, Structure::ReverseChronological).is_err());

        cv.experience[0].start = date(2020, Some(12));
        assert!(cv.export_problem().is_none(), "a valid date must still export");
        assert!(render_document(&cv, Structure::ReverseChronological).is_ok());
    }

    #[test]
    fn every_section_survives_a_multipage_pdf() {
        // Enough authored content to paginate, all sections present in one document.
        let mut cv = fixture(6, 140);
        cv.education = vec![Education {
            school: "Test University".into(),
            degree: "BSc Testing".into(),
            location: "Testville".into(),
            start: date(2016, Some(8)),
            end: Some(date(2019, None)),
            bullets: vec!["Graduated with honours.".into()],
        }];
        cv.skills = vec![SkillGroup {
            name: "Test automation".into(),
            bullets: vec!["Wrote regression checks.".into()],
        }];
        cv.projects = vec![ProjectEntry {
            name: "Synthetic project".into(),
            detail: "Local tooling".into(),
            bullets: vec!["Built a local renderer.".into()],
        }];
        cv.certifications = vec![Certification {
            name: "Certified tester".into(),
            issuer: "Test board".into(),
            date: date(2023, None),
        }];
        cv.languages = vec![Language { name: "Indonesian".into(), level: "Native".into() }];

        let document = render_document(&cv, Structure::ReverseChronological).unwrap();
        assert!(document.pages().len() > 1, "expected multipage output, got {}", document.pages().len());

        let text = document_text(&document);
        for expected in [
            "PROFESSIONAL SUMMARY",
            "WORK EXPERIENCE",
            "EDUCATION",
            "SKILLS",
            "PROJECTS",
            "CERTIFICATIONS",
            "LANGUAGES",
            "BSc Testing",
            "Test automation",
            "Synthetic project",
            "Certified tester",
            "Indonesian — Native",
            "Achievement 5",
        ] {
            assert!(text.contains(expected), "missing {expected:?} in multipage text");
        }

        // External pdftotext/pdfinfo evidence for the acceptance notes.
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.scratch/evidence/ticket03");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("experienced-multipage.pdf"), render(&cv, Structure::ReverseChronological).unwrap()).unwrap();
    }

    #[test]
    fn fresh_graduate_cv_exports_without_employment() {
        let cv = Cv {
            name: "Synthetic Graduate".into(),
            email: "graduate@example.invalid".into(),
            location: "Testville".into(),
            summary: "Synthetic fresh graduate.".into(),
            education: vec![Education {
                school: "Test University".into(),
                degree: "BSc Testing".into(),
                start: date(2022, None),
                end: Some(date(2025, None)),
                ..Default::default()
            }],
            projects: vec![ProjectEntry {
                name: "Synthetic project".into(),
                detail: "Coursework".into(),
                bullets: vec!["Built a fixture project.".into()],
            }],
            ..Cv::default()
        };
        assert!(cv.export_problem().is_none(), "education and projects are substantive content");

        let document = render_document(&cv, Structure::ReverseChronological).unwrap();
        assert_eq!(document.pages().len(), 1);
        let text = document_text(&document);
        assert!(!text.contains("WORK EXPERIENCE"), "fresh graduate invented work history: {text:?}");
        for expected in ["EDUCATION", "BSc Testing", "PROJECTS", "Synthetic project"] {
            assert!(text.contains(expected), "missing {expected:?} in fresh-graduate text");
        }

        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.scratch/evidence/ticket03");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("fresh-graduate.pdf"), render(&cv, Structure::ReverseChronological).unwrap()).unwrap();
    }

    #[test]
    fn export_warnings_name_missing_optional_content() {
        let mut cv = fixture(1, 6);
        assert!(cv.export_warnings(Structure::ReverseChronological).is_empty(), "a complete CV warns about nothing");

        cv.email = String::new();
        cv.phone = String::new();
        assert!(
            cv.export_warnings(Structure::ReverseChronological)
                .iter()
                .any(|w| w.contains("no way to contact you")),
            "a CV without contact details must warn"
        );

        cv.phone = "555-0100".into();
        assert!(cv.export_warnings(Structure::ReverseChronological).is_empty(), "one contact method is enough");

        cv.experience[0].start = CvDate::default();
        assert!(
            cv.export_warnings(Structure::ReverseChronological)
                .iter()
                .any(|w| w.contains("no start date")),
            "an included entry without a start date must warn"
        );
    }

    // --- ticket 04: structure projections, retained content, skill warnings ---

    /// One CV that exercises the difference between the three projections:
    /// named skill groups with evidence, a detailed role, and a supporting location.
    fn structure_fixture() -> Cv {
        Cv {
            name: "Synthetic Candidate".into(),
            email: "candidate@example.invalid".into(),
            summary: "Synthetic summary.".into(),
            experience: vec![Experience {
                title: "Synthetic Engineer".into(),
                company: "Example Employer".into(),
                location: "Testville".into(),
                start: date(2020, Some(1)),
                end: Some(date(2022, Some(6))),
                bullets: vec!["Employment achievement bullet.".into()],
            }],
            skills: vec![
                SkillGroup {
                    name: "Synthetic skill group".into(),
                    bullets: vec!["Skill achievement bullet.".into()],
                },
                SkillGroup {
                    name: "Second named group".into(),
                    bullets: vec!["Second skill bullet.".into()],
                },
            ],
            ..Cv::default()
        }
    }

    #[test]
    fn functional_leads_with_skills_and_compacts_employment() {
        let text = document_text(&render_document(&structure_fixture(), Structure::Functional).unwrap());

        let skills = text.find("SKILLS").unwrap_or_else(|| panic!("SKILLS missing: {text:?}"));
        let work = text
            .find("WORK EXPERIENCE")
            .unwrap_or_else(|| panic!("WORK EXPERIENCE missing: {text:?}"));
        assert!(skills < work, "functional must lead with skill evidence: {text:?}");

        for expected in [
            "Synthetic skill group",
            "Skill achievement bullet.",
            "Second named group",
            "Second skill bullet.",
            "Synthetic Engineer",
            "Example Employer",
            "Jan 2020 – Jun 2022",
        ] {
            assert!(text.contains(expected), "missing {expected:?} in functional text: {text:?}");
        }

        assert!(
            !text.contains("Employment achievement bullet."),
            "functional employment must stay compact: {text:?}"
        );
        assert!(!text.contains("Testville"), "compact employment omits location: {text:?}");
    }

    #[test]
    fn combination_leads_with_skills_and_keeps_detailed_employment() {
        let text = document_text(&render_document(&structure_fixture(), Structure::Combination).unwrap());

        let skills = text.find("SKILLS").unwrap_or_else(|| panic!("SKILLS missing: {text:?}"));
        let work = text
            .find("WORK EXPERIENCE")
            .unwrap_or_else(|| panic!("WORK EXPERIENCE missing: {text:?}"));
        assert!(skills < work, "combination must lead with skill evidence: {text:?}");

        for expected in [
            "Skill achievement bullet.",
            "Employment achievement bullet.",
            "Synthetic Engineer",
            "Example Employer",
            "Testville",
            "Jan 2020 – Jun 2022",
        ] {
            assert!(text.contains(expected), "missing {expected:?} in combination text: {text:?}");
        }
    }

    #[test]
    fn reverse_chronological_keeps_detailed_employment_before_supporting_skills() {
        let text =
            document_text(&render_document(&structure_fixture(), Structure::ReverseChronological).unwrap());

        let work = text
            .find("WORK EXPERIENCE")
            .unwrap_or_else(|| panic!("WORK EXPERIENCE missing: {text:?}"));
        let skills = text.find("SKILLS").unwrap_or_else(|| panic!("SKILLS missing: {text:?}"));
        assert!(work < skills, "reverse chronological leads with employment: {text:?}");

        for expected in [
            "Employment achievement bullet.",
            "Skill achievement bullet.",
            "Testville",
        ] {
            assert!(text.contains(expected), "missing {expected:?} in reverse text: {text:?}");
        }
    }

    #[test]
    fn functional_hides_entries_with_no_compact_fields() {
        let mut cv = structure_fixture();
        cv.experience = vec![Experience {
            title: String::new(),
            company: String::new(),
            location: "Hidden location".into(),
            start: CvDate::default(),
            end: Some(CvDate::default()),
            bullets: vec!["Hidden-only achievement.".into()],
        }];
        cv.skills = vec![SkillGroup {
            name: "Visible group".into(),
            bullets: vec!["Visible evidence.".into()],
        }];

        // The only role has no title, employer, or start date: compact output
        // has nothing to show, so it must not emit an empty heading or block.
        let functional = document_text(&render_document(&cv, Structure::Functional).unwrap());
        assert!(
            !functional.contains("WORK EXPERIENCE"),
            "functional rendered a blank employment section: {functional:?}"
        );
        assert!(
            !functional.contains("Hidden-only achievement."),
            "functional leaked a hidden bullet: {functional:?}"
        );

        // Reverse chronological still shows the same entry, because its location
        // and bullets are visible in that structure.
        let reverse = document_text(&render_document(&cv, Structure::ReverseChronological).unwrap());
        for expected in ["WORK EXPERIENCE", "Hidden location", "Hidden-only achievement."] {
            assert!(reverse.contains(expected), "reverse lost {expected:?}: {reverse:?}");
        }

        // A hidden-only draft beside a real role must not add an empty row.
        cv.experience.push(Experience {
            title: "Visible Engineer".into(),
            company: "Visible Employer".into(),
            start: date(2023, None),
            ..Default::default()
        });
        let functional = document_text(&render_document(&cv, Structure::Functional).unwrap());
        assert!(functional.contains("WORK EXPERIENCE"));
        assert_eq!(functional.matches("Visible Engineer").count(), 1);
        assert!(!functional.contains("Hidden location"), "functional leaked a hidden location: {functional:?}");
    }

    #[test]
    fn one_cv_renders_a_different_document_per_structure() {
        let cv = structure_fixture();
        let reverse = render(&cv, Structure::ReverseChronological).unwrap();
        let functional = render(&cv, Structure::Functional).unwrap();
        let combination = render(&cv, Structure::Combination).unwrap();

        assert_ne!(reverse, functional, "structure did not change the document");
        assert_ne!(functional, combination, "structure did not change the document");
        assert_ne!(reverse, combination, "structure did not change the document");
    }

    /// Writes the synthetic short and multipage artifacts for external
    /// pdfinfo/pdftotext inspection, and proves each structure paginates.
    #[test]
    fn writes_structure_evidence_pdfs() {
        let mut long = structure_fixture();
        long.experience = (0..6)
            .map(|i| Experience {
                title: format!("Synthetic Engineer {i}"),
                company: "Example Employer".into(),
                location: "Testville".into(),
                start: date(2016 + i, Some(1)),
                end: Some(date(2017 + i, Some(12))),
                bullets: (0..4)
                    .map(|b| format!("Employment detail {i}.{b} {}", "x".repeat(90)))
                    .collect(),
            })
            .collect();
        long.skills = (0..8)
            .map(|i| SkillGroup {
                name: format!("Synthetic skill group {i}"),
                bullets: (0..3)
                    .map(|b| format!("Skill evidence {i}.{b} {}", "y".repeat(120)))
                    .collect(),
            })
            .collect();

        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.scratch/evidence/ticket04");
        fs::create_dir_all(&dir).unwrap();
        let structures = [
            ("reverse-chronological", Structure::ReverseChronological),
            ("functional", Structure::Functional),
            ("combination", Structure::Combination),
        ];

        for (name, cv) in [("short", structure_fixture()), ("multipage", long)] {
            for (slug, structure) in structures {
                let document = render_document(&cv, structure).unwrap();
                if name == "multipage" {
                    assert!(
                        document.pages().len() > 1,
                        "{slug} should paginate the multipage fixture, got {}",
                        document.pages().len()
                    );
                }
                fs::write(dir.join(format!("{name}-{slug}.pdf")), render(&cv, structure).unwrap()).unwrap();
            }
        }
    }

    #[test]
    fn switching_structure_preserves_content_through_save_and_reopen() {
        let dir = temp_dir("structures");
        let mut project = project();
        project.cv = structure_fixture();
        write_project(&dir, &project).unwrap();
        let original = read_project(&dir, &project.id).unwrap();

        for structure in [Structure::Functional, Structure::Combination, Structure::ReverseChronological] {
            let mut switched = read_project(&dir, &project.id).unwrap();
            switched.structure = structure;
            let stored = write_project(&dir, &switched).unwrap();

            assert_eq!(stored.structure, structure);
            assert_eq!(stored.cv, original.cv, "switching to {structure:?} rewrote CV content");
            assert_eq!(
                read_project(&dir, &project.id).unwrap().cv,
                original.cv,
                "reopening after {structure:?} lost hidden content"
            );
        }

        // The hidden employment bullet is still on disk after the functional round-trip.
        let json = fs::read_to_string(dir.join(format!("{}.json", project.id))).unwrap();
        assert!(json.contains("Employment achievement bullet."), "hidden content was deleted: {json}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn skill_group_order_and_bullets_survive_save_and_reopen() {
        let dir = temp_dir("skills-order");
        let mut project = project();
        project.cv.skills = vec![
            SkillGroup {
                name: "Second named group".into(),
                bullets: vec!["Second bullet.".into(), "Third bullet.".into()],
            },
            SkillGroup { name: "First named group".into(), bullets: vec!["First bullet.".into()] },
        ];

        write_project(&dir, &project).unwrap();
        let reopened = read_project(&dir, &project.id).unwrap();

        let names: Vec<&str> = reopened.cv.skills.iter().map(|group| group.name.as_str()).collect();
        assert_eq!(names, ["Second named group", "First named group"], "manual group order lost");
        assert_eq!(reopened.cv.skills[0].bullets, ["Second bullet.", "Third bullet."]);
        assert_eq!(reopened.cv.skills[1].bullets, ["First bullet."]);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_skill_achievements_warn_only_for_leading_structures() {
        let mut cv = structure_fixture();
        cv.skills = vec![SkillGroup { name: "Named but no evidence".into(), bullets: vec!["  ".into()] }];

        assert!(
            !cv.export_warnings(Structure::ReverseChronological)
                .iter()
                .any(|w| w.contains("skill group")),
            "reverse chronological does not lead with skills, so it must not warn"
        );

        for structure in [Structure::Functional, Structure::Combination] {
            let warnings = cv.export_warnings(structure);
            assert!(
                warnings.iter().any(|w| w.contains("skill-group achievements")),
                "{structure:?} must warn about missing evidence: {warnings:?}"
            );
            assert!(
                cv.export_problem().is_none(),
                "missing skill evidence must stay a warning, not block export"
            );
        }

        cv.skills[0].bullets = vec!["Evidence now exists.".into()];
        for structure in [Structure::Functional, Structure::Combination] {
            assert!(
                !cv.export_warnings(structure).iter().any(|w| w.contains("skill-group achievements")),
                "{structure:?} must stop warning once evidence exists"
            );
        }
    }
}
