// Checks: A4 pages, text survives as extractable text, export is byte-identical.
// Progressive setup reads better than one giant struct literal in these checks.
#![allow(clippy::field_reassign_with_default)]

// Progressive setup reads better than one giant struct literal in these checks.


use std::fs;
use std::path::{Path, PathBuf};

use typst::World;
use typst_layout::PagedDocument;

use crate::{backup::*, cv::*, files::*, library::*, render::*, trash::*};

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
fn headline_follows_the_name_and_page_numbers_appear_only_on_multipage_cvs() {
    let mut cv = fixture(1, 10);
    cv.headline = "Full Stack Developer".into();
    let text = document_text(&render_document(&cv, Structure::ReverseChronological, Language::English).unwrap());
    let (name, headline, email) = (text.find("Ada Lovelace"), text.find("Full Stack Developer"), text.find("ada@example"));
    assert!(name < headline && headline < email, "headline out of reading order: {text:?}");
    assert!(!text.contains("1 / 1"), "a one-page CV is numbered: {text:?}");

    let pages = page_texts(&render_document(&fixture(12, 140), Structure::ReverseChronological, Language::English).unwrap());
    let total = pages.len();
    assert!(total > 1);
    for (i, page) in pages.iter().enumerate() {
        assert!(page.ends_with(&format!("{} / {total}", i + 1)), "page {} footer: {page:?}", i + 1);
    }
}

#[test]
fn different_content_renders_different_documents() {
    let a = render(&fixture(3, 60), Structure::ReverseChronological, Language::English).unwrap();
    let b = render(&fixture(12, 140), Structure::ReverseChronological, Language::English).unwrap();
    assert_ne!(a, b, "renderer returned a cached document for different content");
}

fn document_text(document: &PagedDocument) -> String {
    page_texts(document).join("\n")
}

fn page_texts(document: &PagedDocument) -> Vec<String> {
    document.pages().iter().map(|page| frame_text(&page.frame)).collect()
}

