# 03 — Edit all career sections with precise dates and validation

**What to build:** A job seeker completes education, skills, projects, certifications, and languages alongside employment, manages entries and bullets, and exports a validated reverse chronological CV without invented date precision.

**Blocked by:** 02 — Create, edit, autosave, and reopen a basic CV.

**Status:** done — implemented, code/PDF seam verified, real save → restart → reopen verified, and two-axis reviewed. UI export and validation clicks are the only unclicked criteria; see [evidence](../../evidence/ticket03/README.md). Review disposition: [../../evidence/ticket03/README.md](../../evidence/ticket03/README.md#review-findings-disposition-code-review-v2-snapshot-641eb85c).

**Proposed triage:** ready-for-agent after approval. Execution still requires explicit implementation authorization.

**Story coverage:** US-10, US-11, US-12, US-13, US-14, US-15, US-16, US-17, US-18, US-19, US-20, US-58.

- [x] Add structured education, supporting skills, projects, certifications, and languages to the existing editor, durable project data, reopening workflow, and PDF output.
  `Cv`/`cv.ts` gained `education`, `skills`, `projects`, `certifications`, `languages`; `src/Sections.tsx` owns the editors. Real UI run saved all five sections, and after restart the reopened project restored them (`44-reopened.png`); `every_career_section_reaches_the_pdf_in_order` asserts each section's text in the Rust-rendered PDF.
- [x] Provide explicit add/remove/reorder buttons for relevant entries and bullets with accessible labels; preserve saved order for non-chronological lists.
  `ItemControls` renders ↑ / ↓ / Remove per entry and per bullet with labels such as `Move work experience 1 up`; lists persist in manual order in the project JSON. Buttons were labelled and code-reviewed but not clicked in this run; manual order is pinned by the ordering checks.
- [x] Preserve month/year, year-only, omitted dates, and Present for ongoing roles across editing, save/reopen, and PDF output. Never materialize a missing month.
  Saved JSON from the UI: work start `{"year":2021,"month":2}`, `"end":null`; education end `{"year":2018,"month":null}`. PDF text read `(Feb 2021 – Present)` and `(Aug 2016 – 2018)`; `mixed_date_precision_never_invents_a_month` and `a_missing_end_date_is_not_reported_as_present` pin the rules.
- [x] Display ongoing histories first, then newest known start dates, then undated histories. Preserve manual order for ties or date precision that cannot establish order; avoid an inconsistent date comparator.
  `ordered_histories` puts ongoing-with-start first, then start-year groups, then every undated entry last; within a year, months only order the group when all entries have one. `ongoing_entries_lead_then_newest_start_then_undated` and `equal_or_imprecise_dates_keep_the_manual_order`.
- [x] Save partial entries as drafts. Ignore completely blank entries during export, warn on missing optional content, and require only the accepted name plus substantive non-contact content baseline.
  A blank work-experience entry was saved in the reopened project and filtered out of the PDF (`blank_draft_entries_and_sections_do_not_render`, `blank_new_section_entries_do_not_render`); `export_requires_a_name_and_a_non_contact_section`; warnings returned with the export acknowledgement and rendered in the UI (`export_warnings_name_missing_optional_content`). Warning UI not clicked through.
- [x] Block export for provably contradictory date ranges and malformed supplied email or web-link values. Validation is authoritative in Rust and errors are accessible in the UI.
  `contradictory_ranges_block_export_but_imprecise_ones_do_not`, `malformed_supplied_email_or_links_block_export`, `invalid_date_shapes_are_refused_before_rendering`. Errors reach the existing `role="alert"` surface; the new messages were not re-clicked in this run.
- [x] Demonstrate a fresh-graduate CV using education/projects without employment and an experienced multipage CV with all sections. Verify selected text reaches the PDF and survives restart.
  `fresh_graduate_cv_exports_without_employment` wrote `fresh-graduate.pdf` (1 page A4; `pdftotext`: summary → education → projects, no `WORK EXPERIENCE`); `every_section_survives_a_multipage_pdf` wrote `experienced-multipage.pdf` (2 pages A4; headings in order). The all-sections CV was saved, the app restarted, reopened, and every value restored. PDF evidence is Rust-engine level; the app export button itself was not re-clicked.
- [x] Add focused edge checks for mixed date precision, ongoing roles, equal dates, missing dates, and invalid provided contact/link values, while retaining application-level acceptance evidence.
  Checks listed above plus `all_career_sections_survive_a_saved_project_file`; application-level evidence is the saved JSON, preview screenshots, and the restart/reopen run in [ticket 03 evidence](../../evidence/ticket03/README.md).
