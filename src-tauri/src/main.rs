use std::fs;
use std::io::Read;
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
pub struct LanguageEntry {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub level: String,
}

impl LanguageEntry {
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
    pub languages: Vec<LanguageEntry>,
}

impl Cv {
    /// Export needs an owner name and one substantive non-contact section.
    /// Messages follow the interface language, never the CV content language.
    fn export_problem(&self, lang: Language) -> Option<String> {
        if self.name.trim().is_empty() {
            return Some(lang.pick(
                "Add the CV owner's full name before exporting.",
                "Tambahkan nama lengkap pemilik CV sebelum mengekspor.",
            ));
        }
        let substantive = !self.summary.trim().is_empty()
            || self.experience.iter().any(|e| !e.is_blank())
            || self.education.iter().any(|e| !e.is_blank())
            || self.skills.iter().any(|s| !s.is_blank())
            || self.projects.iter().any(|p| !p.is_blank())
            || self.certifications.iter().any(|c| !c.is_blank())
            || self.languages.iter().any(|l| !l.is_blank());
        if !substantive {
            return Some(lang.pick(
                "Add a professional summary or at least one work experience, education, skill, \
                 project, certification, or language entry before exporting.",
                "Tambahkan ringkasan profesional atau setidaknya satu entri pengalaman kerja, \
                 pendidikan, keahlian, proyek, sertifikasi, atau bahasa sebelum mengekspor.",
            ));
        }
        if !self.email.trim().is_empty() && !valid_email(&self.email) {
            return Some(lang.pick(
                "The email address is not valid. Fix it or clear it before exporting.",
                "Alamat email tidak valid. Perbaiki atau kosongkan sebelum mengekspor.",
            ));
        }
        if let Some(link) = self.links.iter().find(|l| !l.url.trim().is_empty() && !valid_web_link(&l.url)) {
            let label = if link.label.trim().is_empty() {
                lang.pick("a link", "sebuah tautan")
            } else {
                format!("\"{}\"", link.label.trim())
            };
            return Some(lang.pick(
                format!(
                    "The web link for {label} is not valid: it must start with http:// or \
                     https:// and contain no spaces. Fix it or clear it before exporting."
                ),
                format!(
                    "Tautan web untuk {label} tidak valid: harus diawali http:// atau https:// \
                     dan tidak boleh mengandung spasi. Perbaiki atau kosongkan sebelum mengekspor."
                ),
            ));
        }
        if let Some(problem) = self.date_shape_problem(lang) {
            return Some(problem);
        }
        self.date_problem(lang)
    }

    /// At least one skill group carries a user-authored achievement bullet.
    /// A named group with no evidence is not a functional/combination lead.
    fn has_skill_achievements(&self) -> bool {
        self.skills
            .iter()
            .any(|group| group.bullets.iter().any(|bullet| !bullet.trim().is_empty()))
    }

    /// Warnings never block export: they name optional content the PDF will lack.
    /// They follow the interface language, independently of the CV content.
    fn export_warnings(&self, lang: Language, structure: Structure) -> Vec<String> {
        let mut warnings = Vec::new();
        if matches!(structure, Structure::Functional | Structure::Combination)
            && !self.has_skill_achievements()
        {
            let (name, id_name) = if structure == Structure::Functional {
                ("functional", "fungsional")
            } else {
                ("combination", "kombinasi")
            };
            warnings.push(lang.pick(
                format!(
                    "The {name} structure relies on skill-group achievements, but no skill group \
                     has an achievement bullet yet. Add achievements or choose another structure."
                ),
                format!(
                    "Struktur {id_name} mengandalkan pencapaian kelompok keahlian, tetapi belum ada \
                     kelompok keahlian yang memiliki butir pencapaian. Tambahkan pencapaian atau \
                     pilih struktur lain."
                ),
            ));
        }
        if self.email.trim().is_empty() && self.phone.trim().is_empty() {
            warnings.push(lang.pick(
                "No email or phone number is included, so the PDF gives employers no way to contact you.",
                "Tidak ada alamat email atau nomor telepon, sehingga pemberi kerja tidak memiliki \
                 cara untuk menghubungi Anda.",
            ));
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
            warnings.push(lang.pick(
                "Some entries have no start date, so they are listed after dated entries.",
                "Beberapa entri tidak memiliki tanggal mulai, sehingga ditampilkan setelah entri \
                 yang bertanggal.",
            ));
        }
        warnings
    }

    /// A month outside 1-12, or a month without a year, is corrupt data rather
    /// than user input; refuse it before it reaches the template.
    fn date_shape_problem(&self, lang: Language) -> Option<String> {
        let invalid = self
            .experience
            .iter()
            .map(|entry| ("work experience", "pengalaman kerja", &entry.start, entry.end.as_ref()))
            .chain(self.education.iter().map(|entry| {
                ("education", "pendidikan", &entry.start, entry.end.as_ref())
            }))
            .chain(
                self.certifications
                    .iter()
                    .map(|entry| ("certification", "sertifikasi", &entry.date, None)),
            )
            .find(|(_, _, start, end)| {
                !start.shape_is_valid() || end.is_some_and(|end| !end.shape_is_valid())
            });
        invalid.map(|(section, id_section, _, _)| {
            lang.pick(
                format!(
                    "Fix the dates for the {section}: a month must be between 1 and 12 and needs a year."
                ),
                format!(
                    "Perbaiki tanggal pada {id_section}: bulan harus antara 1 dan 12 dan memerlukan tahun."
                ),
            )
        })
    }

