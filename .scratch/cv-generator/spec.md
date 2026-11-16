# Offline Desktop CV Generator — Specification

**Status:** Local draft for test-seam and delivery-slice review. The product design is confirmed; implementation is not authorized.
**Tracker:** Local files only for this request. No remote tracker publication.
**Proposed triage:** ready-for-agent after review approval; the label is not permission to implement, commit, or publish.

## Problem Statement

Individual job seekers need to prepare accurate CVs for different applications without entering the same information repeatedly for each content structure. They need local control of sensitive career information, readable multipage PDFs, and confidence that the preview, exported document, and saved project represent their current work. Fresh graduates must not be forced to invent work history, and users must not be promised universal ATS acceptance.

## Solution

Provide an offline desktop application for Windows and Linux that manages independent CV projects, edits career content through structured forms, and presents that content in reverse chronological, functional, or combination structure. All structures share one conservative ATS-friendly presentation. Users author Indonesian or English content, preview the actual PDF, export that current PDF artifact, and save portable editable project backups. Autosave, explicit recovery, safe import, and reversible deletion protect local work without introducing an account or backend.

## User Stories

1. As a job seeker, I want to install the application on Windows 11 x64, so that I can prepare CVs on a supported Windows desktop.
2. As a job seeker, I want to install the application on Ubuntu 24.04 LTS x64, so that I can prepare CVs on a supported Linux desktop.
3. As a Arch/Omarchy user, I want to have an AppImage with a documented smoke-test result, so that I can evaluate the application without assuming universal rolling-release compatibility.
4. As a job seeker, I want to edit, preview, and export CVs without internet, so that my core workflow does not depend on an online service.
5. As a job seeker, I want to use the application without an account, so that I do not have to register or transmit career data.
6. As a job seeker, I want to create, name, open, and manage independent CV projects in a local library, so that I can prepare more than one application.
7. As a job seeker, I want to enter contact details without mandatory phone numbers or street addresses, so that I can disclose only the contact information I choose.
8. As a job seeker, I want to write a professional summary, so that I can explain my suitability for a role.
9. As a job seeker, I want to record employers, roles, dates, and achievement bullets, so that I can describe my work experience.
10. As a job seeker, I want to record education, so that I can present my qualifications even without employment history.
11. As a job seeker, I want to record skills and named skill groups with achievement bullets, so that I can support capabilities with evidence.
12. As a job seeker, I want to record projects, so that I can demonstrate relevant work outside employment.
13. As a job seeker, I want to record certifications, so that I can present relevant credentials.
14. As a job seeker, I want to record language capabilities, so that I can describe communication skills.
15. As a job seeker, I want to add, remove, and reorder supported entries and bullets using explicit controls, so that I can organize my CV without a rich-text editor.
16. As a job seeker, I want to save incomplete drafts without mandatory work history, so that I can build my CV gradually as a fresh graduate or experienced applicant.
17. As a job seeker, I want to omit empty sections and completely blank entries from the PDF, so that my CV contains no empty headings or placeholder rows.
18. As a job seeker, I want to receive clear export validation for a missing name or substantive non-contact content, so that I can fix the minimum information needed for export.
19. As a job seeker, I want to use month/year, year-only, omitted dates, and Present without inferred months, so that my CV accurately reflects the precision of my records.
20. As a job seeker, I want to see ongoing and recent histories ordered consistently while preserving manual ties, so that chronology remains useful without invented dates.
21. As a job seeker, I want to choose reverse chronological content structure, so that my recent employment history leads the CV.
22. As a job seeker, I want to choose functional content structure, so that skill achievements lead and employment remains compact.
23. As a job seeker, I want to choose combination content structure, so that skill achievements and detailed employment history both receive emphasis.
24. As a job seeker, I want to switch content structures without deleting hidden content or re-entering information, so that I can compare structures safely.
25. As a job seeker, I want to receive a warning rather than a new hard export requirement when skill-group achievements are missing, so that I can still export available functional or combination content.
26. As a job seeker, I want to duplicate a CV into an independent CV copy, so that tailoring one application never changes another.
27. As a job seeker, I want to select Indonesian or English as the content language and standard heading language, so that my CV fits the application language.
28. As a job seeker, I want to switch the interface between Indonesian and English independently of content language, so that I can use my preferred interface without changing my CV.
29. As a job seeker, I want to duplicate and manually translate a CV without synchronization, so that each language version stays under my control.
30. As a job seeker, I want to preview the actual generated PDF, so that preview pagination represents the document I will export.
31. As a job seeker, I want to see outdated previews identified and late rendering results ignored, so that older content never masquerades as my latest edits.
32. As a job seeker, I want to export the current revision using the previewed PDF artifact, so that the exported document matches the current preview.
33. As a job seeker, I want to keep a visibly outdated preview after a rendering error without exporting it as current, so that I can recover without unknowingly submitting old content.
34. As a job seeker, I want to export A4 PDFs with automatic pagination and heading placement, so that long CVs remain readable without a forced one-page limit.
35. As a job seeker, I want to preserve long text, long words, and links without clipping or silent shrinking, so that none of my authored content disappears to fit a layout.
36. As a job seeker, I want to select and extract PDF text in the intended reading order, so that my career information remains machine-readable.
37. As a job seeker, I want to use a conservative single-column presentation with body contact details, so that I can avoid common parsing obstacles without a false ATS acceptance guarantee.
38. As a job seeker, I want to have edits autosaved after one second without further edits, so that I do not have to save each field manually.
39. As a job seeker, I want to see accurate Saving, Saved, and Save failed states, so that I know whether edits reached persistent storage.
40. As a job seeker, I want to retain unsaved edits after a failed save and retry or export a backup, so that storage errors do not immediately destroy my work.
41. As a job seeker, I want to save pending changes on normal close and choose a safe response when saving fails, so that closing the application does not silently discard unsaved edits.
42. As a job seeker, I want to recover a corrupted CV from one rolling previous valid revision with a notice, so that I can recover saved work without confusing recovery with full version history.
43. As a job seeker, I want to export an editable portable project backup, so that I can keep a snapshot outside the application library.
44. As a job seeker, I want to import every project backup as an independent CV, including repeated imports, so that import cannot silently replace an existing project.
45. As a job seeker, I want to have malformed, oversized, or invalid-schema backups rejected without library changes, so that unsafe files cannot partially modify my projects.
46. As a job seeker, I want to receive an explicit rejection for an unsupported newer project format, so that opening an incompatible backup cannot silently lose information.
47. As a job seeker, I want to have authored and imported content treated as plain data, so that text cannot execute template code or read arbitrary local files.
48. As a job seeker, I want to confirm destination overwrites and avoid partially written PDF or backup files, so that export failures do not corrupt an existing destination.
49. As a job seeker, I want to move unwanted CVs into local Trash, so that ordinary deletion remains reversible.
50. As a job seeker, I want to restore a CV from Trash, so that I can correct an accidental deletion.
51. As a job seeker, I want to confirm permanent deletion of a CV and its internal recovery copy without touching exported backups, so that I understand what deletion removes and what it cannot remove.
52. As a keyboard user, I want to use labelled forms, visible focus, accessible errors, and reorder buttons, so that I can complete core workflows without a mouse or drag-and-drop.
53. As a job seeker, I want to switch between form and preview tabs on narrow windows, so that I can work without relying on a wide side-by-side layout.
54. As a job seeker, I want to use locally bundled resources without telemetry or career content in logs, so that my CV data stays out of remote services and application diagnostics.
55. As a job seeker, I want to understand that local projects, recovery copies, Trash, and exported backups are unencrypted, so that I can choose appropriate device and backup protection.
56. As a job seeker, I want to know that first installation may download OS/runtime dependencies, so that I can distinguish offline daily operation from air-gapped installation.
57. As a job seeker, I want to receive clear guidance about unsigned test packages without bypassing OS protections, so that I can evaluate a local build without mistaking it for a signed public release.
58. As a job seeker, I want to receive errors for clearly contradictory date ranges and malformed supplied email addresses or web links, so that I can correct known-invalid information before exporting.
59. As a job seeker, I want to have only one application instance and one active CV editor, so that simultaneous application windows do not overwrite each other’s edits.

