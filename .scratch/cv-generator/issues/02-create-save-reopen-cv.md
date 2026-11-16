# 02 — Create, edit, autosave, and reopen a basic CV

**What to build:** A job seeker creates a named CV project in a local library, edits contact details, a summary, and work experience, previews a reverse chronological CV, and reopens saved work after restarting the application.

**Blocked by:** 01 — Prove offline PDF generation and identical preview/export. (Accepted on Arch/Omarchy; native Windows/Ubuntu runs deferred to 10/11.)

**Status:** done — implemented and runtime-verified on Arch/Omarchy. Evidence: [../../evidence/ticket02/README.md](../../evidence/ticket02/README.md).

**Proposed triage:** ready-for-agent after approval. Execution still requires explicit implementation authorization.

**Story coverage:** US-05, US-06, US-07, US-08, US-09, US-16, US-17, US-18, US-21, US-38, US-39.

- [x] Provide a local library and one active editor for independently identified projects; create, name, open, and revisit multiple CVs without an account or network service.
  `list_projects` / `create_project` / `open_project` / `save_project` in `src-tauri/src/main.rs`; library screen and one editor in `src/App.tsx`. Verified in the running app: empty library → `New CV` → project listed after restart → reopened.
- [x] Edit contact details, professional summary, and employment roles with plain text and achievement bullets. No phone number, address, or employment history is mandatory.
  Plain inputs plus a one-bullet-per-line textarea per role. A project with only a name and summary exports successfully (fresh-graduate case below); blank phone/location are omitted from the PDF. Contact links exist in the schema but have no editing control yet (ticket 03).
- [x] Use an initial documented versioned project schema that can represent one CV, its selected structure, and content language; do not add a database or shared master profile.
  Schema version 1 documented in [docs/project-schema.md](../../../docs/project-schema.md); one JSON file per project under the application data directory, no database. Verified on disk after a real edit: `"version": 1`, `"structure": "reverse-chronological"`, `"content_language": "english"`.
- [x] Autosave after one second without edits with atomic replacement and truthful Saving / Saved / Save failed states. Editing and reopening demonstrate durable data, not just in-memory state.
  Debounced 1 s autosave writes through the existing temp-file-plus-rename path; badge states are `Unsaved changes` / `Saving…` / `Saved` / `Save failed`. Verified: typing → `Saved` → edited fields and `updated_at` present in the project JSON; restart → `open_project` restored the same values.
- [x] Incomplete drafts persist; empty sections/blank entries disappear from PDF output. Export requires a name and meaningful non-contact content, without invented data.
  Template filters blank entries, blank bullets and whitespace-only summaries; a draft with only a name still saves. `Cv::export_problem` in Rust rejects export without a name or a substantive non-contact section; an empty end date renders the start alone instead of claiming "Present". Covered by `blank_draft_entries_and_sections_do_not_render`, `a_missing_end_date_is_not_reported_as_present`, `export_requires_a_name_and_a_non_contact_section`.
- [x] Connect the existing Rust PDF generation and preview path to the edited reverse chronological CV. No separate approximate HTML document preview is introduced.
  The editor sends the whole edited CV to `render_pdf`; the preview remains PDF.js canvases of the Rust-produced bytes and export reuses those cached bytes. Rust now also caches the CV those bytes were rendered from and refuses export when the content has changed since, so the identity guarantee no longer depends on the frontend (`CachedRender::matches`). Verified: preview and export SHA-256 identical after editing; regeneration-on-export remains ticket 09.
- [x] Failed writes never report Saved or silently discard edits; retain them for continued editing/retry. Full recovery and shutdown choices are completed by the later safety slice.
  Verified by making the library directory read-only mid-session: badge became `Save failed`, the error was shown, the project file on disk stayed unchanged, the edit stayed in the form, and `Retry save` after restoring permissions persisted it. Edits arriving during an in-flight write are re-saved rather than reported as saved.
- [x] Exercise create/edit/save/restart/reopen and export through the running desktop application, including a minimal fresh-graduate CV with a summary and no work history.
  Fresh-graduate CV (name, email, location, summary, no experience) exported as A4 595.276 × 841.89 pt with `pdftotext` reading order name → contact → `PROFESSIONAL SUMMARY` → text and no empty work-experience heading; the app reported `identical bytes to preview`.
