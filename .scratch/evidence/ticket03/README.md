# Evidence — ticket 03 all career sections, dates, ordering, validation

Synthetic fixtures only. No real career data was entered, logged, or exported.
The application ran against an isolated `XDG_DATA_HOME`
(`/home/airzy/.hermes/cache/scratch/cvgen-ticket03/data`), so the real local
library was never touched.

## Code checks

```bash
cargo test --manifest-path src-tauri/Cargo.toml     # 27 checks, all pass
npm run build                                       # tsc --noEmit + vite build
npx tauri build --debug --no-bundle                 # frontend embedded
```

Ticket-03 checks: `mixed_date_precision_never_invents_a_month`,
`ongoing_entries_lead_then_newest_start_then_undated`,
`equal_or_imprecise_dates_keep_the_manual_order`,
`contradictory_ranges_block_export_but_imprecise_ones_do_not`,
`malformed_supplied_email_or_links_block_export`,
`invalid_date_shapes_are_refused_before_rendering`,
`every_career_section_reaches_the_pdf_in_order`,
`every_section_survives_a_multipage_pdf`,
`fresh_graduate_cv_exports_without_employment`,
`blank_new_section_entries_do_not_render`,
`all_career_sections_survive_a_saved_project_file`,
`export_warnings_name_missing_optional_content`.

## PDF artifacts (Rust engine, inspected with pdfinfo/pdftotext)

| Artifact | Pages | Extracted text confirms |
| --- | --- | --- |
| [`experienced-multipage.pdf`](experienced-multipage.pdf) | 2, A4 595.276 × 841.89 pt | `PROFESSIONAL SUMMARY` → `WORK EXPERIENCE` → `EDUCATION` → `SKILLS` → `PROJECTS` → `CERTIFICATIONS` → `LANGUAGES` in that order, with `BSc Testing`, `Test automation`, `Synthetic project`, `Certified tester`, `Indonesian — Native` |
| [`fresh-graduate.pdf`](fresh-graduate.pdf) | 1, A4 | `Synthetic Graduate` → contact → `PROFESSIONAL SUMMARY` → `EDUCATION` (`BSc Testing, Test University (2022 – 2025)`) → `PROJECTS`; no `WORK EXPERIENCE` heading |

## Application path (real UI, headless X)

Screenshots: `01-editor.png` … `45-final-editor.png`, driver
[`drive.py`](drive.py).

1. Created `Full career CV` and saved contacts and summary through the real form
   (`02-basics.png`).
2. Work experience `Senior Test Engineer` / `Synthetic Systems Ltd` / `Testville`
   with start `February 2021`, `Ongoing (Present)` checked, and two achievement
   bullets. Saved JSON: `"start":{"year":2021,"month":2}`, `"end":null`, both
   bullet strings (`19-bullets.png`).
3. Education `BSc Synthetic Testing` / `Test University`, start `August 2016`,
   end year `2018` with no month. Preview rendered `(Aug 2016 – 2018)` and never
   invented a month or `Present` (`25-education-done.png`).
4. Skill group `Test automation` with one achievement bullet, project
   `Synthetic Renderer` / `Offline PDF tooling` with one bullet, certification
   `Certified Synthetic Tester` with year-only date `2023`, and language
   `Indonesian — Native` (`27`–`38`).
5. Restarted the application and reopened the project. The library listed
   `Full career CV`; every section was restored and the preview re-rendered all
   seven headings (`43-restart-library.png`, `44-reopened.png`). Editing the
   summary after reopen autosaved again, so the reopen path is durable rather
   than read-only.
6. A blank work-experience entry was saved in the library as a draft; the
   blank-entry filters keep it out of the PDF (see the two `blank_*` checks).

## What was proven where

- **Code/PDF seam:** all-section and multipage pagination, fresh-graduate output,
  date precision, ordering, blank filtering, export validation, warnings, and
  project-file round-trip. The `pdfinfo`/`pdftotext` results above are real
  artifacts, not screenshots of the preview.
- **Real UI:** entry of every section, autosave to disk, restart → reopen →
  restored values and preview, and the blank draft entry.
- **Not clicked through this run:** the `Export current PDF…` button and the
  three export-rejection messages. Preview/export identity and the
  `role="alert"` error surface are unchanged from ticket 02, where both were
  exercised; the new validation messages themselves are pinned by the Rust
  checks. Reorder buttons are present with accessible labels but were not
  clicked in this run; manual order persistence is covered by the JSON/PDF
  ordering checks.

## Review findings disposition (code-review v2, snapshot `641eb85c…`)

| Finding | Disposition |
| --- | --- |
| Fixed English section headings although `docs/design.md` says headings follow the CV content language | **Deferred to slice 5** — ticket 05 owns content language and standard headings; pre-existing for `Work experience`. `render_pdf` receives only `Cv`, so the language must be passed to the renderer there. |
| `CvDate` month range/without-year enforced only by the UI select | **Fixed in this change** — `CvDate::shape_is_valid` via `Cv::date_shape_problem`, used by `export_problem` and `render_document`; regression added. |
| Ongoing entries without a start date were rendered first while the warning said undated entries are listed last | **Fixed in this change** — undated entries (ongoing or not) go last; regression pins it. |
| Reorder buttons inside `<legend>` polluted the fieldset's accessible name | **Fixed in this change** — legend is plain text, controls live in a sibling `.entry-head` row. |
| Schema version 1 shape changed without a migration; ticket-02 files would not open | **Accepted local limit** — the app is unreleased and the real library directory (`~/.local/share/app.cvgen.desktop`) was never created (verified absent), so no v1 project data exists. Versioned migration belongs with backup import/recovery (slices 06/07); `list_projects` skipping unreadable files is ticket 07's recovery notice. |
| Export warnings cover only missing contact details and undated entries | **Deferred to slice 4** for missing skill-group achievements (US-25); missing summary/location/links is an **accepted local limit** — the two warnings name the cases that change what a PDF reader can do. |
| Seven near-identical section shells; blank-entry rule encoded in Rust and Typst; month names duplicated in `cv.ts`/`cv.typ`; `Language` vs glossary | **Accepted local limits / judgement calls** — Rust and Typst cannot share constants, and the explicit section shells are deferred until structures diverge in slice 4. `DateField` export was dropped. |