## Implementation Decisions

- **Ownership:** One project contains one independently editable CV. A CV copy has a new independent identity; no shared master profile, linked variants, or synchronized translations exist.
- **Application boundaries:** Tauri and Rust provide the desktop application. React, TypeScript, and Vite provide structured forms and preview. Rust owns authoritative validation, local file access, persistence, and export orchestration through narrowly scoped Tauri commands. No external HTTP API is required.
- **Initial modules:** A local library and editor, shared CV validation and content projections, local persistence/recovery and project transfer, PDF generation/preview, and platform packaging. These are behavioral responsibilities, not requirements for one class, service, or abstraction per responsibility.
- **Minimal dependencies:** No router, global state library, database, generic repository layer, or future integration scaffolding initially. Establish one application instance and one active CV editor.
- **Content:** Support contact details, professional summary, work experience, education, skills, projects, certifications, and languages. Use plain text and achievement bullets; all career claims are user-authored.
- **Draft/export distinction:** Save incomplete drafts. Omit empty sections and completely blank entries. Export requires a name and substantive content in at least one non-contact section. Optional missing information produces warnings. Clearly contradictory date ranges and malformed supplied email/web values block export; missing skill-group achievements produce a warning, not another hard requirement. Do not invent field values.
- **Dates and ordering:** Preserve month/year, year-only, omitted dates, and Present. Reject only date ranges provably contradictory at entered precision. Ongoing entries lead, then known start dates newest first, then undated entries. Preserve manual order for ties and indeterminate precision. Other lists and bullets have explicit reorder controls.
- **Structures:** Reverse chronological shows detailed employment with supporting skills; functional leads with skill-group achievements and a compact employer/title/date history; combination retains leading skill-group achievements plus detailed employment history. Structure changes never delete or require re-entry of hidden content.
- **Language:** Each CV has one content language, Indonesian or English, including standard headings. Interface language is independently selectable between those languages. Duplicate and manually translate for another language version; changing interface language does not mutate the CV.
- **Layout:** One fixed conservative presentation, single column, standard headings, selectable body text, and body contact information. No photos, icons, tables, or skill-rating bars in exported CVs. A4 automatic pagination has no arbitrary one-page limit, truncation, or silent text shrinking. Keep headings with subsequent content when possible and allow long content to span pages.
- **PDF engine:** Embed Typst through Rust using fixed application-controlled templates and bundled licensed fonts. Treat all CV content as data, never executable Typst input; restrict compiler resources rather than expose arbitrary local files or downloadable packages. Verify third-party notices and font redistribution before bundling.
- **Preview/export artifact:** Bundle PDF.js, its matching worker, and required supporting assets locally. Refresh after a short editing pause, explicitly identify outdated content, and discard late results from older revisions. Preview the actual generated PDF bytes. Reuse those bytes for export when they match the current revision; otherwise generate and preview the current revision first. On rendering failure, an older preview can remain visible as outdated but cannot be silently exported as current.
- **Persistence:** Use an application-managed local library, one versioned JSON file per project, and atomic replacement. There is no existing schema or database migration. Define and document the initial file schema during implementation, validate it in Rust, and reject unsupported future versions without partial import.
- **Autosave:** Save after one second without edits, with accurate Saving / Saved / Save failed indicators. Keep one rolling previous valid revision, not full history. A corrupted file requires an explicit recovery notice and must not be silently replaced. A crash can lose edits not yet persisted; zero data loss is not promised.
- **Close and failures:** Flush pending edits on normal close. If saving fails, retain edits in memory and offer Retry / Export backup / Discard and close / Cancel. Do not treat attempted writes as successful persistence.
- **Project transfer:** Export an independent editable snapshot. Import always creates a new independent CV, even on repeated import. Validate file size, schema, and supported version before mutation. Use internal validated identities for storage; imported identifiers never become unrestricted paths. Numeric resource limits are implementation parameters to document and boundary-test, not settled product limits.
- **Deletion:** Delete moves a project to local Trash, with Restore. Explicit confirmation is required for permanent deletion of the project and its internal recovery copy. No automatic Trash cleanup. Exported backups outside the library remain untouched. Deletion is not secure erasure.
- **Destination safety:** PDF and backup export confirm overwrites and preserve existing destination files on failed writes; do not leave a partial output masquerading as successful export. Saving, backup export, and PDF export remain distinct operations.
- **Privacy:** Projects, recovery copies, Trash, and exported backups are unencrypted and rely on OS account permissions. No telemetry, automatic uploads, remote fonts/assets, or career content in logs. No application password or encryption in this release.
- **Interaction/accessibility:** Provide a library and editor, side-by-side forms/preview on wide windows and switchable tabs on narrow windows. Require labelled controls, keyboard operation, visible focus, accessible error messages, and reorder buttons. Drag-and-drop is not required.
- **Platform scope:** Required release targets are Windows 11 x64 and Ubuntu 24.04 LTS x64. Arch/Omarchy is an additional smoke-test target, not blanket rolling-release compatibility. Build/test Windows natively and Linux against the Ubuntu baseline.
- **Packaging/distribution:** Produce an NSIS setup executable, a Debian package, and an AppImage for the additional Linux smoke test. First installation may download OS/runtime dependencies; offline daily operation does not promise air-gapped installation. Initial outputs are locally installable test packages, not a public signed release. Explain unsigned-build warnings without bypassing OS protections.
- **Delivery gate:** First implement only a small rendering integration check once implementation is authorized. Prove the selected rendering/viewer/font combination before building the full editor; failed evidence requires revisiting the choice, not silently dropping requirements. Dependency/toolchain versions and the font family remain verification-driven implementation selections, not guessed commitments.