fn frame_text(frame: &typst::layout::Frame) -> String {
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
    walk(frame, &mut out);
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

#[test]
fn a_failed_render_keeps_the_previous_preview_and_blocks_exporting_newer_content() {
    let dir = temp_dir("render-failure");
    let cache = RenderCache::default();
    let older = fixture(1, 6);
    let older_pdf = render(&older, Structure::ReverseChronological, Language::English).unwrap();
    cache_render(&cache, 1, Structure::ReverseChronological, Language::English, older.clone(), older_pdf.clone()).unwrap();

    // A month without a year cannot be rendered, so the render fails and the
    // previous preview — the only artifact the reader can still see — survives.
    let mut unrenderable = fixture(1, 6);
    unrenderable.experience[0].start = CvDate { year: None, month: Some(5) };
    let error = render_for(&unrenderable, Structure::ReverseChronological, Language::English, Language::English).unwrap_err();
    assert!(!error.is_empty(), "a rendering failure must report a message");
    let kept = cache.0.lock().unwrap().clone().unwrap();
    assert_eq!(kept.revision, 1, "a failed render must not replace the previous preview");
    assert_eq!(kept.bytes, older_pdf, "the visible preview bytes must survive the failure");

    // Exporting content that never rendered is refused and writes nothing, so
    // the older preview cannot be exported as the current CV.
    let target = dir.join("cv.pdf");
    let mut edited = older;
    edited.summary = "Edited while rendering kept failing.".into();
    let refusal = export_cached(
        &cache,
        target.to_string_lossy().into_owned(),
        edited,
        Structure::ReverseChronological,
        Language::English,
        Language::English,
    )
    .unwrap_err();
    assert!(refusal.contains("out of date"), "{refusal}");
    assert!(!target.exists(), "a refused export must not create the destination");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_failed_pdf_export_keeps_the_destination_and_writes_nothing_partial() {
    let dir = temp_dir("pdf-destination");
    let cv = fixture(1, 10);
    let pdf = render(&cv, Structure::ReverseChronological, Language::English).unwrap();
    let cache = RenderCache::default();
    cache_render(&cache, 1, Structure::ReverseChronological, Language::English, cv.clone(), pdf.clone()).unwrap();

    // A destination that cannot be replaced (a non-empty directory): the old
    // content survives and no temp file is left behind.
    let blocked = dir.join("existing.pdf");
    fs::create_dir(&blocked).unwrap();
    fs::write(blocked.join("keep"), b"original").unwrap();
    let refused = export_cached(
        &cache,
        blocked.to_string_lossy().into_owned(),
        cv.clone(),
        Structure::ReverseChronological,
        Language::English,
        Language::English,
    )
    .unwrap_err();
    assert!(!refused.is_empty(), "a failed write must report an error");
    assert_eq!(fs::read(blocked.join("keep")).unwrap(), b"original");
    assert_eq!(fs::read_dir(&dir).unwrap().count(), 1, "a temp file was left behind");

    // A current preview exports its exact bytes over an existing file.
    let target = dir.join("cv.pdf");
    fs::write(&target, b"older pdf that must be replaced whole").unwrap();
    let ack = export_cached(
        &cache,
        target.to_string_lossy().into_owned(),
        cv.clone(),
        Structure::ReverseChronological,
        Language::English,
        Language::English,
    )
    .unwrap();
    assert_eq!(fs::read(&target).unwrap(), pdf);
    assert_eq!(ack.sha256, hex(&pdf));

    // Content that was never rendered is refused and leaves the file alone.
    let mut edited = cv;
    edited.summary = "Edited after the preview was rendered.".into();
    let stale = export_cached(
        &cache,
        target.to_string_lossy().into_owned(),
        edited,
        Structure::ReverseChronological,
        Language::English,
        Language::English,
    )
    .unwrap_err();
    assert!(stale.contains("out of date"), "{stale}");
    assert_eq!(fs::read(&target).unwrap(), pdf);
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
    let cache = RenderCache::default();
    cache_render(
        &cache,
        4,
        Structure::Functional,
        Language::English,
        fixture(1, 6),
        b"%PDF".to_vec(),
    )
    .unwrap();
    let cached = cache.0.lock().unwrap().clone().unwrap();
    assert!(
        cached.matches(Structure::Functional, Language::English, &fixture(1, 6)),
        "same structure, language and content must match"
    );

    // A re-render of identical content at a newer revision still matches: the
    // client's render counter is not the export identity.
    cache_render(
        &cache,
        9,
        Structure::Functional,
        Language::English,
        fixture(1, 6),
        b"%PDF".to_vec(),
    )
    .unwrap();
    let newer = cache.0.lock().unwrap().clone().unwrap();
    assert!(
        newer.matches(Structure::Functional, Language::English, &fixture(1, 6)),
        "a re-render of the same content must still export"
    );

    let mut edited = fixture(1, 6);
    edited.summary = "Edited after the preview was rendered.".into();
    assert!(
        !newer.matches(Structure::Functional, Language::English, &edited),
        "newer content must not be exported with older bytes"
    );
    assert!(
        !newer.matches(Structure::Combination, Language::English, &fixture(1, 6)),
        "a structure change must not reuse the old preview bytes"
    );
    assert!(
        !newer.matches(Structure::Functional, Language::Indonesian, &fixture(1, 6)),
        "a content language change must not reuse the old preview bytes"
    );
}

#[test]
fn a_late_render_result_cannot_replace_a_newer_preview() {
    let cache = RenderCache::default();
    let newer = fixture(1, 6);
    let older = {
        let mut cv = fixture(1, 6);
        cv.summary = "Older content rendered first.".into();
        cv
    };
    cache_render(&cache, 2, Structure::Functional, Language::English, newer.clone(), b"newer bytes".to_vec()).unwrap();
    let refused = cache_render(&cache, 1, Structure::Functional, Language::English, older, b"older bytes".to_vec());
    assert!(refused.is_err(), "an older revision must not overwrite newer bytes");

    // The newer bytes and their content are intact, so export still matches them.
    let kept = cache.0.lock().unwrap().clone().unwrap();
    assert_eq!(kept.revision, 2);
    assert_eq!(kept.bytes, b"newer bytes");
    assert!(kept.matches(Structure::Functional, Language::English, &newer));
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
    assert!(text.contains("Year role") && text.contains("2019"), "year-only start missing: {text:?}");
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
        "2023",
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

/// Long content in every section, so each structure has real page breaks and
/// headings that land near them.
fn long_fixture() -> Cv {
    let mut cv = fixture(0, 0);
    cv.summary = format!("Long content marker summary. {}", "A detailed summary sentence. ".repeat(14));
    cv.experience = (0..8)
        .map(|i| Experience {
            title: format!("Long content role {i}"),
            company: "Analytical Engines Ltd".into(),
            location: "London".into(),
            start: date(2012 + i as u16, Some(1)),
            end: Some(date(2013 + i as u16, Some(12))),
            bullets: (0..5)
                .map(|b| format!("Long content marker {i}-{b} {}", "achieved something measurable ".repeat(6)))
                .collect(),
        })
        .collect();
    cv.skills = (0..4)
        .map(|s| SkillGroup {
            name: format!("Skill group {s}"),
            bullets: (0..3)
                .map(|b| format!("Long content marker skill {s}-{b} {}", "evidence of the capability ".repeat(5)))
                .collect(),
        })
        .collect();
    cv.education = vec![Education {
        school: "Test University".into(),
        degree: "BSc Testing".into(),
        location: "Testville".into(),
        start: date(2008, Some(9)),
        end: Some(date(2011, Some(6))),
        bullets: vec!["Long content marker education 0".into()],
    }];
    cv.projects = vec![ProjectEntry {
        name: "Synthetic project".into(),
        detail: "Local tooling".into(),
        bullets: vec!["Long content marker project 0".into()],
    }];
    cv.certifications = vec![Certification {
        name: "Certified tester".into(),
        issuer: "Test board".into(),
        date: date(2023, None),
    }];
    cv.languages = vec![LanguageEntry { name: "Indonesian".into(), level: "Native".into() }];
    cv
}

#[test]
fn long_content_keeps_a4_pages_readable_order_and_headings_with_their_content() {
    let cv = long_fixture();
    let headings = [
        "PROFESSIONAL SUMMARY",
        "WORK EXPERIENCE",
        "SKILLS",
        "EDUCATION",
        "PROJECTS",
        "CERTIFICATIONS",
        "LANGUAGES",
    ];
    for structure in [
        Structure::ReverseChronological,
        Structure::Functional,
        Structure::Combination,
    ] {
        let document = render_document(&cv, structure, Language::English).unwrap();
        assert!(
            document.pages().len() > 1,
            "{structure:?} must span pages for long content, got {}",
            document.pages().len()
        );

        for (n, page) in document.pages().iter().enumerate() {
            let size = page.frame.size();
            assert!(
                (size.x.to_pt() - 595.276).abs() < 0.1 && (size.y.to_pt() - 841.89).abs() < 0.1,
                "{structure:?} page {} is not A4: {size:?}",
                n + 1
            );
        }

        let text = document_text(&document);
        // Functional shows a compact employer/title/date row: its bullets stay
        // in the project by design, but no role or other section may be lost.
        let mut expected = headings.to_vec();
        expected.extend([
            "Long content marker skill 3-2",
            "Long content marker education 0",
            "Long content marker project 0",
        ]);
        if structure == Structure::Functional {
            expected.push("Long content role 7");
        } else {
            expected.push("Long content marker 7-4");
        }
        for expected in expected {
            assert!(
                text.contains(expected),
                "{structure:?} lost {expected:?}; long content must not be clipped, shrunk, or dropped"
            );
        }

        // Reading order follows the structure: skills lead in functional and
        // combination, employment leads in reverse chronological.
        let at = |needle: &str| text.find(needle).unwrap_or_else(|| panic!("{structure:?} lost {needle}"));
        if structure == Structure::ReverseChronological {
            assert!(at("WORK EXPERIENCE") < at("SKILLS"), "{structure:?} must keep skills as support");
        } else {
            assert!(at("SKILLS") < at("WORK EXPERIENCE"), "{structure:?} must lead with skills");
        }

        // A sticky heading never ends a page alone.
        for (n, page) in page_texts(&document).iter().enumerate() {
            for heading in headings {
                assert!(
                    !page.trim_end().ends_with(heading),
                    "{structure:?} page {} ends with the heading {heading:?}",
                    n + 1
                );
            }
        }
    }
}

#[test]
fn writes_ticket09_structure_evidence_pdfs() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.scratch/evidence/ticket09");
    fs::create_dir_all(&dir).unwrap();
    let cv = long_fixture();
    for (structure, name) in [
        (Structure::ReverseChronological, "long-reverse-chronological.pdf"),
        (Structure::Functional, "long-functional.pdf"),
        (Structure::Combination, "long-combination.pdf"),
    ] {
        let pdf = render(&cv, structure, Language::English).unwrap();
        assert!(pdf.starts_with(b"%PDF"), "{name} is not a PDF");
        assert!(
            render_document(&cv, structure, Language::English).unwrap().pages().len() > 1,
            "{name} does not span pages"
        );
        fs::write(dir.join(name), pdf).unwrap();
    }
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
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.is_file())
        .map(|path| {
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

    // A file belonging to another project is refused at the save path too.
    let foreign = br#"{"version":1,"id":"someone-else","cv":{"name":"Not mine"}}"#;
    fs::write(&current, foreign).unwrap();
    let before = library_snapshot(&dir);
    let error = write_project(&dir, &project).unwrap_err();
    assert!(error.contains("belongs to another project"), "{error}");
    assert_eq!(library_snapshot(&dir), before, "a save rewrote a foreign project file");
    let _ = fs::remove_dir_all(&dir);
}

// --- ticket 08: Trash, restore, explicit permanent deletion ---

/// Files currently waiting in Trash, sorted by name.
fn trash_names(dir: &Path) -> Vec<String> {
    let trash = dir.join("trash");
    if !trash.exists() {
        return Vec::new();
    }
    let mut names: Vec<String> = fs::read_dir(trash)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// An id that starts with `project().id`, which must never be claimed by it.
fn neighbouring_project() -> Project {
    let mut other = project();
    other.id = "0a1b2cff".into();
    other.title = "Other CV".into();
    other.cv.name = "Grace Hopper".into();
    other
}

#[test]
fn delete_moves_the_saved_file_previous_revision_and_damaged_copy_to_trash() {
    let dir = temp_dir("trash-move");
    let project = project();
    write_project(&dir, &project).unwrap();
    let first = project.cv.summary.clone();
    let mut second = project.clone();
    second.cv.summary = "Second saved revision.".into();
    write_project(&dir, &second).unwrap();
    let damaged = b"corrupt current".to_vec();
    let current = project_path(&dir, &project.id).unwrap();
    fs::write(&current, &damaged).unwrap();
    let kept = preserve_corrupt(&dir, &project.id).unwrap().unwrap();
    let neighbour = neighbouring_project();
    write_project(&dir, &neighbour).unwrap();

    delete_in(&dir, &project.id, Language::English).unwrap();

    // The whole project left the active library; the neighbouring CV stayed.
    assert_eq!(listed(&dir), vec![format!("{}.json", neighbour.id)]);
    assert!(!current.exists(), "the saved file stayed in the library");
    let names = trash_names(&dir);
    assert_eq!(names.len(), 3, "trash must hold the saved file, its previous revision and the damaged copy: {names:?}");
    assert!(names.contains(&format!("{}.json", project.id)));
    assert!(names.contains(&format!("{}.prev.json", project.id)));
    assert!(names.contains(&kept.file_name().unwrap().to_string_lossy().into_owned()));

    // Bytes moved unchanged: no copy was rewritten by the move.
    assert_eq!(fs::read(dir.join("trash").join(format!("{}.json", project.id))).unwrap(), damaged);
    assert_eq!(
        read_project_file(&dir.join("trash").join(format!("{}.prev.json", project.id))).unwrap().cv.summary,
        first
    );

    // The trashed CV is listed for restore, marked damaged from its kept revision.
    let trashed = summaries_in(&trash_dir(&dir)).unwrap();
    assert_eq!(trashed.len(), 1);
    assert_eq!(trashed[0].id, project.id);
    assert_eq!(trashed[0].name, "Ada Lovelace");
    assert!(trashed[0].damaged);

    // Deleting it again is reported, and nothing cleans Trash automatically.
    assert!(delete_in(&dir, &project.id, Language::English).is_err());
    assert_eq!(summaries_in(&trash_dir(&dir)).unwrap().len(), 1);
    assert_eq!(trash_names(&dir).len(), 3);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn restore_returns_the_cv_and_its_recovery_state_and_still_renders() {
    let dir = temp_dir("trash-restore");
    let project = project();
    write_project(&dir, &project).unwrap();
    let first = project.cv.summary.clone();
    let mut second = project.clone();
    second.cv.summary = "Second saved revision.".into();
    write_project(&dir, &second).unwrap();

    delete_in(&dir, &project.id, Language::English).unwrap();
    // A half-finished permanent deletion can leave its save guard behind; the
    // restore must clear it or the restored CV could never be saved.
    fs::write(deleted_marker(&dir, &project.id).unwrap(), b"").unwrap();
    restore_in(&dir, &project.id, Language::English).unwrap();

    assert_eq!(listed(&dir), vec![format!("{}.json", project.id)]);
    assert!(trash_names(&dir).is_empty(), "restore left a copy in Trash");
    assert!(!deleted_marker(&dir, &project.id).unwrap().exists(), "restore kept the deletion marker");
    let restored = read_project(&dir, &project.id).unwrap();
    assert_eq!(restored.cv.summary, "Second saved revision.");
    assert_eq!(
        read_project_file(&previous_path(&dir, &project.id).unwrap()).unwrap().cv.summary,
        first,
        "the previous revision did not travel with the restored CV"
    );

    // The restored CV reopens and generates a PDF with its content.
    let reopened = open_in(&dir, &project.id, Language::English).unwrap();
    assert!(reopened.recovery.is_none());
    let document = render_document(&reopened.project.cv, reopened.project.structure, reopened.project.content_language).unwrap();
    assert!(document_text(&document).contains("Ada Lovelace"));

    // Saving works again: the Trash marker does not outlive the restore.
    let mut edited = restored;
    edited.cv.summary = "Edited after restore.".into();
    save_in(&dir, &edited, Language::English).unwrap();
    assert_eq!(read_project(&dir, &project.id).unwrap().cv.summary, "Edited after restore.");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_damaged_cv_survives_trash_restore_and_recovery() {
    let dir = temp_dir("trash-damaged");
    let project = project();
    write_project(&dir, &project).unwrap();
    let first = project.cv.summary.clone();
    let mut second = project.clone();
    second.cv.summary = "Second saved revision.".into();
    write_project(&dir, &second).unwrap();
    fs::write(project_path(&dir, &project.id).unwrap(), b"corrupt current").unwrap();

    delete_in(&dir, &project.id, Language::English).unwrap();
    assert!(summaries_in(&dir).unwrap().is_empty(), "a damaged CV was deleted from view but still listed");
    assert!(summaries_in(&trash_dir(&dir)).unwrap()[0].damaged);

    restore_in(&dir, &project.id, Language::English).unwrap();
    let opened = open_in(&dir, &project.id, Language::English).unwrap();
    assert_eq!(opened.project.cv.summary, first, "recovery lost the previous revision after a Trash round-trip");
    assert!(opened.recovery.unwrap().notice.contains("previous saved revision"));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn restore_refuses_to_overwrite_an_active_project() {
    let dir = temp_dir("trash-collision");
    let project = project();
    write_project(&dir, &project).unwrap();
    delete_in(&dir, &project.id, Language::English).unwrap();

    // A new project happens to use the same identity while the old one waits in Trash.
    write_project(&dir, &project).unwrap();
    let active = library_snapshot(&dir);
    let trashed = trash_names(&dir);

    let error = restore_in(&dir, &project.id, Language::English).unwrap_err();
    assert!(error.contains("already in the library"), "{error}");
    assert_eq!(library_snapshot(&dir), active, "restore overwrote an active project");
    assert_eq!(trash_names(&dir), trashed, "restore moved a copy it refused to place");

    // Once the active copy is removed, the same restore succeeds.
    fs::remove_file(project_path(&dir, &project.id).unwrap()).unwrap();
    restore_in(&dir, &project.id, Language::English).unwrap();
    assert_eq!(listed(&dir), vec![format!("{}.json", project.id)]);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn permanent_deletion_removes_only_the_selected_cv_and_leaves_exported_backups() {
    let dir = temp_dir("trash-permanent");
    let project = project();
    let neighbour = neighbouring_project();
    for entry in [&project, &neighbour] {
        write_project(&dir, entry).unwrap();
        let mut second = entry.clone();
        second.cv.summary = "Second saved revision.".into();
        write_project(&dir, &second).unwrap();
    }
    delete_in(&dir, &project.id, Language::English).unwrap();
    delete_in(&dir, &neighbour.id, Language::English).unwrap();

    // An exported backup outside the library is not part of deletion.
    let files = temp_dir("trash-external-backup");
    let backup = files.join("cv-backup.json");
    fs::write(&backup, b"{\"format\":\"cv-generator-backup\"}").unwrap();
    let before = fs::read(&backup).unwrap();

    delete_permanently_in(&dir, &project.id, Language::English).unwrap();

    let names = trash_names(&dir);
    assert_eq!(
        names,
        vec![format!("{}.deleted.json", project.id), format!("{}.json", neighbour.id), format!("{}.prev.json", neighbour.id)],
        "permanent deletion touched another CV: {names:?}"
    );
    assert!(summaries_in(&trash_dir(&dir)).unwrap().iter().all(|entry| entry.id != project.id));
    assert_eq!(fs::read(&backup).unwrap(), before, "an exported backup outside the library changed");

    // The neighbouring CV is still restorable after the other was erased.
    restore_in(&dir, &neighbour.id, Language::English).unwrap();
    assert_eq!(listed(&dir), vec![format!("{}.json", neighbour.id)]);
    assert_eq!(read_project(&dir, &neighbour.id).unwrap().cv.name, "Grace Hopper");

    delete_in(&dir, &neighbour.id, Language::English).unwrap();
    delete_permanently_in(&dir, &neighbour.id, Language::English).unwrap();
    assert!(summaries_in(&trash_dir(&dir)).unwrap().is_empty(), "a permanently deleted CV is still listed in Trash");
    assert!(delete_permanently_in(&dir, &neighbour.id, Language::English).is_err());
    let _ = fs::remove_dir_all(&files);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_failed_internal_cleanup_keeps_the_trash_row_retryable() {
    let dir = temp_dir("trash-retry");
    let project = project();
    write_project(&dir, &project).unwrap();
    let mut second = project.clone();
    second.cv.summary = "Second saved revision.".into();
    write_project(&dir, &second).unwrap();
    delete_in(&dir, &project.id, Language::English).unwrap();

    // A folder where a recovery copy belongs makes that removal fail. The
    // listed saved file must survive so the row stays and can be retried.
    let blocked = trash_dir(&dir).join(format!("{}.corrupt-blocked.json", project.id));
    fs::create_dir(&blocked).unwrap();
    let error = delete_permanently_in(&dir, &project.id, Language::English).unwrap_err();
    assert!(!error.is_empty());
    assert!(
        trash_path(&dir, &project.id).unwrap().exists(),
        "a failed cleanup unlisted the CV before it was gone"
    );
    assert_eq!(summaries_in(&trash_dir(&dir)).unwrap().len(), 1);
    assert!(deleted_marker(&dir, &project.id).unwrap().exists());

    fs::remove_dir(&blocked).unwrap();
    delete_permanently_in(&dir, &project.id, Language::English).unwrap();
    assert!(summaries_in(&trash_dir(&dir)).unwrap().is_empty(), "the retried cleanup left the row");
    assert!(
        deleted_marker(&dir, &project.id).unwrap().exists(),
        "a retried deletion must still block a late save"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_late_save_cannot_resurrect_a_deleted_or_permanently_deleted_cv() {
    let dir = temp_dir("trash-no-resurrect");
    let project = project();
    write_project(&dir, &project).unwrap();
    delete_in(&dir, &project.id, Language::English).unwrap();

    // A save already in flight when the CV reached Trash must not write it back.
    let error = save_in(&dir, &project, Language::English).unwrap_err();
    assert!(error.contains("in Trash"), "{error}");
    assert!(!project_path(&dir, &project.id).unwrap().exists(), "a late save recreated a trashed CV");
    assert!(summaries_in(&dir).unwrap().is_empty());

    delete_permanently_in(&dir, &project.id, Language::English).unwrap();
    let error = save_in(&dir, &project, Language::English).unwrap_err();
    assert!(error.contains("permanently deleted"), "{error}");
    assert!(!project_path(&dir, &project.id).unwrap().exists(), "a late save resurrected a deleted CV");
    assert!(summaries_in(&dir).unwrap().is_empty());
    assert!(summaries_in(&trash_dir(&dir)).unwrap().is_empty());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn trash_controls_and_refusals_follow_the_interface_language() {
    let dir = temp_dir("trash-messages");
    let project = project();
    write_project(&dir, &project).unwrap();

    let english = delete_in(&dir, "missing-cv", Language::English).unwrap_err();
    let indonesian = delete_in(&dir, "missing-cv", Language::Indonesian).unwrap_err();
    assert!(english.contains("not in the library"), "{english}");
    assert!(indonesian.contains("tidak ada di pustaka"), "{indonesian}");

    delete_in(&dir, &project.id, Language::English).unwrap();
    let english = restore_in(&dir, &neighbouring_project().id, Language::English).unwrap_err();
    let indonesian = restore_in(&dir, &neighbouring_project().id, Language::Indonesian).unwrap_err();
    assert!(english.contains("no longer in Trash"), "{english}");
    assert!(indonesian.contains("tidak lagi ada di Sampah"), "{indonesian}");

    let error = save_in(&dir, &project, Language::Indonesian).unwrap_err();
    assert!(error.contains("ada di Sampah") && !error.contains("in Trash"), "{error}");

    let english = delete_permanently_in(&dir, "missing-cv", Language::English).unwrap_err();
    let indonesian = delete_permanently_in(&dir, "missing-cv", Language::Indonesian).unwrap_err();
    assert!(english.contains("no longer in Trash"), "{english}");
    assert!(indonesian.contains("tidak lagi ada di Sampah"), "{indonesian}");
    let _ = fs::remove_dir_all(&dir);
}
