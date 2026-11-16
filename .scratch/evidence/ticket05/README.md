# Evidence — ticket 05 independent copies and separate interface/content languages

Synthetic fixtures only. No real career data was entered, logged, or exported.
The application ran against an isolated `XDG_DATA_HOME`
(`/home/airzy/.hermes/cache/scratch/cvgen-ticket05/data`), so the real local
library was never touched.

## Environment

Omarchy (Arch-based), Rust 1.99.0, Node 26.12.0, Tauri 2, Typst, PDF.js, Xvfb
`:99`, and the local XTEST driver [`drive.py`](drive.py) (plus
[`click_export.py`](click_export.py), which locates the export button by OCR
because the form scrolls itself as fields take focus). The debug binary embeds
the built frontend (`npx tauri build --debug --no-bundle`); the Rust backend,
project files, renderer, cache, and native save dialog are real application
paths.

## Code checks

```bash
cargo test --manifest-path src-tauri/Cargo.toml     # 44 checks, all pass
npm run build                                       # tsc --noEmit + vite build
npx tauri build --debug --no-bundle                 # succeeds
```

Ticket-05 checks: `content_language_selects_headings_and_present_wording_only`,
`every_structure_renders_in_both_content_languages`,
`duplicate_is_an_independent_copy_of_content_structure_and_language`,
`switching_content_language_preserves_authored_text_on_disk`,
`interface_language_localizes_export_messages_without_changing_content`,
`preview_validation_messages_follow_the_interface_language`,
`contradictory_dates_name_the_entry_in_the_interface_language`,
`cached_bytes_only_match_the_content_structure_and_language_they_were_rendered_from`,
`writes_language_evidence_pdfs`.

## PDF artifacts

### Rust engine (`writes_language_evidence_pdfs`), inspected with pdfinfo/pdftotext

One short synthetic CV rendered as all three structures in both content
languages. Every file is 1 page, A4 (`595.276 × 841.89 pts`).

| Artifact | Generated headings | Structure evidence |
| --- | --- | --- |
| `english-reverse-chronological.pdf` | `PROFESSIONAL SUMMARY`, `WORK EXPERIENCE`, `EDUCATION`, `SKILLS`, `PROJECTS`, `CERTIFICATIONS`, `LANGUAGES` | detailed employment before skills, `May 2020 – Present` |
| `english-functional.pdf` | same headings in English | `SKILLS` first, compact `Synthetic Engineer, Example Employer (May 2020 – Present)`, no employment bullet/location |
| `english-combination.pdf` | same headings in English | `SKILLS` first, detailed employment with location and bullet |
| `indonesian-reverse-chronological.pdf` | `RINGKASAN PROFESIONAL`, `PENGALAMAN KERJA`, `PENDIDIKAN`, `KEAHLIAN`, `PROYEK`, `SERTIFIKASI`, `BAHASA` | `Mei 2020 – Sekarang`, detailed employment before skills |
| `indonesian-functional.pdf` | Indonesian headings | `KEAHLIAN` first, compact employment |
| `indonesian-combination.pdf` | Indonesian headings | `KEAHLIAN` first, detailed employment |

Authored text (name, summary, employers, bullets, skill names, month-precision
education dates) is byte-for-byte identical in both languages; only generated
headings, month abbreviations (`Agu`, `Mei`), and the ongoing-role word change.

### Real UI exports (native save dialog, then pdfinfo/pdftotext)