    fn date_problem(&self, lang: Language) -> Option<String> {
        let histories = self
            .experience
            .iter()
            .map(|entry| {
                (
                    "Work experience",
                    "Pengalaman kerja",
                    entry.title.as_str(),
                    &entry.start,
                    entry.end.as_ref(),
                )
            })
            .chain(self.education.iter().map(|entry| {
                (
                    "Education",
                    "Pendidikan",
                    entry.degree.as_str(),
                    &entry.start,
                    entry.end.as_ref(),
                )
            }));
        for (section, id_section, label, start, end) in histories {
            let Some(end) = end else { continue };
            if start.contradicts(end) {
                let name = if label.trim().is_empty() {
                    lang.pick("entry", "entri")
                } else {
                    format!("\"{}\"", label.trim())
                };
                return Some(lang.pick(
                    format!(
                        "Fix the dates for {section} {name}: the start date is after the end date."
                    ),
                    format!(
                        "Perbaiki tanggal untuk {id_section} {name}: tanggal mulai lebih akhir \
                         daripada tanggal selesai."
                    ),
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

/// What the fixed template receives: one CV, the selected structure, and the
/// content language that decides the standard headings and Present wording.
/// All of it is data; the template only projects sections, it never mutates content.
#[derive(Serialize)]
struct RenderInput<'a> {
    structure: Structure,
    language: Language,
    cv: &'a Cv,
}

impl CvWorld {
    fn new(cv: &Cv, structure: Structure, language: Language) -> Result<Self, String> {
        // User text travels as JSON data, never as Typst source.
        let json = serde_json::to_string(&RenderInput { structure, language, cv }).map_err(|e| e.to_string())?;
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

fn render_document(cv: &Cv, structure: Structure, language: Language) -> Result<PagedDocument, String> {
    if let Some(problem) = cv.date_shape_problem(language) {
        return Err(problem);
    }
    let view = cv.render_view();
    let world = CvWorld::new(&view, structure, language)?;
    typst::compile::<PagedDocument>(&world)
        .output
        .map_err(|diagnostics| format!("render failed: {}", diagnostics_message(&diagnostics)))
}

pub fn render(cv: &Cv, structure: Structure, language: Language) -> Result<Vec<u8>, String> {
    let document = render_document(cv, structure, language)?;
    typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())
        .map_err(|diagnostics| format!("pdf export failed: {}", diagnostics_message(&diagnostics)))
}

/// Rendering itself follows the CV content language, but a validation failure
/// is interface text: the person reading it picked the interface language.
fn render_for(
    cv: &Cv,
    structure: Structure,
    content_language: Language,
    interface_language: Language,
) -> Result<Vec<u8>, String> {
    if let Some(problem) = cv.date_shape_problem(interface_language) {
        return Err(problem);
    }
    render(cv, structure, content_language)
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

/// Content and interface language. Both are user choices, independently
/// selectable: one decides PDF headings, the other decides the interface text.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Language {
    #[default]
    English,
    Indonesian,
}

impl Language {
    /// One message, two languages; `String` because messages carry values.
    fn pick(self, english: impl Into<String>, indonesian: impl Into<String>) -> String {
        match self {
            Language::English => english.into(),
            Language::Indonesian => indonesian.into(),
        }
    }
}

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
    id: String,
    title: String,
    name: String,
    updated_at: u64,
    /// The current file is unreadable. The CV stays listed so opening it can
    /// recover the previous revision or report why it cannot.
    damaged: bool,
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

/// One running application owns the library, so two windows cannot race two
/// writers against the same projects. The OS releases this lock on process exit,
/// so a crash never leaves a stale lock behind.
#[derive(Debug)]
struct InstanceLock(#[allow(dead_code)] fs::File);

fn instance_lock(dir: &Path) -> Result<InstanceLock, String> {
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
fn is_valid_project_id(id: &str) -> bool {
    !id.is_empty() && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}

fn project_file(dir: &Path, id: &str, suffix: &str) -> Result<PathBuf, String> {
    if !is_valid_project_id(id) {
        return Err("invalid project id".into());
    }
    Ok(dir.join(format!("{id}{suffix}.json")))
}

fn project_path(dir: &Path, id: &str) -> Result<PathBuf, String> {
    project_file(dir, id, "")
}

/// The one rolling previous valid revision. Its dotted name is not a project id,
/// so the library listing never mistakes it (or a preserved damaged file) for a CV.
fn previous_path(dir: &Path, id: &str) -> Result<PathBuf, String> {
    project_file(dir, id, ".prev")
}

/// Test-only control over the replacement sequence. The running application
/// always passes the default; checks choose the failure they need.
#[derive(Clone, Copy, Default)]
struct Faults {
    /// The library refuses the write (read-only folder, full disk).
    deny_write: bool,
    /// The replacement stops after the previous revision was rotated in.
    interrupt_replace: bool,
}

/// A fresh identity for a new project or a duplicate. Lowercase hex only, as
/// documented: nanosecond clock concatenated with a process counter cannot
/// repeat inside one process; ids never come from user input.
fn new_project_id() -> String {
    format!("{:x}{:013x}", now_nanos(), NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
}

fn now_nanos() -> u128 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0)
}

static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

#[derive(Debug)]
enum ReadError {
    /// Missing or unparseable: recovery from the previous revision applies.
    Damaged(String),
    /// Readable but refused by this app: never rewritten or rolled back.
    Refused(Refusal),
}

#[derive(Debug, Clone, Copy)]
enum Refusal {
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

fn read_project_file(path: &Path) -> Result<Project, ReadError> {
    let text = fs::read_to_string(path)
        .map_err(|e| ReadError::Damaged(format!("could not open project: {e}")))?;
    let project: Project = serde_json::from_str(&text)
        .map_err(|e| ReadError::Damaged(format!("project file is unreadable: {e}")))?;
    if project.version != SCHEMA_VERSION {
        return Err(ReadError::Refused(Refusal::Version(project.version)));
    }
    Ok(project)
}

fn read_project(dir: &Path, id: &str) -> Result<Project, ReadError> {
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
fn rotate_previous(current: &Path, previous: &Path) -> Result<(), String> {
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
fn write_project_in(
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
    // A readable file this app refuses is never overwritten by older content.
    let preserved = match read_project_file(&current) {
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

fn write_project(dir: &Path, project: &Project) -> Result<Project, String> {
    Ok(write_project_in(dir, project, Faults::default())?.0)
}

/// Interface notice for a save that kept a damaged file. Never silent: the
/// editor shows it beside the Saved state.
fn damage_notice(path: &Path, lang: Language) -> String {
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
fn summaries_in(dir: &Path) -> Result<Vec<ProjectSummary>, String> {
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
        let (title, name, updated_at, damaged) = match read_project(dir, id) {
            Ok(project) => (project.title, project.cv.name, project.updated_at, false),
            Err(error) => {
                // The row stays so it can be opened and reported; its previous
                // revision supplies the label when that copy is readable.
                let label = read_project_file(&previous_path(dir, id)?)
                    .map(|previous| (previous.title, previous.cv.name, previous.updated_at))
                    .unwrap_or_default();
                let damaged = matches!(error, ReadError::Damaged(_));
                (label.0, label.1, label.2, damaged)
            }
        };
        out.push(ProjectSummary { id: id.to_string(), title, name, updated_at, damaged });
    }
    out.sort_by_key(|project| std::cmp::Reverse(project.updated_at));
    Ok(out)
}

#[tauri::command]
fn list_projects(app: tauri::AppHandle) -> Result<Vec<ProjectSummary>, String> {
    summaries_in(&projects_dir(&app)?)
}

#[tauri::command]
fn create_project(app: tauri::AppHandle) -> Result<Project, String> {
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
fn duplicate_in(dir: &Path, id: &str) -> Result<Project, String> {
    let mut copy = read_project(dir, id).map_err(|e| e.to_string())?;
    copy.id = new_project_id();
    write_project(dir, &copy)
}

#[tauri::command]
fn duplicate_project(app: tauri::AppHandle, id: String) -> Result<Project, String> {
    duplicate_in(&projects_dir(&app)?, &id)
}

/// What opening a CV means when the saved file was damaged: the notice the
/// interface shows and where the unreadable file was kept for inspection.
#[derive(Debug, Serialize)]
pub struct Recovery {
    notice: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    preserved_as: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct OpenedProject {
    project: Project,
    #[serde(skip_serializing_if = "Option::is_none")]
    recovery: Option<Recovery>,
}

/// Keep a copy of the unreadable current file before recovery or a save replaces
/// it. Copy, not rename: if the replacement write fails, the CV must stay at its
/// current path so the library still lists it. The name is not a project id, so
/// the library listing skips it.
fn preserve_corrupt(dir: &Path, id: &str) -> Result<Option<PathBuf>, String> {
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
fn recover_in(dir: &Path, id: &str, lang: Language) -> Result<OpenedProject, String> {
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

fn open_in(dir: &Path, id: &str, lang: Language) -> Result<OpenedProject, String> {
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
fn open_project(
    app: tauri::AppHandle,
    id: String,
    interface_language: Language,
) -> Result<OpenedProject, String> {
    open_in(&projects_dir(&app)?, &id, interface_language)
}

#[derive(Serialize)]
pub struct SaveResult {
    project: Project,
    /// Present when the save kept a damaged file it found while writing.
    #[serde(skip_serializing_if = "Option::is_none")]
    notice: Option<String>,
}

#[tauri::command]
fn save_project(
    app: tauri::AppHandle,
    project: Project,
    interface_language: Language,
) -> Result<SaveResult, String> {
    let (stored, preserved) = write_project_in(&projects_dir(&app)?, &project, Faults::default())?;
    Ok(SaveResult {
        project: stored,
        notice: preserved.map(|path| damage_notice(&path, interface_language)),
    })
}

// ---------------------------------------------------------------------------
// Project backups: portable, unencrypted snapshots that import as a new project
// ---------------------------------------------------------------------------

const BACKUP_FORMAT: &str = "cv-generator-backup";
const BACKUP_VERSION: u32 = 1;
/// A CV is kilobytes of text. This bounds the read and parse before any decoding;
/// export refuses to write a file import would refuse to read.
const MAX_BACKUP_BYTES: usize = 2 * 1024 * 1024;

/// A backup carries no id, path, or timestamp: identity belongs to the library.
/// `deny_unknown_fields` rejects a file that tries to bring one.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Backup {
    format: String,
    version: u32,
    #[serde(default)]
    title: String,
    #[serde(default)]
    structure: Structure,
    #[serde(default)]
    content_language: Language,
    /// Required: a file without content is not a snapshot of a project.
    cv: Cv,
}

/// Both directions refuse the same size; export says so before writing anything.
fn within_backup_limit(len: usize, lang: Language) -> Result<(), String> {
    if len <= MAX_BACKUP_BYTES {
        return Ok(());
    }
    let limit = MAX_BACKUP_BYTES;
    Err(lang.pick(
        format!("The backup is larger than the {limit}-byte limit."),
        format!("Cadangan melebihi batas {limit} byte."),
    ))
}

fn encode_backup(backup: &Backup, lang: Language) -> Result<Vec<u8>, String> {
    let json = serde_json::to_vec_pretty(backup).map_err(|e| format!("could not encode backup: {e}"))?;
    within_backup_limit(json.len(), lang)?;
    Ok(json)
}

/// Everything is checked here, before the library is touched.
fn parse_backup(bytes: &[u8], lang: Language) -> Result<Backup, String> {
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
fn import_backup_in(dir: &Path, path: &Path, lang: Language) -> Result<Project, String> {
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
fn import_backup(app: tauri::AppHandle, path: String, interface_language: Language) -> Result<Project, String> {
    import_backup_in(&projects_dir(&app)?, Path::new(&path), interface_language)
}

/// Snapshot of the editor's in-memory CV. The save dialog confirms replacing a
/// file; `save_atomically` guarantees a failed write leaves the old one intact.
#[tauri::command]
fn export_backup(
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

// ---------------------------------------------------------------------------
// Commands: preview bytes are cached in Rust and reused verbatim for export
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct CachedRender {
    revision: u64,
    /// The exact input these bytes were rendered from: export re-checks it so the
    /// frontend cannot hand over newer content, another structure, or another
    /// content language beside older artifacts.
    structure: Structure,
    language: Language,
    cv: Cv,
    bytes: Vec<u8>,
}

impl CachedRender {
    fn matches(&self, revision: u64, structure: Structure, language: Language, cv: &Cv) -> bool {
        self.revision == revision && self.structure == structure && self.language == language && &self.cv == cv
    }
}

#[derive(Default)]
struct RenderCache(Mutex<Option<CachedRender>>);

#[derive(Debug, Serialize)]
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
    content_language: Language,
    interface_language: Language,
    revision: u64,
    cache: tauri::State<RenderCache>,
) -> Result<tauri::ipc::Response, String> {
    let pdf = render_for(&cv, structure, content_language, interface_language)?;
    let mut cached = cache.0.lock().unwrap();
    // Concurrent invokes can finish out of order; keep the newest revision only.
    if cached.as_ref().is_some_and(|c| c.revision > revision) {
        return Err("stale render discarded".into());
    }
    *cached = Some(CachedRender { revision, structure, language: content_language, cv, bytes: pdf.clone() });
    Ok(tauri::ipc::Response::new(pdf))
}

#[tauri::command]
fn export_pdf(
    revision: u64,
    path: String,
    cv: Cv,
    structure: Structure,
    content_language: Language,
    interface_language: Language,
    cache: tauri::State<RenderCache>,
) -> Result<ExportAck, String> {
    // Rust owns export validation: drafts may be incomplete, exports may not.
    // The reader is the person using the interface, so its language is used here.
    if let Some(problem) = cv.export_problem(interface_language) {
        return Err(problem);
    }
    let cached = cache
        .0
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| {
            interface_language.pick(
                "nothing has been rendered yet",
                "belum ada pratinjau yang dirender",
            )
        })?;
    // Rust, not the frontend, proves these bytes belong to the validated CV, structure, and language.
    if !cached.matches(revision, structure, content_language, &cv) {
        return Err(interface_language.pick(
            "preview is out of date; render it again before exporting",
            "pratinjau sudah tidak sesuai; render ulang sebelum mengekspor",
        ));
    }
    // The dialog is advisory: only a .pdf destination is ever written.
    if !path.to_ascii_lowercase().ends_with(".pdf") {
        return Err(interface_language.pick(
            "destination must be a .pdf file",
            "tujuan harus berupa berkas .pdf",
        ));
    }
    save_atomically(Path::new(&path), &cached.bytes, interface_language)?;
    Ok(ExportAck {
        path,
        bytes: cached.bytes.len(),
        sha256: hex(&cached.bytes),
        warnings: cv.export_warnings(interface_language, structure),
    })
}

/// Write next to the destination, then rename so a failed write leaves the
/// existing file untouched.
fn save_atomically(path: &Path, bytes: &[u8], lang: Language) -> Result<(), String> {
    let invalid = || lang.pick("invalid destination path", "tujuan tidak valid");
    let dir = path.parent().filter(|p| !p.as_os_str().is_empty()).ok_or_else(invalid)?;
    let name = path.file_name().ok_or_else(invalid)?.to_string_lossy();
    let tmp = dir.join(format!(".{}.tmp-{}", name, std::process::id()));
    if let Err(e) = fs::write(&tmp, bytes) {
        let _ = fs::remove_file(&tmp); // never leave a partial snapshot behind
        return Err(lang.pick(format!("writing the file failed: {e}"), format!("penulisan berkas gagal: {e}")));
    }
    if let Err(e) = fs::rename(&tmp, path) {
        let _ = fs::remove_file(&tmp);
        return Err(lang.pick(
            format!("could not replace the destination: {e}"),
            format!("tidak dapat mengganti tujuan: {e}"),
        ));
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
        .setup(|app| {
            let dir = app
                .path()
                .app_data_dir()
                .map_err(|e| format!("no application data directory: {e}"))?;
            fs::create_dir_all(&dir)
                .map_err(|e| format!("could not open the CV library: {e}"))?;
            match instance_lock(&dir) {
                Ok(lock) => {
                    app.manage(lock);
                }
                Err(message) => {
                    eprintln!("{message}");
                    std::process::exit(1);
                }
            }
            Ok(())
        })
        .manage(RenderCache::default())
        .invoke_handler(tauri::generate_handler![
            render_pdf,
            export_pdf,
            list_projects,
            create_project,
            duplicate_project,
            open_project,
            save_project,
            export_backup,
            import_backup
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
        fs::write(dir.join("short.pdf"), render(&fixture(3, 60), Structure::ReverseChronological, Language::English).unwrap()).unwrap();
        fs::write(dir.join("multipage.pdf"), render(&fixture(12, 140), Structure::ReverseChronological, Language::English).unwrap()).unwrap();
        let mut narrow = fixture(1, 0);
        narrow.summary = format!("Long token: {} and {}", "y".repeat(200), "https://example.invalid/".to_string() + &"segment/".repeat(30));
        fs::write(dir.join("longtoken.pdf"), render(&narrow, Structure::ReverseChronological, Language::English).unwrap()).unwrap();
    }

    #[test]
    fn renders_a4_multipage_pdf() {
        let document = render_document(&fixture(12, 140), Structure::ReverseChronological, Language::English).unwrap();
        assert!(
            document.pages().len() > 1,
            "expected multipage output, got {}",
            document.pages().len()
        );
        let size = document.pages()[0].frame.size();
        assert!((size.x.to_pt() - 595.276).abs() < 0.1, "width {}", size.x.to_pt());
        assert!((size.y.to_pt() - 841.89).abs() < 0.1, "height {}", size.y.to_pt());
        assert!(render(&fixture(12, 140), Structure::ReverseChronological, Language::English).unwrap().starts_with(b"%PDF"));
    }

    #[test]
    fn different_content_renders_different_documents() {
        let a = render(&fixture(3, 60), Structure::ReverseChronological, Language::English).unwrap();
        let b = render(&fixture(12, 140), Structure::ReverseChronological, Language::English).unwrap();
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
        let text = document_text(&render_document(&fixture(1, 10), Structure::ReverseChronological, Language::English).unwrap());
        assert!(text.contains("Ada Lovelace"), "name missing from rendered text");
        assert!(text.contains("PROFESSIONAL SUMMARY"), "heading missing");
        assert!(text.contains("Achievement 0"), "bullet missing");
    }

    #[test]
    fn template_looking_text_stays_literal() {
        let mut cv = fixture(1, 4);
        cv.name = r#"#eval("read(\"/etc/passwd\")") *bold*"#.into();
        cv.summary = r#"#include "secret.json""#.into();
        let text = document_text(&render_document(&cv, Structure::ReverseChronological, Language::English).unwrap());
        assert!(text.contains("#eval("), "authored text missing from output: {text}");
        assert!(text.contains("#include"), "authored include text missing");
        assert!(!text.contains("root:"), "user text was evaluated as code");
    }


    #[test]
    fn world_serves_only_its_declared_files() {
        let world = CvWorld::new(&fixture(1, 1), Structure::ReverseChronological, Language::English).unwrap();
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

        let pdf = render(&fixture(1, 10), Structure::ReverseChronological, Language::English).unwrap();
        save_atomically(&target, &pdf, Language::English).unwrap();
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
            content_language: Language::default(),
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
        assert_eq!(
            read_project_file(&previous_path(&dir, &project.id).unwrap()).unwrap().cv.summary,
            "Analytical engine programmer.",
            "the replaced revision is kept for recovery"
        );
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 2, "one current and one previous file");
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
        assert!(matches!(
            read_project(&dir, "future").unwrap_err(),
            ReadError::Refused(Refusal::Version(99))
        ));
        assert_eq!(fs::read_to_string(dir.join("future.json")).unwrap(), future, "refused file rewritten");

        fs::write(dir.join("other.json"), r#"{"version":1,"id":"someone-else","cv":{}}"#).unwrap();
        assert!(matches!(
            read_project(&dir, "other").unwrap_err(),
            ReadError::Refused(Refusal::ForeignId)
        ));
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
    fn cached_bytes_only_match_the_content_structure_and_language_they_were_rendered_from() {
        let cached = CachedRender {
            revision: 4,
            structure: Structure::Functional,
            language: Language::English,
            cv: fixture(1, 6),
            bytes: b"%PDF".to_vec(),
        };
        assert!(
            cached.matches(4, Structure::Functional, Language::English, &fixture(1, 6)),
            "same revision, structure, language and content must match"
        );

        let mut edited = fixture(1, 6);
        edited.summary = "Edited after the preview was rendered.".into();
        assert!(
            !cached.matches(4, Structure::Functional, Language::English, &edited),
            "newer content must not be exported with older bytes"
        );
        assert!(
            !cached.matches(5, Structure::Functional, Language::English, &fixture(1, 6)),
            "a different revision must not match"
        );
        assert!(
            !cached.matches(4, Structure::Combination, Language::English, &fixture(1, 6)),
            "a structure change must not reuse the old preview bytes"
        );
        assert!(
            !cached.matches(4, Structure::Functional, Language::Indonesian, &fixture(1, 6)),
            "a content language change must not reuse the old preview bytes"
        );
    }

    #[test]
    fn export_requires_a_name_and_a_non_contact_section() {
        let mut cv = Cv::default();
        assert!(cv.export_problem(Language::English).unwrap().contains("full name"));

        cv.name = "Ada Lovelace".into();
        assert!(cv.export_problem(Language::English).unwrap().contains("summary or"));

        cv.summary = "   ".into();
        assert!(cv.export_problem(Language::English).is_some(), "whitespace is not content");

        cv.experience = vec![Experience::default()];
        assert!(cv.export_problem(Language::English).is_some(), "blank entries are not content");

        cv.experience[0].bullets = vec!["  ".into()];
        assert!(cv.export_problem(Language::English).is_some(), "blank bullets are not content");

        cv.summary = "Analytical engine programmer.".into();
        assert!(cv.export_problem(Language::English).is_none(), "a summary alone is enough");
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

        let text = document_text(&render_document(&cv, Structure::ReverseChronological, Language::English).unwrap());
        assert!(text.contains("Ada Lovelace"), "the name still renders: {text:?}");
        assert!(!text.contains("WORK EXPERIENCE"), "empty section rendered: {text:?}");
        assert!(!text.contains("PROFESSIONAL SUMMARY"), "blank summary rendered: {text:?}");
        assert!(!text.contains("Present"), "blank entry invented an ongoing role: {text:?}");

        cv.summary = "Analytical engine programmer.".into();
        cv.experience[1].title = "Engineer".into();
        let text = document_text(&render_document(&cv, Structure::ReverseChronological, Language::English).unwrap());
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
        let text = document_text(&render_document(&cv, Structure::ReverseChronological, Language::English).unwrap());
        assert!(text.contains("2020"), "{text:?}");
        assert!(!text.contains("Present"), "empty end became Present: {text:?}");
        assert!(!text.contains("2020 – "), "trailing separator rendered: {text:?}");

        // Only an explicit ongoing role (no end value at all) says Present.
        cv.experience[0].end = None;
        let text = document_text(&render_document(&cv, Structure::ReverseChronological, Language::English).unwrap());
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

        let text = document_text(&render_document(&cv, Structure::ReverseChronological, Language::English).unwrap());
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
            cv.export_problem(Language::English).unwrap().contains("start date is after the end date"),
            "a later start year must be rejected"
        );

        cv.experience[0].start = date(2021, Some(6));
        cv.experience[0].end = Some(date(2021, Some(3)));
        assert!(cv.export_problem(Language::English).unwrap().contains("start date is after the end date"));

        // A year-only end cannot prove a month contradiction.
        cv.experience[0].end = Some(date(2021, None));
        assert!(cv.export_problem(Language::English).is_none(), "year-only end was treated as a contradiction");

        // The same year with no months is a tie, not a contradiction.
        cv.experience[0].start = date(2021, None);
        cv.experience[0].end = Some(date(2021, None));
        assert!(cv.export_problem(Language::English).is_none());

        // Ongoing roles never contradict themselves.
        cv.experience[0].start = date(2024, Some(2));
        cv.experience[0].end = None;
        assert!(cv.export_problem(Language::English).is_none());
    }

    #[test]
    fn malformed_supplied_email_or_links_block_export() {
        let mut cv = fixture(1, 6);

        cv.email = "ada@".into();
        assert!(cv.export_problem(Language::English).unwrap().contains("email address is not valid"));
        cv.email = "ada lovelace@example.invalid".into();
        assert!(cv.export_problem(Language::English).unwrap().contains("email address is not valid"));
        cv.email = "ada@example".into();
        assert!(cv.export_problem(Language::English).unwrap().contains("email address is not valid"));
        cv.email = "ada@example.invalid".into();
        assert!(cv.export_problem(Language::English).is_none(), "a structurally valid email was rejected");

        // Clearing an optional email is allowed: the draft stays incomplete.
        cv.email = "   ".into();
        assert!(cv.export_problem(Language::English).is_none(), "empty email must not block export");
        cv.email = "ada@example.invalid".into();

        cv.links = vec![Link { label: "Portfolio".into(), url: "example.invalid".into() }];
        assert!(cv.export_problem(Language::English).unwrap().contains("must start with http:// or https://"));
        cv.links[0].url = "https://example.invalid/a b".into();
        assert!(cv.export_problem(Language::English).unwrap().contains("must start with http:// or https://"));
        cv.links[0].url = "https://example.invalid/portfolio".into();
        assert!(cv.export_problem(Language::English).is_none(), "a valid link was rejected");
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
        cv.languages = vec![LanguageEntry { name: "Indonesian".into(), level: "Native".into() }];

        let text = document_text(&render_document(&cv, Structure::ReverseChronological, Language::English).unwrap());
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
        cv.languages = vec![LanguageEntry::default(), LanguageEntry { level: "  ".into(), ..Default::default() }];

        let text = document_text(&render_document(&cv, Structure::ReverseChronological, Language::English).unwrap());
        for heading in ["EDUCATION", "SKILLS", "PROJECTS", "CERTIFICATIONS", "LANGUAGES"] {
            assert!(!text.contains(heading), "blank {heading} section rendered: {text:?}");
        }

        cv.skills[0].name = "Test automation".into();
        let text = document_text(&render_document(&cv, Structure::ReverseChronological, Language::English).unwrap());
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
        project.cv.languages = vec![LanguageEntry { name: "Indonesian".into(), level: "Native".into() }];
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
            cv.export_problem(Language::English).unwrap().contains("between 1 and 12"),
            "an out-of-range month must block export"
        );
        assert!(
            render_document(&cv, Structure::ReverseChronological, Language::English).is_err(),
            "an out-of-range month must not reach the template"
        );

        // A month without a year cannot be rendered meaningfully either.
        cv.experience[0].start = CvDate { year: None, month: Some(5) };
        assert!(cv.export_problem(Language::English).unwrap().contains("between 1 and 12"));
        assert!(render_document(&cv, Structure::ReverseChronological, Language::English).is_err());

        cv.experience[0].start = date(2020, Some(12));
        assert!(cv.export_problem(Language::English).is_none(), "a valid date must still export");
        assert!(render_document(&cv, Structure::ReverseChronological, Language::English).is_ok());
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
        cv.languages = vec![LanguageEntry { name: "Indonesian".into(), level: "Native".into() }];

        let document = render_document(&cv, Structure::ReverseChronological, Language::English).unwrap();
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
        fs::write(dir.join("experienced-multipage.pdf"), render(&cv, Structure::ReverseChronological, Language::English).unwrap()).unwrap();
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
        assert!(cv.export_problem(Language::English).is_none(), "education and projects are substantive content");

        let document = render_document(&cv, Structure::ReverseChronological, Language::English).unwrap();
        assert_eq!(document.pages().len(), 1);
        let text = document_text(&document);
        assert!(!text.contains("WORK EXPERIENCE"), "fresh graduate invented work history: {text:?}");
        for expected in ["EDUCATION", "BSc Testing", "PROJECTS", "Synthetic project"] {
            assert!(text.contains(expected), "missing {expected:?} in fresh-graduate text");
        }

        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.scratch/evidence/ticket03");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("fresh-graduate.pdf"), render(&cv, Structure::ReverseChronological, Language::English).unwrap()).unwrap();
    }

    #[test]
    fn export_warnings_name_missing_optional_content() {
        let mut cv = fixture(1, 6);
        assert!(cv.export_warnings(Language::English, Structure::ReverseChronological).is_empty(), "a complete CV warns about nothing");

        cv.email = String::new();
        cv.phone = String::new();
        assert!(
            cv.export_warnings(Language::English, Structure::ReverseChronological)
                .iter()
                .any(|w| w.contains("no way to contact you")),
            "a CV without contact details must warn"
        );

        cv.phone = "555-0100".into();
        assert!(cv.export_warnings(Language::English, Structure::ReverseChronological).is_empty(), "one contact method is enough");

        cv.experience[0].start = CvDate::default();
        assert!(
            cv.export_warnings(Language::English, Structure::ReverseChronological)
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
        let text = document_text(&render_document(&structure_fixture(), Structure::Functional, Language::English).unwrap());

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
        let text = document_text(&render_document(&structure_fixture(), Structure::Combination, Language::English).unwrap());

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
            document_text(&render_document(&structure_fixture(), Structure::ReverseChronological, Language::English).unwrap());

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
        let functional = document_text(&render_document(&cv, Structure::Functional, Language::English).unwrap());
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
        let reverse = document_text(&render_document(&cv, Structure::ReverseChronological, Language::English).unwrap());
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
        let functional = document_text(&render_document(&cv, Structure::Functional, Language::English).unwrap());
        assert!(functional.contains("WORK EXPERIENCE"));
        assert_eq!(functional.matches("Visible Engineer").count(), 1);
        assert!(!functional.contains("Hidden location"), "functional leaked a hidden location: {functional:?}");
    }

    #[test]
    fn one_cv_renders_a_different_document_per_structure() {
        let cv = structure_fixture();
        let reverse = render(&cv, Structure::ReverseChronological, Language::English).unwrap();
        let functional = render(&cv, Structure::Functional, Language::English).unwrap();
        let combination = render(&cv, Structure::Combination, Language::English).unwrap();

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
                let document = render_document(&cv, structure, Language::English).unwrap();
                if name == "multipage" {
                    assert!(
                        document.pages().len() > 1,
                        "{slug} should paginate the multipage fixture, got {}",
                        document.pages().len()
                    );
                }
                fs::write(dir.join(format!("{name}-{slug}.pdf")), render(&cv, structure, Language::English).unwrap())
                    .unwrap();
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
            !cv.export_warnings(Language::English, Structure::ReverseChronological)
                .iter()
                .any(|w| w.contains("skill group")),
            "reverse chronological does not lead with skills, so it must not warn"
        );

        for structure in [Structure::Functional, Structure::Combination] {
            let warnings = cv.export_warnings(Language::English, structure);
            assert!(
                warnings.iter().any(|w| w.contains("skill-group achievements")),
                "{structure:?} must warn about missing evidence: {warnings:?}"
            );
            assert!(
                cv.export_problem(Language::English).is_none(),
                "missing skill evidence must stay a warning, not block export"
            );
        }

        cv.skills[0].bullets = vec!["Evidence now exists.".into()];
        for structure in [Structure::Functional, Structure::Combination] {
            assert!(
                !cv.export_warnings(Language::English, structure).iter().any(|w| w.contains("skill-group achievements")),
                "{structure:?} must stop warning once evidence exists"
            );
        }
    }

    // --- ticket 05: independent duplicates, content and interface languages ---

    /// One complete CV whose authored text stays English so both content
    /// languages render the same source data and only the generated text differs.
    fn language_fixture() -> Cv {
        Cv {
            name: "Synthetic Candidate".into(),
            email: "candidate@example.invalid".into(),
            summary: "Authored English summary stays as written.".into(),
            experience: vec![Experience {
                title: "Synthetic Engineer".into(),
                company: "Example Employer".into(),
                location: "Testville".into(),
                start: date(2020, Some(5)),
                end: None, // ongoing role: Present / Sekarang
                bullets: vec!["Authored achievement bullet.".into()],
            }],
            education: vec![Education {
                school: "Test University".into(),
                degree: "BSc Testing".into(),
                location: "Testville".into(),
                start: date(2016, Some(8)),
                end: Some(date(2019, Some(5))),
                bullets: vec!["Authored education bullet.".into()],
            }],
            skills: vec![SkillGroup {
                name: "Test automation".into(),
                bullets: vec!["Authored skill bullet.".into()],
            }],
            projects: vec![ProjectEntry {
                name: "Synthetic project".into(),
                detail: "Local tooling".into(),
                bullets: vec![],
            }],
            certifications: vec![Certification {
                name: "Certified tester".into(),
                issuer: "Test board".into(),
                date: date(2023, None),
            }],
            languages: vec![LanguageEntry { name: "Indonesian".into(), level: "Native".into() }],
            ..Cv::default()
        }
    }

    #[test]
    fn content_language_selects_headings_and_present_wording_only() {
        let cv = language_fixture();
        let english =
            document_text(&render_document(&cv, Structure::ReverseChronological, Language::English).unwrap());
        let indonesian =
            document_text(&render_document(&cv, Structure::ReverseChronological, Language::Indonesian).unwrap());

        for expected in [
            "PROFESSIONAL SUMMARY",
            "WORK EXPERIENCE",
            "EDUCATION",
            "SKILLS",
            "PROJECTS",
            "CERTIFICATIONS",
            "LANGUAGES",
            "May 2020 – Present",
        ] {
            assert!(english.contains(expected), "English render missing {expected:?}: {english:?}");
        }
        for expected in [
            "RINGKASAN PROFESIONAL",
            "PENGALAMAN KERJA",
            "PENDIDIKAN",
            "KEAHLIAN",
            "PROYEK",
            "SERTIFIKASI",
            "BAHASA",
            "Mei 2020 – Sekarang",
        ] {
            assert!(indonesian.contains(expected), "Indonesian render missing {expected:?}: {indonesian:?}");
        }
        for leaked in ["WORK EXPERIENCE", "PROFESSIONAL SUMMARY", "Present"] {
            assert!(
                !indonesian.contains(leaked),
                "Indonesian render kept English generated text {leaked:?}: {indonesian:?}"
            );
        }

        // Authored content is copied into both documents, never translated.
        for authored in [
            "Synthetic Candidate",
            "Authored English summary stays as written.",
            "Synthetic Engineer",
            "Example Employer",
            "Testville",
            "Authored achievement bullet.",
            "BSc Testing",
            "Test automation",
            "Authored skill bullet.",
            "Synthetic project",
            "Certified tester",
            "Indonesian — Native",
        ] {
            assert!(english.contains(authored), "English render lost authored text {authored:?}");
            assert!(
                indonesian.contains(authored),
                "Indonesian render translated or lost authored text {authored:?}"
            );
        }
    }

    #[test]
    fn every_structure_renders_in_both_content_languages() {
        let cv = language_fixture();
        for language in [Language::English, Language::Indonesian] {
            let (work, skills, summary) = match language {
                Language::English => ("WORK EXPERIENCE", "SKILLS", "PROFESSIONAL SUMMARY"),
                Language::Indonesian => ("PENGALAMAN KERJA", "KEAHLIAN", "RINGKASAN PROFESIONAL"),
            };
            for structure in [Structure::ReverseChronological, Structure::Functional, Structure::Combination] {
                let text = document_text(&render_document(&cv, structure, language).unwrap());
                assert!(text.contains(summary), "{structure:?}/{language:?} lost the summary heading: {text:?}");
                assert!(
                    text.contains("Authored skill bullet."),
                    "{structure:?}/{language:?} lost authored skill evidence: {text:?}"
                );

                let work_at = text.find(work).unwrap_or_else(|| panic!("{work} missing in {structure:?}/{language:?}"));
                let skills_at = text.find(skills).unwrap_or_else(|| panic!("{skills} missing in {structure:?}/{language:?}"));
                if matches!(structure, Structure::Functional | Structure::Combination) {
                    assert!(skills_at < work_at, "{structure:?}/{language:?} must lead with skills: {text:?}");
                } else {
                    assert!(work_at < skills_at, "{structure:?}/{language:?} must lead with employment: {text:?}");
                }

                if matches!(structure, Structure::Functional) {
                    assert!(
                        !text.contains("Authored achievement bullet."),
                        "functional/{language:?} must stay compact: {text:?}"
                    );
                } else {
                    assert!(
                        text.contains("Authored achievement bullet."),
                        "{structure:?}/{language:?} lost the employment bullet: {text:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn duplicate_is_an_independent_copy_of_content_structure_and_language() {
        let dir = temp_dir("duplicate");
        let mut source = project();
        source.title = "Synthetic CV".into();
        source.cv = language_fixture();
        source.structure = Structure::Combination;
        source.content_language = Language::Indonesian;
        write_project(&dir, &source).unwrap();

        let copy = duplicate_in(&dir, &source.id).unwrap();
        assert_ne!(copy.id, source.id, "a duplicate needs its own identity");
        assert!(
            copy.id.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()),
            "ids stay lowercase hex as documented: {}",
            copy.id
        );
        assert_eq!(copy.title, source.title);
        assert_eq!(copy.cv, source.cv, "duplicate lost authored content");
        assert_eq!(copy.structure, Structure::Combination, "duplicate lost the selected structure");
        assert_eq!(
            copy.content_language,
            Language::Indonesian,
            "duplicate lost the selected content language"
        );
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 2, "duplicate must add exactly one file");

        // Editing the copy leaves the source file untouched...
        let mut edited_copy = read_project(&dir, &copy.id).unwrap();
        edited_copy.cv.summary = "Synthetic copy edited alone.".into();
        edited_copy.content_language = Language::English;
        edited_copy.structure = Structure::Functional;
        write_project(&dir, &edited_copy).unwrap();
        let reopened_source = read_project(&dir, &source.id).unwrap();
        assert_eq!(reopened_source.cv.summary, "Authored English summary stays as written.");
        assert_eq!(reopened_source.content_language, Language::Indonesian);
        assert_eq!(reopened_source.structure, Structure::Combination);

        // ...and editing the source leaves the copy untouched.
        let mut edited_source = read_project(&dir, &source.id).unwrap();
        edited_source.cv.summary = "Synthetic source edited alone.".into();
        write_project(&dir, &edited_source).unwrap();
        let reopened_copy = read_project(&dir, &copy.id).unwrap();
        assert_eq!(reopened_copy.cv.summary, "Synthetic copy edited alone.");
        assert_eq!(reopened_copy.content_language, Language::English);
        assert_eq!(reopened_copy.structure, Structure::Functional);

        assert!(duplicate_in(&dir, "missing-project").is_err(), "duplicating an absent project must fail");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn switching_content_language_preserves_authored_text_on_disk() {
        let dir = temp_dir("content-language");
        let mut project = project();
        project.cv = language_fixture();
        write_project(&dir, &project).unwrap();
        let original = read_project(&dir, &project.id).unwrap().cv;

        for language in [Language::Indonesian, Language::English] {
            let mut switched = read_project(&dir, &project.id).unwrap();
            switched.content_language = language;
            let stored = write_project(&dir, &switched).unwrap();

            assert_eq!(stored.content_language, language);
            assert_eq!(stored.cv, original, "changing content language rewrote authored text");
        }

        let json = fs::read_to_string(dir.join(format!("{}.json", project.id))).unwrap();
        assert!(json.contains("Authored English summary stays as written."), "authored text missing: {json}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn interface_language_localizes_export_messages_without_changing_content() {
        let mut cv = language_fixture();
        assert!(cv.export_problem(Language::English).is_none());
        assert!(cv.export_problem(Language::Indonesian).is_none());

        let untitled = Cv::default();
        let english = untitled.export_problem(Language::English).unwrap();
        let indonesian = untitled.export_problem(Language::Indonesian).unwrap();
        assert!(english.contains("full name"), "English message missing: {english}");
        assert!(indonesian.contains("nama lengkap"), "Indonesian message missing: {indonesian}");
        assert_ne!(english, indonesian, "both interface languages returned the same message");

        // Contradictory dates: the section name follows the interface language.
        cv.experience[0].start = date(2022, Some(1));
        cv.experience[0].end = Some(date(2021, Some(12)));
        assert!(cv.export_problem(Language::English).unwrap().contains("Work experience"));
        assert!(
            cv.export_problem(Language::Indonesian).unwrap().contains("Pengalaman kerja"),
            "Indonesian date message must name the Indonesian section"
        );

        // Warnings are localized too, and never block export.
        let mut incomplete = language_fixture();
        incomplete.email = String::new();
        incomplete.phone = String::new();
        incomplete.skills = vec![SkillGroup { name: "No evidence".into(), bullets: vec![] }];
        let english = incomplete.export_warnings(Language::English, Structure::Functional);
        let indonesian = incomplete.export_warnings(Language::Indonesian, Structure::Functional);
        assert!(english.iter().any(|w| w.contains("no way to contact you")), "{english:?}");
        assert!(english.iter().any(|w| w.contains("skill-group achievements")), "{english:?}");
        assert!(
            indonesian.iter().any(|w| w.contains("tidak memiliki cara untuk menghubungi")),
            "{indonesian:?}"
        );
        assert!(indonesian.iter().any(|w| w.contains("kelompok keahlian")), "{indonesian:?}");
        assert!(
            indonesian.iter().all(|w| !w.contains("no way to contact you")),
            "Indonesian warnings leaked English text: {indonesian:?}"
        );
        assert!(incomplete.export_problem(Language::Indonesian).is_none());
    }

    #[test]
    fn preview_validation_messages_follow_the_interface_language() {
        let mut cv = language_fixture();
        cv.experience[0].start = CvDate { year: Some(2020), month: Some(13) };

        let english =
            render_for(&cv, Structure::ReverseChronological, Language::English, Language::English).unwrap_err();
        let indonesian =
            render_for(&cv, Structure::ReverseChronological, Language::English, Language::Indonesian).unwrap_err();

        assert!(english.contains("between 1 and 12"), "{english}");
        assert!(indonesian.contains("bulan harus antara 1 dan 12"), "{indonesian}");
        assert!(
            !indonesian.contains("between 1 and 12"),
            "content language leaked into an interface message: {indonesian}"
        );
    }

    #[test]
    fn contradictory_dates_name_the_entry_in_the_interface_language() {
        let mut cv = language_fixture();
        cv.experience[0].title = String::new();
        cv.experience[0].company = String::new();
        cv.experience[0].start = date(2022, Some(1));
        cv.experience[0].end = Some(date(2021, Some(12)));

        let indonesian = cv.export_problem(Language::Indonesian).unwrap();
        assert!(indonesian.contains("Pengalaman kerja entri"), "{indonesian}");
        let english = cv.export_problem(Language::English).unwrap();
        assert!(english.contains("Work experience entry"), "{english}");
    }

    /// Writes one PDF per structure/content-language pair for external
    /// pdfinfo/pdftotext inspection.
    #[test]
    fn writes_language_evidence_pdfs() {
        let cv = language_fixture();
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.scratch/evidence/ticket05");
        fs::create_dir_all(&dir).unwrap();

        let structures = [
            ("reverse-chronological", Structure::ReverseChronological),
            ("functional", Structure::Functional),
            ("combination", Structure::Combination),
        ];
        for (slug, structure) in structures {
            for (language_slug, language) in [("english", Language::English), ("indonesian", Language::Indonesian)] {
                let bytes = render(&cv, structure, language).unwrap();
                assert!(bytes.starts_with(b"%PDF"));
                fs::write(dir.join(format!("{language_slug}-{slug}.pdf")), bytes).unwrap();
            }
        }

        // The same authored data rendered in another content language is a
        // different document, which is why it must never reuse cached bytes.
        assert_ne!(
            render(&cv, Structure::ReverseChronological, Language::English).unwrap(),
            render(&cv, Structure::ReverseChronological, Language::Indonesian).unwrap(),
            "content language did not change the document"
        );
    }

    // --- ticket 06: portable project backups ---

    fn source_project() -> Project {
        let mut source = project();
        source.title = "Synthetic CV".into();
        source.cv = language_fixture();
        source.structure = Structure::Combination;
        source.content_language = Language::Indonesian;
        source
    }

    fn backup_of(project: &Project) -> Backup {
        Backup {
            format: BACKUP_FORMAT.into(),
            version: BACKUP_VERSION,
            title: project.title.clone(),
            structure: project.structure,
            content_language: project.content_language,
            cv: project.cv.clone(),
        }
    }

    fn library_snapshot(dir: &Path) -> Vec<(String, Vec<u8>)> {
        let mut files: Vec<_> = fs::read_dir(dir)
            .unwrap()
            .map(|entry| {
                let path = entry.unwrap().path();
                (path.file_name().unwrap().to_string_lossy().into_owned(), fs::read(&path).unwrap())
            })
            .collect();
        files.sort();
        files
    }

    fn import_bytes(dir: &Path, bytes: &[u8], lang: Language) -> Result<Project, String> {
        // Unique per call: parallel checks must not overwrite each other's input.
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let file = temp_dir(&format!(
            "backup-input-{}",
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ))
        .join("backup.json");
        fs::write(&file, bytes).unwrap();
        import_backup_in(dir, &file, lang)
    }

    #[test]
    fn export_then_import_twice_gives_independent_copies_with_all_content() {
        let library = temp_dir("backup-library");
        let files = temp_dir("backup-files");
        let source = source_project();
        write_project(&library, &source).unwrap();
        let source_file = library_snapshot(&library);

        let target = files.join("cv-backup.json");
        let ack = export_backup(
            target.to_string_lossy().into_owned(),
            source.title.clone(),
            source.structure,
            source.content_language,
            source.cv.clone(),
            Language::English,
        )
        .unwrap();
        assert_eq!(ack.bytes, fs::read(&target).unwrap().len());
        let text = fs::read_to_string(&target).unwrap();
        assert!(!text.contains(&source.id), "a backup must not carry the project identity");
        assert!(!text.contains("updated_at"), "a backup must not carry the save timestamp");

        let first = import_backup_in(&library, &target, Language::English).unwrap();
        let second = import_backup_in(&library, &target, Language::English).unwrap();
        for copy in [&first, &second] {
            assert_ne!(copy.id, source.id);
            assert_eq!(copy.title, source.title);
            assert_eq!(copy.cv, source.cv, "import lost authored content");
            assert_eq!(copy.structure, Structure::Combination);
            assert_eq!(copy.content_language, Language::Indonesian);
        }
        assert_ne!(first.id, second.id, "repeated imports must not share an identity");
        assert_eq!(fs::read_dir(&library).unwrap().count(), 3);
        assert!(source_file.iter().all(|f| library_snapshot(&library).contains(f)), "source file changed");

        // Edits to one copy reach neither the other copy nor the source.
        let mut edited = read_project(&library, &first.id).unwrap();
        edited.cv.summary = "Only the first import was edited.".into();
        write_project(&library, &edited).unwrap();
        assert_eq!(read_project(&library, &second.id).unwrap().cv, source.cv);
        assert_eq!(read_project(&library, &source.id).unwrap().cv, source.cv);

        // The imported copy renders the same document text as the source.
        let render_text = |p: &Project| document_text(&render_document(&p.cv, p.structure, p.content_language).unwrap());
        assert_eq!(render_text(&read_project(&library, &second.id).unwrap()), render_text(&source));

        for dir in [library, files] {
            let _ = fs::remove_dir_all(dir);
        }
    }

    #[test]
    fn unsafe_backups_are_rejected_and_leave_the_library_untouched() {
        let library = temp_dir("backup-reject");
        write_project(&library, &source_project()).unwrap();
        let before = library_snapshot(&library);
        let valid = serde_json::to_value(backup_of(&source_project())).unwrap();
        let with = |edit: &dyn Fn(&mut serde_json::Value)| {
            let mut value = valid.clone();
            edit(&mut value);
            serde_json::to_vec(&value).unwrap()
        };
        let whole = serde_json::to_vec(&valid).unwrap();

        let cases: Vec<(&str, Vec<u8>)> = vec![
            ("empty file", Vec::new()),
            ("not JSON", b"not json at all".to_vec()),
            ("JSON array", b"[]".to_vec()),
            ("truncated", whole[..whole.len() / 2].to_vec()),
            ("no format marker", with(&|v| { v.as_object_mut().unwrap().remove("format"); })),
            ("another format", with(&|v| v["format"] = "something-else".into())),
            ("version zero", with(&|v| v["version"] = 0.into())),
            ("version as text", with(&|v| v["version"] = "1".into())),
            ("wrong data type", with(&|v| v["cv"]["name"] = 5.into())),
            ("bad structure", with(&|v| v["structure"] = "chaos".into())),
            ("month 13", with(&|v| v["cv"]["experience"][0]["start"]["month"] = 13.into())),
            ("month without year", with(&|v| v["cv"]["certifications"][0]["date"] = serde_json::json!({"year": null, "month": 4}))),
            ("traversal id", with(&|v| v["id"] = "../../escape".into())),
            ("path value", with(&|v| v["path"] = "/etc/passwd".into())),
            ("project updated_at", with(&|v| v["updated_at"] = 7.into())),
            ("no cv", with(&|v| { v.as_object_mut().unwrap().remove("cv"); })),
        ];
        for (label, bytes) in cases {
            for lang in [Language::English, Language::Indonesian] {
                let error = import_bytes(&library, &bytes, lang).expect_err(label);
                assert!(!error.is_empty(), "{label}: empty error");
                assert_eq!(library_snapshot(&library), before, "{label}: rejected backup changed the library");
            }
        }
        let english = import_bytes(&library, b"[]", Language::English).unwrap_err();
        let indonesian = import_bytes(&library, b"[]", Language::Indonesian).unwrap_err();
        assert!(english.contains("not a valid CV Generator backup"), "{english}");
        assert!(indonesian.contains("bukan cadangan CV Generator"), "{indonesian}");

        // A missing file is an error, not a panic or a library change.
        assert!(import_backup_in(&library, &library.join("nope.json"), Language::English).is_err());
        assert!(import_backup_in(&library, &library, Language::English).is_err(), "a directory is not a backup");
        assert_eq!(library_snapshot(&library), before);
        let _ = fs::remove_dir_all(library);
    }

    #[test]
    fn a_newer_backup_format_is_refused_by_name_whatever_its_shape() {
        let library = temp_dir("backup-newer");
        let newer = br#"{"format":"cv-generator-backup","version":2,"cv":"a shape this app cannot read"}"#;
        let english = import_bytes(&library, newer, Language::English).unwrap_err();
        let indonesian = import_bytes(&library, newer, Language::Indonesian).unwrap_err();
        assert!(english.contains("version 2") && english.contains("Update the app"), "{english}");
        assert!(indonesian.contains("versi 2") && indonesian.contains("Perbarui aplikasi"), "{indonesian}");
        assert_eq!(fs::read_dir(&library).unwrap().count(), 0, "a refused file left something behind");
        let _ = fs::remove_dir_all(library);
    }

    #[test]
    fn backup_size_limit_is_exact_for_import_and_export() {
        let library = temp_dir("backup-limit");
        let mut bytes = serde_json::to_vec(&backup_of(&source_project())).unwrap();
        bytes.resize(MAX_BACKUP_BYTES, b' '); // trailing whitespace keeps it valid JSON
        assert_eq!(bytes.len(), MAX_BACKUP_BYTES);
        assert!(import_bytes(&library, &bytes, Language::English).is_ok(), "a file at the limit must import");

        bytes.push(b' ');
        let error = import_bytes(&library, &bytes, Language::English).unwrap_err();
        assert!(error.contains("limit"), "{error}");
        assert_eq!(fs::read_dir(&library).unwrap().count(), 1, "an oversized file created a project");

        // Export refuses what import would refuse, and writes nothing.
        let mut huge = source_project();
        huge.cv.summary = "x".repeat(MAX_BACKUP_BYTES);
        let target = library.join("../cvgen-never-written.json");
        let error = export_backup(
            target.to_string_lossy().into_owned(),
            String::new(),
            huge.structure,
            huge.content_language,
            huge.cv,
            Language::English,
        )
        .unwrap_err();
        assert!(error.contains("limit"), "{error}");
        assert!(!target.exists());
        let _ = fs::remove_dir_all(library);
    }

    #[test]
    fn export_refuses_a_document_import_would_refuse() {
        let dir = temp_dir("backup-export-dates");
        let mut cv = source_project().cv;
        cv.experience[0].start = CvDate { year: Some(2020), month: Some(13) };
        let target = dir.join("never-written.json");
        let error = export_backup(
            target.to_string_lossy().into_owned(),
            String::new(),
            Structure::Combination,
            Language::English,
            cv,
            Language::Indonesian,
        )
        .unwrap_err();
        assert!(error.contains("bulan harus antara 1 dan 12"), "{error}");
        assert!(!target.exists(), "a refused export wrote a partial snapshot");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn markup_looking_text_imports_verbatim_into_the_library_only() {
        let library = temp_dir("backup-plain-text");
        let mut source = source_project();
        source.title = "../../escape".into();
        source.cv.name = r#"<script>alert(1)</script> #eval("read(\"/etc/passwd\")")"#.into();
        source.cv.summary = "../../../etc/passwd".into();
        let bytes = encode_backup(&backup_of(&source), Language::English).unwrap();

        let imported = import_bytes(&library, &bytes, Language::English).unwrap();
        assert_eq!(imported.title, source.title);
        assert_eq!(imported.cv, source.cv, "text must stay plain data");
        let names: Vec<String> = library_snapshot(&library).into_iter().map(|f| f.0).collect();
        assert_eq!(names, vec![format!("{}.json", imported.id)], "import wrote outside its own project file");
        assert!(!library.join("../escape").exists());
        let _ = fs::remove_dir_all(library);
    }

    #[test]
    fn failed_or_misdirected_backup_writes_keep_the_destination() {
        let dir = temp_dir("backup-destination");
        // The destination cannot be replaced (a non-empty directory): the old
        // content and the directory listing survive, with no temp file left.
        let blocked = dir.join("existing.json");
        fs::create_dir(&blocked).unwrap();
        fs::write(blocked.join("keep"), b"original").unwrap();
        assert!(save_atomically(&blocked, b"new backup", Language::English).is_err());
        assert_eq!(fs::read(blocked.join("keep")).unwrap(), b"original");
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 1, "a temp file was left behind");

        // An existing real file is replaced whole, never appended or truncated.
        let target = dir.join("backup.json");
        fs::write(&target, b"older backup that is longer than the new one").unwrap();
        save_atomically(&target, b"new", Language::English).unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"new");

        // Only .json destinations are written, and nothing appears otherwise.
        let wrong = dir.join("backup.pdf");
        let error = export_backup(
            wrong.to_string_lossy().into_owned(),
            String::new(),
            Structure::default(),
            Language::default(),
            Cv::default(),
            Language::Indonesian,
        )
        .unwrap_err();
        assert!(error.contains(".json"), "{error}");
        assert!(!wrong.exists());
        let _ = fs::remove_dir_all(dir);
    }

    // --- ticket 07: one rolling revision, damaged-file recovery, safe close ---

    /// Current project files only: the previous revision and preserved damaged
    /// files use dotted names, so they are never listed as CVs.
    fn listed(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| {
                name.strip_suffix(".json").is_some_and(is_valid_project_id)
            })
            .collect();
        names.sort();
        names
    }

    #[test]
    fn each_save_keeps_exactly_one_previous_valid_revision() {
        let dir = temp_dir("rolling");
        let mut project = project();
        let first = project.cv.summary.clone();
        write_project(&dir, &project).unwrap();
        assert!(
            !previous_path(&dir, &project.id).unwrap().exists(),
            "a first save has nothing to keep"
        );

        project.cv.summary = "Second saved revision.".into();
        write_project(&dir, &project).unwrap();
        let previous = previous_path(&dir, &project.id).unwrap();
        assert_eq!(read_project_file(&previous).unwrap().cv.summary, first);

        project.cv.summary = "Third saved revision.".into();
        write_project(&dir, &project).unwrap();
        assert_eq!(
            read_project_file(&previous).unwrap().cv.summary,
            "Second saved revision.",
            "only the immediately previous revision is kept, not history"
        );
        assert_eq!(read_project(&dir, &project.id).unwrap().cv.summary, "Third saved revision.");
        assert_eq!(listed(&dir), vec![format!("{}.json", project.id)], "one project file");
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 2, "one current and one previous file");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_damaged_current_file_recovers_the_previous_revision_and_keeps_the_evidence() {
        let dir = temp_dir("recovery");
        let mut project = project();
        let first = project.cv.summary.clone();
        write_project(&dir, &project).unwrap();
        project.cv.summary = "Second saved revision.".into();
        write_project(&dir, &project).unwrap();

        let damaged = b"{\"version\": 1, \"id\": \"0a1b2c\", \"cv\": {".to_vec();
        fs::write(project_path(&dir, &project.id).unwrap(), &damaged).unwrap();

        let opened = open_in(&dir, &project.id, Language::English).unwrap();
        let recovery = opened.recovery.expect("recovery must be reported, never silent");
        assert_eq!(
            opened.project.cv.summary, first,
            "the previous valid revision was not recovered"
        );
        assert!(recovery.notice.contains("previous saved revision"), "{}", recovery.notice);
        assert!(recovery.notice.contains("not full history"), "{}", recovery.notice);
        let preserved = PathBuf::from(recovery.preserved_as.expect("the damaged path is reported"));
        assert!(preserved.exists(), "the damaged file was not kept");
        assert_eq!(fs::read(&preserved).unwrap(), damaged, "the damaged bytes were changed");
        assert!(
            recovery.notice.contains(&preserved.display().to_string()),
            "the notice must name the kept file: {}",
            recovery.notice
        );

        // The current file is healthy again: a restart is an ordinary open.
        assert_eq!(read_project(&dir, &project.id).unwrap().cv.summary, first);
        assert_eq!(
            fs::read(project_path(&dir, &project.id).unwrap()).unwrap(),
            fs::read(previous_path(&dir, &project.id).unwrap()).unwrap(),
            "the previous revision was not restored verbatim"
        );
        assert!(open_in(&dir, &project.id, Language::English).unwrap().recovery.is_none());
        assert_eq!(listed(&dir), vec![format!("{}.json", project.id)]);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_damaged_project_with_no_valid_previous_revision_is_reported_and_left_alone() {
        let dir = temp_dir("no-recovery");
        let lonely = project();
        write_project(&dir, &lonely).unwrap();
        let current = project_path(&dir, &lonely.id).unwrap();
        fs::write(&current, b"not json").unwrap();
        let before = library_snapshot(&dir);

        let english = open_in(&dir, &lonely.id, Language::English).unwrap_err();
        let indonesian = open_in(&dir, &lonely.id, Language::Indonesian).unwrap_err();
        assert!(english.contains("no valid previous revision"), "{english}");
        assert!(indonesian.contains("tidak ada revisi sebelumnya yang valid"), "{indonesian}");
        assert!(english.contains(&current.display().to_string()), "{english}");
        assert_eq!(library_snapshot(&dir), before, "a failed open wrote or blanked something");

        // A previous revision that is damaged too: same report, still no write.
        let mut project = project();
        write_project(&dir, &project).unwrap();
        project.cv.summary = "Second saved revision.".into();
        write_project(&dir, &project).unwrap();
        fs::write(&current, b"corrupt current").unwrap();
        fs::write(previous_path(&dir, &project.id).unwrap(), b"corrupt previous").unwrap();
        let before = library_snapshot(&dir);
        assert!(open_in(&dir, &project.id, Language::English).is_err());
        assert_eq!(library_snapshot(&dir), before, "both damaged files were rewritten");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_denied_save_keeps_the_current_and_previous_revisions() {
        let dir = temp_dir("denied");
        let mut project = project();
        write_project(&dir, &project).unwrap();
        project.cv.summary = "Second saved revision.".into();
        write_project(&dir, &project).unwrap();
        let before = library_snapshot(&dir);

        project.cv.summary = "Unsaved edit.".into();
        let error = write_project_in(&dir, &project, Faults { deny_write: true, ..Faults::default() })
            .unwrap_err();
        assert!(!error.is_empty(), "a denied write must report an error");
        assert_eq!(library_snapshot(&dir), before, "a denied save changed the library");
        assert_eq!(read_project(&dir, &project.id).unwrap().cv.summary, "Second saved revision.");

        // The same edits are still in memory and retry once writes are allowed.
        let stored = write_project(&dir, &project).unwrap();
        assert_eq!(stored.cv.summary, "Unsaved edit.");
        assert_eq!(
            read_project_file(&previous_path(&dir, &project.id).unwrap()).unwrap().cv.summary,
            "Second saved revision.",
            "the retry keeps what was saved before it"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_interrupted_replacement_leaves_the_last_saved_revision_readable() {
        let dir = temp_dir("interrupted");
        let mut project = project();
        write_project(&dir, &project).unwrap();
        let kept = fs::read(project_path(&dir, &project.id).unwrap()).unwrap();

        project.cv.summary = "Never replaced the file.".into();
        let error = write_project_in(&dir, &project, Faults { interrupt_replace: true, ..Faults::default() })
            .unwrap_err();
        assert!(!error.is_empty());
        assert_eq!(
            fs::read(project_path(&dir, &project.id).unwrap()).unwrap(),
            kept,
            "the interrupted replacement changed the current file"
        );
        assert_eq!(read_project(&dir, &project.id).unwrap().cv.summary, "Analytical engine programmer.");
        assert!(
            open_in(&dir, &project.id, Language::English).unwrap().recovery.is_none(),
            "the interrupted content was never saved, so there is nothing to recover"
        );

        write_project(&dir, &project).unwrap();
        assert_eq!(read_project(&dir, &project.id).unwrap().cv.summary, "Never replaced the file.");
        assert_eq!(
            read_project_file(&previous_path(&dir, &project.id).unwrap()).unwrap().cv.summary,
            "Analytical engine programmer."
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_corrupt_current_file_never_replaces_the_previous_valid_revision() {
        let dir = temp_dir("corrupt-rotate");
        let mut project = project();
        write_project(&dir, &project).unwrap();
        project.cv.summary = "Second saved revision.".into();
        write_project(&dir, &project).unwrap();
        let previous = previous_path(&dir, &project.id).unwrap();
        let kept = fs::read(&previous).unwrap();

        fs::write(project_path(&dir, &project.id).unwrap(), b"corrupt current").unwrap();
        project.cv.summary = "Written over a damaged current file.".into();
        write_project(&dir, &project).unwrap();

        assert_eq!(
            fs::read(&previous).unwrap(),
            kept,
            "a damaged current file replaced the good previous revision"
        );
        assert_eq!(read_project_file(&previous).unwrap().cv.summary, "Analytical engine programmer.");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_library_lists_a_damaged_project_so_it_can_be_recovered() {
        let dir = temp_dir("damaged-list");
        let mut project = project();
        project.title = "Synthetic CV".into();
        write_project(&dir, &project).unwrap();
        project.cv.summary = "Second saved revision.".into();
        write_project(&dir, &project).unwrap();

        let healthy = summaries_in(&dir).unwrap();
        assert_eq!(healthy.len(), 1);
        assert!(!healthy[0].damaged);

        fs::write(project_path(&dir, &project.id).unwrap(), b"corrupt").unwrap();
        let damaged = summaries_in(&dir).unwrap();
        assert_eq!(damaged.len(), 1, "a damaged CV must stay listed so it can be recovered");
        assert!(damaged[0].damaged);
        assert_eq!(damaged[0].id, project.id);
        assert_eq!(damaged[0].title, "Synthetic CV", "the previous revision labels the row");
        assert_eq!(damaged[0].name, "Ada Lovelace");

        open_in(&dir, &project.id, Language::English).unwrap();
        let recovered = summaries_in(&dir).unwrap();
        assert_eq!(recovered.len(), 1, "the previous revision is not a second CV");
        assert!(!recovered[0].damaged);

        // With no readable revision either, the row still exists so opening can
        // report the condition instead of the CV silently disappearing.
        fs::write(project_path(&dir, &project.id).unwrap(), b"corrupt").unwrap();
        fs::write(previous_path(&dir, &project.id).unwrap(), b"corrupt").unwrap();
        let unreadable = summaries_in(&dir).unwrap();
        assert_eq!(unreadable.len(), 1);
        assert!(unreadable[0].damaged);
        assert_eq!(unreadable[0].id, project.id);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn recovery_and_unreadable_notices_follow_the_interface_language() {
        let dir = temp_dir("recovery-messages");
        let mut project = project();
        write_project(&dir, &project).unwrap();
        project.cv.summary = "Second saved revision.".into();
        write_project(&dir, &project).unwrap();
        fs::write(project_path(&dir, &project.id).unwrap(), b"corrupt").unwrap();

        let notice = open_in(&dir, &project.id, Language::Indonesian)
            .unwrap()
            .recovery
            .unwrap()
            .notice;
        assert!(notice.contains("revisi tersimpan sebelumnya dipulihkan"), "{notice}");
        assert!(notice.contains("bukan seluruh riwayat"), "{notice}");
        assert!(!notice.contains("Your saved file"), "{notice}");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn one_running_instance_holds_the_library() {
        let dir = temp_dir("instance");
        let first = instance_lock(&dir).unwrap();
        let second = instance_lock(&dir).unwrap_err();
        assert!(second.contains("Another CV Generator window"), "{second}");
        drop(first);
        assert!(
            instance_lock(&dir).is_ok(),
            "the lock must be released when the process holding it exits"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_newer_project_file_is_refused_and_left_alone() {
        let dir = temp_dir("refused-newer");
        let mut project = project();
        project.title = "Synthetic CV".into();
        write_project(&dir, &project).unwrap();
        project.cv.summary = "Second saved revision.".into();
        write_project(&dir, &project).unwrap();

        let current = project_path(&dir, &project.id).unwrap();
        let newer = br#"{"version":2,"id":"0a1b2c","cv":{"name":"Ada from a newer app"}}"#;
        fs::write(&current, newer).unwrap();
        let before = library_snapshot(&dir);

        let english = open_in(&dir, &project.id, Language::English).unwrap_err();
        let indonesian = open_in(&dir, &project.id, Language::Indonesian).unwrap_err();
        assert!(english.contains("project version 2"), "{english}");
        assert!(english.contains("will not rewrite"), "{english}");
        assert!(indonesian.contains("versi proyek 2"), "{indonesian}");
        assert_eq!(library_snapshot(&dir), before, "a newer file was quarantined or rewritten");

        // It stays listed and is not reported as damage; opening explains the version.
        let summaries = summaries_in(&dir).unwrap();
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].id, project.id);
        assert!(!summaries[0].damaged, "a refused newer file is not damage");
        assert_eq!(summaries[0].title, "Synthetic CV", "the previous revision labels the row");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_foreign_project_file_is_refused_without_rewriting() {
        let dir = temp_dir("refused-foreign");
        let foreign = r#"{"version":1,"id":"someone-else","cv":{"name":"Not mine"}}"#;
        fs::write(dir.join("0a1b2c.json"), foreign).unwrap();
        let before = library_snapshot(&dir);

        let english = open_in(&dir, "0a1b2c", Language::English).unwrap_err();
        let indonesian = open_in(&dir, "0a1b2c", Language::Indonesian).unwrap_err();
        assert!(english.contains("belongs to another project"), "{english}");
        assert!(indonesian.contains("milik proyek lain"), "{indonesian}");
        assert_eq!(library_snapshot(&dir), before, "a foreign file was rewritten or quarantined");
        assert!(!summaries_in(&dir).unwrap()[0].damaged);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn damage_found_during_a_save_is_preserved() {
        let dir = temp_dir("save-preserves-damage");
        let mut project = project();
        write_project(&dir, &project).unwrap();
        project.cv.summary = "Second saved revision.".into();
        write_project(&dir, &project).unwrap();

        let damaged = b"corrupt current".to_vec();
        fs::write(project_path(&dir, &project.id).unwrap(), &damaged).unwrap();
        project.cv.summary = "Written over a damaged current file.".into();
        write_project(&dir, &project).unwrap();

        let kept: Vec<PathBuf> = fs::read_dir(&dir)
            .unwrap()
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| path.to_string_lossy().contains(".corrupt-"))
            .collect();
        assert_eq!(kept.len(), 1, "the damaged bytes were not kept: {kept:?}");
        assert_eq!(fs::read(&kept[0]).unwrap(), damaged, "the damaged bytes were changed");
        assert_eq!(read_project(&dir, &project.id).unwrap().cv.summary, "Written over a damaged current file.");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn preserving_damage_keeps_the_current_file_in_place() {
        let dir = temp_dir("preserve-copy");
        let project = project();
        write_project(&dir, &project).unwrap();
        let current = project_path(&dir, &project.id).unwrap();
        let damaged = b"corrupt current".to_vec();
        fs::write(&current, &damaged).unwrap();

        let kept = preserve_corrupt(&dir, &project.id).unwrap().unwrap();
        assert!(current.exists(), "the CV must stay at its path so the library still lists it");
        assert_eq!(fs::read(&current).unwrap(), damaged);
        assert_eq!(fs::read(&kept).unwrap(), damaged, "the kept copy must match the damaged bytes");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_save_reports_the_damage_it_preserved() {
        let dir = temp_dir("save-damage-notice");
        let mut project = project();
        write_project(&dir, &project).unwrap();
        project.cv.summary = "Second saved revision.".into();
        write_project(&dir, &project).unwrap();
        fs::write(project_path(&dir, &project.id).unwrap(), b"corrupt current").unwrap();
        project.cv.summary = "Saved over damage.".into();

        let (stored, preserved) = write_project_in(&dir, &project, Faults::default()).unwrap();
        let kept = preserved.expect("a save that kept a damaged file must report it");
        assert_eq!(stored.cv.summary, "Saved over damage.");
        let english = damage_notice(&kept, Language::English);
        let indonesian = damage_notice(&kept, Language::Indonesian);
        assert!(english.contains("damaged while you were editing"), "{english}");
        assert!(english.contains(&kept.display().to_string()), "{english}");
        assert!(indonesian.contains("rusak saat Anda mengedit"), "{indonesian}");
        assert!(!indonesian.contains("damaged while"), "{indonesian}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_save_never_overwrites_a_refused_project_file() {
        let dir = temp_dir("save-refused");
        let project = project();
        let current = project_path(&dir, &project.id).unwrap();
        let newer = br#"{"version":2,"id":"0a1b2c","cv":{"name":"Ada from a newer app"}}"#;
        fs::write(&current, newer).unwrap();
        let before = library_snapshot(&dir);

        let error = write_project(&dir, &project).unwrap_err();
        assert!(error.contains("refusing to overwrite project version 2"), "{error}");
        assert_eq!(library_snapshot(&dir), before, "a save rewrote a refused file");
        let _ = fs::remove_dir_all(&dir);
    }
}