## Testing Decisions

- **Review required:** Confirm the proposed test seam and ticket slicing before marking local drafts ready-for-agent. Product acceptance requirements were approved during design; the exact test seam is proposed here.
- **Primary seam:** Exercise observable behavior through the running desktop application: edit a project, preview/export it, restart/reopen it, and inspect the resulting PDF or project backup. This is the highest application seam and covers UI, Tauri commands, Rust behavior, persistence, and rendering together. Do not count a browser-only mock UI or mocked Rust response as desktop integration evidence.
- **Good tests:** Assert user-visible state and durable artifacts, not component structure, private helpers, exact method calls, internal directory layouts, incidental PDF metadata, or snapshots of implementation details. Preview/export identity is specifically a byte-artifact contract for the same current revision, not a requirement for independent renders to be byte-deterministic.
- **Supplemental failure boundary:** Use the smallest test-only fault control necessary at Rust persistence/rendering entry points to reproduce denied writes, interrupted writes, malformed files, and delayed/out-of-order renders. Assert the same public errors, preserved data, and export freshness. This is not a request for production-wide dependency injection or a second application architecture.
- **Coverage responsibilities:** Verify library/editor workflows, validation and structure projections, persistence/recovery, backup import/export, deletion, language independence, PDF artifacts, keyboard access, and packaged application behavior through the primary seam. Add focused pure checks only where date precision, ordering, and malformed input have edge cases that are clearer below the UI.
- **PDF evidence:** Inspect generated documents visually and extract text in intended reading order. Cover all three structures and both content languages, fresh-graduate and experienced CVs, empty sections, long words/URLs, multipage entries, and headings near page boundaries. Verify selectable text, no loss/clipping, bundled fonts, and actual preview/export artifact identity. Correct PDF extraction is not proof of universal ATS acceptance.
- **Safety evidence:** Cover independent duplicate and repeated import, rejection of invalid/unsupported/oversized files without library mutation, literal handling of markup-looking input, path traversal rejection, rolling recovery and its notice, save-failure retention, close choices, overwrite cancellation/failure, late preview rejection, and Trash/Restore/permanent deletion without touching external backups.
- **Offline/accessibility evidence:** Disable network after runtime installation and exercise editing, local resources, preview, backup, and PDF export. Check both interface languages without CV mutation, narrow-window tabs, keyboard-only controls, focus, labels, errors, and non-drag reordering. Avoid logging real career data; synthetic test content must be clearly identified as fixtures.
- **Platform evidence:** Build/install and exercise the supported workflows on native Windows 11 x64 and Ubuntu 24.04 LTS x64, then run the additional Arch/Omarchy AppImage smoke test. Missing environments are release blockers, not skipped checks reported as passes. A Linux build is not Windows runtime verification.
- **Prior art:** The current project contains design documents and ADRs only. There are no application routes, producer groups, test suites, executable seams, or established test conventions to reuse. No code prefactor is needed. The rendering check establishes the first executable seam; choose tooling only when implementation is authorized.