| Artifact | Structure | Content language | Read back |
| --- | --- | --- | --- |
| `ui-reverse-id.pdf` | reverse chronological | Indonesian | `RINGKASAN PROFESIONAL` → `PENGALAMAN KERJA` (detailed, `Mei 2020 – Sekarang`) → `KEAHLIAN` |
| `ui-functional-id.pdf` | functional | Indonesian | `KEAHLIAN` first, compact `(Mei 2020 – Sekarang)`, no employment bullet |
| `ui-combination-id.pdf` | combination | Indonesian | `KEAHLIAN` first, detailed employment with location and bullet |
| `ui-reverse-en.pdf` | reverse chronological | English | English headings, employment before skills, `May 2020 – Present` |
| `ui-functional-en.pdf` | functional | English | `SKILLS` first, compact employment |
| `ui-combination-en.pdf` | combination | English | `SKILLS` first, detailed employment |
| `ui-copy-id.pdf` | combination (copy) | Indonesian | `RINGKASAN PROFESIONAL` + manually authored Indonesian summary |

All seven are 1-page A4 PDFs. The functional pair contains 0 occurrences of the
employment bullet; the other five contain exactly 1.

## Application path (real UI, headless X)

Screenshots `01`–`59`; driver [`drive.py`](drive.py).

1. Seeded one synthetic CV (`English CV`, reverse chronological, content
   language English) into the isolated library; the library listed it under an
   English interface (`live.png`).
2. Tabbed to the interface selector and changed it to **Bahasa Indonesia**:
   `Pustaka lokal — proyek tetap di perangkat ini.`, `CV Baru`, `Muat ulang`,
   `Duplikat` (`02-library-indonesian.png`). No project file was touched.
3. Opened the CV: Indonesian interface, English PDF headings, and the
   Indonesian content-language helper text
   (`03-editor-indonesian.png`).
4. Edited the CV through the Indonesian interface: focused the summary
   textarea and typed ` Edited under an Indonesian interface.`. Stored JSON
   became `"summary": "Authored English summary stays as written. Edited under
   an Indonesian interface."` while the interface stayed Indonesian.
5. Switched **Bahasa konten** to `Bahasa Indonesia`
   (`12-content-indonesian-preview.png`): the preview re-rendered with
   `RINGKASAN PROFESIONAL` / `PENGALAMAN KERJA` / `KEAHLIAN` and
   `Mei 2020 – Sekarang`; the authored English summary was unchanged. Stored
   `content_language: "indonesian"`.
6. Selected `Fungsional` (`13-structure-functional-id.png`) and exported through
   the native save dialog (`ui-functional-id.pdf`); then `Kombinasi`
   (`24-structure-combination-id.png`, `ui-combination-id.pdf`) and
   `Kronologis terbalik` (`34`–`35`, `ui-reverse-id.pdf`). The preview status
   read `1 halaman, revisi N` for each and the export row reported identical
   bytes to the preview.
7. Switched **Bahasa konten** back to `English` and exported the same three
   structures (`ui-reverse-en.pdf`, `ui-functional-en.pdf`,
   `ui-combination-en.pdf`). Generated headings and Present wording followed
   the content language in every file; authored text did not.
8. Returned to the library and pressed **Duplikat** on `English CV`
   (`45-library-before-duplicate.png`, `46-duplicate-opened.png`). The library
   then held two files (`t05source.json` and `18e87d1d117c17be-0.json`) with
   equal content, structure, and content language but different ids; the copy
   opened in the editor. (That run predates the v2 id-format fix, so its id
   still contains `-`; step 12 re-runs the same action against v2.)
9. Renamed the copy to `CV Indonesia`, changed its content language to
   `Bahasa Indonesia`, and replaced its summary with the manually authored
   `Ringkasan bahasa Indonesia ditulis manual untuk lamaran ini.`
   (`47-copy-translated.png`). The source file still read `"summary": "Authored
   English summary stays as written. Edited under an Indonesian interface."`
   and `"content_language": "english"` — no synchronization in either
   direction. Exported the copy as `ui-copy-id.pdf`.
10. Restarted the application (`52-restart-library.png`): the Indonesian
    interface preference survived, and the library listed both `CV Indonesia`
    and `English CV`. Reopening the copy restored `Kombinasi` +
    `Bahasa Indonesia` and the translated summary
    (`53-reopened-copy.png`); reopening the source restored `Kombinasi` +
    `English` and the original English summary (`55-reopened-source.png`).
