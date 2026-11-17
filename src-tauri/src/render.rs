use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use serde::Serialize;
use typst::diag::{FileError, FileResult, SourceDiagnostic};
use typst::foundations::{Bytes, Datetime, Duration};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};
use typst_layout::PagedDocument;


use crate::{cv::*, files::*};

// ---------------------------------------------------------------------------
// Embedded renderer: Typst with bundled fonts, one fixed template, no I/O
// ---------------------------------------------------------------------------

pub(crate) const TEMPLATE: &str = include_str!("../templates/cv.typ");
pub(crate) const DATA_PATH: &str = "cv.json";

pub(crate) static LIBRARY: OnceLock<LazyHash<Library>> = OnceLock::new();
pub(crate) static BOOK: OnceLock<LazyHash<FontBook>> = OnceLock::new();
pub(crate) static FONTS: OnceLock<Vec<Font>> = OnceLock::new();

pub(crate) static FONT_FILES: [&[u8]; 3] = [
    include_bytes!("../fonts/LiberationSans-Regular.ttf"),
    include_bytes!("../fonts/LiberationSans-Bold.ttf"),
    include_bytes!("../fonts/LiberationSans-Italic.ttf"),
];

pub(crate) fn fonts() -> &'static Vec<Font> {
    FONTS.get_or_init(|| {
        FONT_FILES
            .iter()
            .filter_map(|data| Font::new(Bytes::new(*data), 0))
            .collect()
    })
}

pub(crate) fn library() -> &'static LazyHash<Library> {
    LIBRARY.get_or_init(|| LazyHash::new(Library::default()))
}

pub(crate) fn book() -> &'static LazyHash<FontBook> {
    BOOK.get_or_init(|| LazyHash::new(fonts().iter().fold(FontBook::new(), |mut book, font| {
        book.push(font.info().clone());
        book
    })))
}

pub(crate) struct CvWorld {
    pub(crate) main: Source,
    pub(crate) data_id: FileId,
    pub(crate) data: Bytes,
}

/// What the fixed template receives: one CV, the selected structure, and the
/// content language that decides the standard headings and Present wording.
/// All of it is data; the template only projects sections, it never mutates content.
#[derive(Serialize)]
pub(crate) struct RenderInput<'a> {
    pub(crate) structure: Structure,
    pub(crate) language: Language,
    pub(crate) cv: &'a Cv,
}

impl CvWorld {
    pub(crate) fn new(cv: &Cv, structure: Structure, language: Language) -> Result<Self, String> {
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

pub(crate) fn not_found(id: FileId) -> FileError {
    FileError::NotFound(PathBuf::from(id.vpath().get_with_slash()))
}

pub(crate) fn file_id(path: &str) -> FileId {
    let vpath = VirtualPath::new(path).expect("static virtual path");
    RootedPath::new(VirtualRoot::Project, vpath).intern()
}

pub(crate) fn diagnostics_message(diagnostics: &[SourceDiagnostic]) -> String {
    diagnostics
        .iter()
        .map(|d| format!("{} @ {:?}", d.message, d.span))
        .collect::<Vec<_>>()
        .join("; ")
}

pub(crate) fn render_document(cv: &Cv, structure: Structure, language: Language) -> Result<PagedDocument, String> {
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
pub(crate) fn render_for(
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
// Commands: preview bytes are cached in Rust and reused verbatim for export
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub(crate) struct CachedRender {
    pub(crate) revision: u64,
    /// The exact input these bytes were rendered from: export re-checks it so the
    /// frontend cannot hand over newer content, another structure, or another
    /// content language beside older artifacts.
    pub(crate) structure: Structure,
    pub(crate) content_language: Language,
    pub(crate) cv: Cv,
    pub(crate) bytes: Vec<u8>,
}

impl CachedRender {
    /// Export proves the bytes belong to this content, structure, and content
    /// language. The client's render counter is not evidence of freshness; the
    /// input is.
    pub(crate) fn matches(&self, structure: Structure, content_language: Language, cv: &Cv) -> bool {
        self.structure == structure && self.content_language == content_language && &self.cv == cv
    }
}

#[derive(Default)]
pub(crate) struct RenderCache(pub(crate) Mutex<Option<CachedRender>>);

/// A render started later always wins. An older result is refused rather than
/// overwriting bytes for newer content, however late it arrives.
pub(crate) fn cache_render(
    cache: &RenderCache,
    revision: u64,
    structure: Structure,
    content_language: Language,
    cv: Cv,
    bytes: Vec<u8>,
) -> Result<(), String> {
    let mut cached = cache.0.lock().unwrap();
    if cached.as_ref().is_some_and(|c| c.revision > revision) {
        return Err("stale render discarded".into());
    }
    *cached = Some(CachedRender { revision, structure, content_language, cv, bytes });
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct ExportAck {
    pub(crate) path: String,
    pub(crate) bytes: usize,
    pub(crate) sha256: String,
    pub(crate) warnings: Vec<String>,
}

#[tauri::command]
pub fn render_pdf(
    cv: Cv,
    structure: Structure,
    content_language: Language,
    interface_language: Language,
    revision: u64,
    cache: tauri::State<RenderCache>,
) -> Result<tauri::ipc::Response, String> {
    let pdf = render_for(&cv, structure, content_language, interface_language)?;
    cache_render(&cache, revision, structure, content_language, cv, pdf.clone())?;
    Ok(tauri::ipc::Response::new(pdf))
}

#[tauri::command]
pub fn export_pdf(
    path: String,
    cv: Cv,
    structure: Structure,
    content_language: Language,
    interface_language: Language,
    cache: tauri::State<RenderCache>,
) -> Result<ExportAck, String> {
    export_cached(&cache, path, cv, structure, content_language, interface_language)
}

/// Exports the cached artifact only for the exact content it was rendered from.
pub(crate) fn export_cached(
    cache: &RenderCache,
    path: String,
    cv: Cv,
    structure: Structure,
    content_language: Language,
    interface_language: Language,
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
    if !cached.matches(structure, content_language, &cv) {
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
