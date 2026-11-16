# CV Generator Design

Status: Design confirmed by the user (design only). Local specification and draft ticket synthesis are authorized; implementation, dependency installation, commits, and external publication are not authorized.

## Confirmed product boundaries

- Build a desktop application using Tauri and Rust.
- Serve individual job seekers; exclude recruiter workflows from the initial release.
- Require both Windows and Linux for the first release. macOS is not a first-release requirement.
- Core functionality works fully offline with local data. No login, backend, cloud sync, or AI dependency in the initial release.
- Support three content structures: reverse chronological, functional, and combination.
- Apply ATS-friendly presentation to each structure rather than implement ATS-friendly as an unrelated fourth generator.
- Switching content structure reorganizes existing information without requiring re-entry.
- Export a text-based PDF and a portable, editable project backup. Defer DOCX unless an explicit need emerges.
- Support manually authored Indonesian and English CV content. Do not provide automatic translation initially.

## Confirmed CV model

- One project contains one CV. Duplication creates an independent copy; edits never propagate between projects. There is no shared master profile.
- Each CV has one content language. Users duplicate and manually translate a CV to create another language version; no translation synchronization occurs.
- Standard section headings follow the CV content language. Interface language is independently selectable as Indonesian or English; switching it never translates or mutates CV content.
- Initial sections: contact details, professional summary, work experience, education, skills, projects, certifications, and languages.
- Drafts permit incomplete content and all sections are optional while drafting. Empty sections are omitted from exports.
- Export requires a name and at least one substantive non-contact section. Completely blank entries are ignored. Missing optional information produces warnings, not invented values.
- Clearly contradictory date ranges and malformed supplied email addresses or web links block export.
- Missing skill-group achievements in functional or combination CVs produce a warning rather than another mandatory export requirement.
- Phone number, address, photo, and work history are not mandatory. Fresh graduates must be supported. Photos are excluded from the initial presentation entirely.
- Career content and achievements are user-authored. The application does not invent career claims.

## Confirmed content structures

- Reverse chronological: detailed employment history ordered newest first, with skills as supporting content.
- Functional: named skill groups with achievement bullets lead; employment history uses a compact employer/title/date list.
- Combination: named skill groups with achievement bullets plus detailed reverse chronological employment history.
- A project retains both employment details and skill-focused content. Switching structures changes presentation without deleting hidden content or requiring re-entry.

## Confirmed presentation and output

- One conservative presentation is shared by all three content structures.
- Use a single column, standard section headings, and selectable text.
- Exclude decorative templates, photos, icons, tables, and skill-rating bars from initial exported CVs.
- Contact details appear in the document body, not page headers or footers.
- Export A4 PDF with automatic pagination and no arbitrary one-page limit.
- Never truncate content or silently shrink text to force a single page.
- Keep headings with following content where possible; allow long sections to span pages.
- Letter size is deferred unless explicitly required.
- ATS-friendly presentation does not guarantee ATS acceptance or identical parsing across systems.

## Confirmed editing and persistence

- Use structured section forms with add, remove, and reorder controls, plus an adjacent preview. Input is plain text and achievement bullets, not rich text or freely positioned document elements.
- Use an application-managed local library with one versioned JSON file per CV and no database initially.
- Autosave locally, use atomic file replacement, and show accurate Saving / Saved / Save failed states.
- Exported project backups are independent snapshots that users can store elsewhere. Nothing uploads automatically.
- Interface language can be Indonesian or English, independently of CV content language.

## Confirmed dates and preview

- Support month/year, year-only, omitted dates, and Present for ongoing roles. Preserve entered precision and never infer a missing month.
- Incomplete dates are allowed in drafts. Block export only for date ranges provably contradictory at the supplied precision; do not infer missing months. Preserve manual order where date precision cannot establish chronology.
- Preview displays the actual generated PDF, not a separate HTML approximation. Refresh after a short editing pause and mark the preview outdated during regeneration.
- PDF preview/export fidelity is an accepted requirement, not a verified renderer capability. Technical research and cross-platform verification remain necessary.

## Confirmed recovery, import, and privacy

- Autosave after 1 second without edits, retaining one previous valid revision. Recover from that revision when the current file is corrupted, with an explicit recovery notice; never silently replace corrupted data.
- On normal close, immediately save pending changes. If saving fails, offer Retry / Export backup / Discard and close / Cancel; retain unsaved edits in memory until explicitly discarded or successfully saved. A crash may lose edits not yet saved. The idle interval is a proposed interaction default, not a measured performance guarantee.
- Project backup import always creates an independent CV, including repeated imports of the same backup. Never automatically overwrite an existing project.
- Validate file size, schema, and supported version before importing. Reject malformed files and newer unsupported formats without modifying the library. Imported content is plain text, not executable markup.
- Delete moves a project to local Trash. Restore is available; permanent deletion requires explicit confirmation. No automatic cleanup initially.
- One rolling previous valid revision is retained for recovery, not full version history. This supersedes the earlier wording suggesting all previous revisions remain until explicitly removed. Trash remains until explicitly emptied; permanent deletion removes the project and its internal recovery copy, but not backups exported elsewhere. Deletion is not secure erasure.
- Working JSON files and exported backups are unencrypted. Use OS account permissions for ordinary local-file protection; no application password or encryption initially.
- No telemetry, remote fonts, automatic uploads, or CV content in application logs.