## Out of Scope

- Recruiter workflows or managing other people's CVs as an organization.
- Accounts, backend services, cloud sync, online collaboration, AI content generation, and automatic translation.
- Shared master profiles, linked CV copies, or synchronized language versions.
- DOCX, Letter paper, decorative templates, photos, columns, icons, tables, skill bars, rich-text editing, or free-positioned design tools.
- Mandatory work history, phone number, or street address; invented career facts or inferred missing dates.
- A database, full revision history, automatic Trash cleanup, application passwords, encryption, or secure-erasure guarantees.
- Guaranteed ATS acceptance, ATS certification, or an ATS scoring service.
- Windows 10, ARM, macOS, universal Linux support, and air-gapped first installation.
- Automatic updates, public publication, store submission, certificate purchases, and bypassing OS security controls.
- Implementation, dependency installation, Git initialization, commits, remote issue creation, and external publication under this documentation-only request.

## Further Notes

The user explicitly confirmed the design only. The current request adds a local specification and draft tracer-bullet ticket breakdown; it does not authorize executing those tickets. Local drafts need approval of granularity, dependency edges, and the proposed test seam before applying ready-for-agent status. No remote tracker or global tracker configuration is changed.

Accepted architecture is not verified capability. Native test environments, font license/coverage, a compatible dependency set, PDF.js worker loading in both WebViews, extraction correctness, long-content handling, responsiveness, and installer behavior remain execution prerequisites or verification gates. No renderer, build, installer, or runtime test has run. The previously attempted standalone research report was not saved because edit approval was denied; this spec does not claim that report exists.

The glossary and accepted design/ADRs govern interpretation. Source documents: [glossary](../../CONTEXT.md), [design](../../docs/design.md), [offline ADR](../../docs/adr/0001-offline-desktop-boundary.md), [local storage ADR](../../docs/adr/0002-local-project-files.md), and [PDF preview ADR](../../docs/adr/0003-preview-generated-pdf.md).