11. Switched the interface English → Bahasa Indonesia twice while the source CV
    was open. `sha256sum` of both project files before and after is identical
    (`3299d9bc…` and `1207cff7…`), so interface switching changed no CV content,
    content language, or `updated_at` (`58-interface-english.png`,
    `59-interface-indonesian-again.png`).
12. Re-ran the library after the v2 review fixes (`60-library-v2.png`,
    `61-library-v2-duplicate.png`): the Indonesian interface now renders
    timestamps as `16/11/2026, 17.07.20` and `16/11/2025, 12.20.00`
    (`id-ID`), and duplicating through `Duplikat` produced
    `18e87e97680a31670000000000000.json` — one lowercase-hex id with no
    separator, matching `docs/project-schema.md`.

## What was proven where

- **Code/PDF seam:** independent duplicate (edit either copy, the other file is
  untouched), heading/Present/month translation per content language, authored
  text preservation, structure projection in both languages, language-aware
  render-cache identity, localized export errors and warnings, and
  content-language persistence through save/reopen.
- **Real UI:** interface language across library, editor, warnings area, and
  preview/export actions; editing an English CV under an Indonesian interface;
  switching one CV's content language without touching its text; six
  structure × language exports through the native dialog; duplication from the
  library; manual translation of the copy with the source unchanged; restart
  and reopen of both copies; byte-identical project files across interface
  switches.
- **Not clicked through this run:** localized *warning* text (missing contact
  details / skill evidence) — the authored CV always had contact details and
  skill evidence, so the warning list stayed empty in the UI. It is pinned by
  `interface_language_localizes_export_messages_without_changing_content` and
  uses the same warning display exercised in tickets 03/04. Narrow-window tabs
  and keyboard-only traversal remain ticket 09.
- **Accepted local limits:** the interface language is an application
  preference stored in the webview's local storage, not a project field, so it
  is deliberately absent from the project schema and backups. Rust IO failures
  (`save failed: <os error>`) keep their English OS detail; only user-facing
  export validation, preview validation, and warnings are localized in this
  slice.
- **Harness note:** pressing Enter in the GTK save dialog can re-activate the
  Export button behind it after the dialog closes, so the driver dismisses the
  second dialog with Escape; the written file is unaffected.

## Review findings disposition (code-review v1)

| Finding | Disposition |
| --- | --- |
| Generated ids used `{hex}-{hex}` while `docs/project-schema.md` promises lowercase hex | **Fixed** — `new_project_id()` emits one lowercase-hex string; the duplicate check asserts the id is lowercase hex. |
| Preview validation errors used the CV *content* language, so an English interface could show Indonesian date errors | **Fixed** — `render_for()` validates with the interface language before rendering; `preview_validation_messages_follow_the_interface_language` fails if the content language leaks back in. |
| Indonesian contradictory-date message dropped the "entri" fallback for an untitled entry | **Fixed** — one localized `name` is used by both messages; `contradictory_dates_name_the_entry_in_the_interface_language` pins `Pengalaman kerja entri` / `Work experience entry`. |
| Dead interface strings `openCv` and `interfaceLanguage` | **Removed** from both dictionaries. |
| Library timestamps ignored the interface language | **Fixed** — `toLocaleString` uses `id-ID` or `en-US` with the selected interface language. |
| Typst helper `label()` shadowed Typst's built-in | **Fixed** — renamed to `word()`. |
| `duplicate_in` set `updated_at = 0` before the write helper overwrote it | **Removed**. |
| Duplicate rows share the source title until renamed | **Accepted reading** — identity is the new id/file; the copy opens on creation and the user names it. Append localized "(copy)" text would persist interface wording into authored metadata. |
| `Language` names both content and interface choices; message pairs built eagerly; two month tables | **Accepted judgement calls** — the call sites name both parameters, messages only build on a validation failure, and interface full month names cannot share the PDF's abbreviations across runtimes. |