## Confirmed chronological ordering

- For dated histories, ongoing entries appear first, followed by newest known start dates. Undated entries appear last.
- Preserve manual order for ties or date precision that does not establish an order. Do not invent dates to sort entries.
- Manual reordering remains available for skill groups, bullets, and other non-chronological lists.

## Confirmed application architecture and behavior

- Use React, TypeScript, and Vite for section forms and preview. Rust handles file access, authoritative validation, persistence, and export orchestration.
- Initially exclude a router, global state library, database, and generic repository layer. Use narrowly scoped Tauri commands; imported IDs never grant unrestricted filesystem access.
- Allow one application instance and one active CV editor initially.
- Associate preview results with a content revision and discard late results from older revisions.
- Export only the current revision, regenerating when necessary. On rendering failure, retain the previous preview with an outdated indication; never silently export it as current.
- Confirm overwrites and avoid partially written PDFs or project backups.
- Provide a library screen and editor. Wide windows show forms and preview side by side; narrow windows use switchable tabs.
- Require keyboard operation, visible focus, explicit field labels, accessible error messages, and reorder buttons. Drag-and-drop is not required initially.

## Confirmed platform scope

- Required first-release targets: Windows 11 x64 and Ubuntu 24.04 LTS x64.
- Arch/Omarchy is an additional smoke-test target, not a blanket rolling-release compatibility promise.
- Defer Windows 10, ARM, other Linux distributions, and macOS.
- This refines the earlier Windows/Linux requirement into testable release boundaries. Acceptance of these targets is not verification of compatibility.

## Confirmed rendering stack

- Embed Typst through Rust for PDF typesetting and bundle PDF.js for viewing the resulting PDF bytes.
- Bundle fixed application templates and a redistributable font family. Verify the chosen font's redistribution license and preserve required third-party license notices before inclusion.
- Treat CV input as plain data, never executable Typst source. Restrict the compiler to application-controlled templates, fonts, and input data, without arbitrary filesystem access or downloadable packages.
- Preview and export use the same generated artifact for the current revision. Reuse the current PDF bytes; regenerate only when they do not represent current content. Never export an older revision as current.
- Bundle PDF.js and its matching worker and required assets locally; do not rely on a platform's built-in PDF viewer or a CDN.
- Engine selection is accepted architecture, not evidence of rendering correctness, performance, or WebView compatibility.

## Confirmed packaging and distribution

- Windows: NSIS setup executable. Ubuntu: Debian package. Provide an AppImage for the additional Arch/Omarchy smoke test.
- Editing, preview, and export operate offline. First installation may require internet for OS/runtime dependencies such as WebView2 or Linux packages; air-gapped installation is not an initial requirement.
- Build and test Windows on Windows, and Linux packages against the accepted Ubuntu baseline. AppImage does not guarantee universal Linux compatibility.
- Initial delivery consists of locally installable test packages. No automatic updater, public publication, store submission, or signing-certificate purchase.
- Unsigned Windows packages may show security warnings. Do not bypass OS protections; public distribution and signing require separate approval.

## Acceptance and delivery gates

First implementation milestone, only after authorization: a small rendering integration check before the full editor. Verify Rust Typst generation, locally bundled fonts, PDF.js in Tauri, extractable text, long-content pagination, and preview/export artifact identity. If the stack fails a requirement, report the evidence and revisit the choice rather than silently weaken requirements.

Release completion requires real evidence for:

- All three content structures in both content languages, including fresh-graduate and multipage cases; switching structures preserves hidden content.
- Extractable PDF text in correct reading order, no clipped or lost content, and matching current preview/export revisions. Include long words, links, and headings near page boundaries.
- Offline editing, preview, and export without remote assets or services.
- Independent duplication and import, malformed/unsupported file rejection without library mutation, crash recovery, save-failure handling, Trash/Restore, permanent deletion, and overwrite protection.
- Interface language switching without CV content mutation, keyboard editing, and narrow-window behavior.
- Installation and core workflows on Windows 11 x64 and Ubuntu 24.04 LTS x64, plus an additional Arch/Omarchy smoke test.

## External prerequisites and unresolved verification

- Provide native Windows and Ubuntu build/test environments and an Arch/Omarchy smoke-test environment. Availability has not been verified.
- Verify a compatible dependency/toolchain set, selected font licensing and coverage, bundled worker loading, and platform-specific packaging/runtime dependencies during implementation.
- Measure rendering responsiveness and verify text extraction, long-content behavior, and PDF preview/export fidelity on the supported targets.
- These are execution prerequisites and verification gates, not undecided product features. No scaffold, build, export, installer, or platform test exists yet.

## Authorization boundary

The user confirmed shared understanding with “confirm design only.” The subsequent request authorizes local specification and draft ticket synthesis, not implementation or external publication. Runtime changes, dependency installation, commits, and publication require separate authorization. Ticket granularity, blocking edges, and the proposed test seam remain subject to review before drafts receive ready-for-agent status.
