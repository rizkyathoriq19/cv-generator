# CV Generator — Local Spec and Draft Tickets

## State and scope

- Product design: confirmed by the user, design only.
- Specification: [local spec](spec.md).
- Tracker for this request: local files only; no remote issues or global tracker configuration changes.
- Tickets: draft proposals pending approval of granularity, genuine blocking edges, and the test seam. Apply ready-for-agent only after approval; execution still requires explicit implementation authorization.
- Current project: ticket 01 rendering gate and ticket 02 local library/autosave/reopen implemented (Tauri + Rust Typst + bundled PDF.js, Rust-owned project storage) and runtime-verified on Arch/Omarchy; no Git repository, installer, or native Windows/Ubuntu runs yet. See [ticket 01 evidence](evidence/README.md) and [ticket 02 evidence](evidence/ticket02/README.md).

## Proposed testing seam

Use the running Tauri application as the primary seam: edit a CV, generate/preview/export its PDF, restart/reopen saved data, and inspect durable artifacts. Add only narrow test-only failure controls at Rust persistence/rendering entry points for deterministic failures and out-of-order results; do not build a generic injection framework. Platform packages must be exercised natively, not inferred from mocked browser tests.

This seam is proposed for review, not already implemented or independently approved. The first rendering gate establishes the executable foundation before the full editor.

## Proposed vertical slices

1. **[Prove offline PDF generation and identical preview/export](issues/01-prove-offline-pdf-rendering.md)**
   - **Blocked by:** None.
   - **Delivers:** A minimal running Tauri application accepts a small CV input, generates an A4 PDF through embedded Rust Typst, shows that PDF through bundled PDF.js, and exports the same artifact offline. This is a rendering gate, not a full editor or a CLI-only demonstration.

2. **[Create, edit, autosave, and reopen a basic CV](issues/02-create-save-reopen-cv.md)**
   - **Blocked by:** 01.
   - **Delivers:** A job seeker creates a named CV project in a local library, edits contact details, a summary, and work experience, previews a reverse chronological CV, and reopens saved work after restarting the application.

3. **[Edit all career sections with precise dates and validation](issues/03-complete-career-content.md)**
   - **Blocked by:** 02.
   - **Delivers:** A job seeker completes education, skills, projects, certifications, and languages alongside employment, manages entries and bullets, and exports a validated reverse chronological CV without invented date precision.

4. **[Switch between all three structures without losing content](issues/04-switch-content-structures.md)**
   - **Blocked by:** 03.
   - **Delivers:** A job seeker authors skill-group achievements and switches one saved CV between reverse chronological, functional, and combination presentation without losing detailed employment or skill evidence.

5. **[Duplicate CVs and separate interface/content languages](issues/05-independent-copies-and-languages.md)**
   - **Blocked by:** 04.
   - **Delivers:** A job seeker duplicates a CV for another application or manual translation, selects Indonesian or English content headings, and changes interface language without mutating any CV.

6. **[Export and safely import independent project backups](issues/06-portable-project-backups.md)**
   - **Blocked by:** 04.
   - **Delivers:** A job seeker exports a portable editable snapshot and imports it into a separate CV, while malformed or incompatible files cannot overwrite projects or partially mutate the local library.

7. **[Recover saved CVs and close safely after storage failures](issues/07-recover-and-close-safely.md)**
   - **Blocked by:** 06.
   - **Delivers:** A job seeker recovers saved work from a rolling previous revision, retains unsaved edits after storage errors, chooses how to close safely, and avoids concurrent application instances overwriting projects.

8. **[Move CVs to Trash, restore, and permanently delete safely](issues/08-trash-and-restore-projects.md)**
   - **Blocked by:** 07.
   - **Delivers:** A job seeker removes a CV from the active library without losing the ability to restore it, then explicitly removes its internal files when permanent deletion is intended.

9. **[Keep PDF export current and the full editor keyboard-accessible](issues/09-fresh-accessible-pdf-workflow.md)**
   - **Blocked by:** 04.
   - **Delivers:** A job seeker edits rapidly, previews and exports only the current CV, handles rendering errors honestly, and completes the editor/preview workflow on narrow windows or without a mouse.

10. **[Install and verify the Ubuntu package and Arch/Omarchy AppImage](issues/10-verify-linux-packages.md)**
   - **Blocked by:** 05, 08, 09.
   - **Delivers:** A Linux job seeker installs a local test package and completes the accepted CV workflows offline, with real Ubuntu release evidence and an additional Arch/Omarchy AppImage smoke test.

11. **[Install and verify the Windows NSIS package natively](issues/11-verify-windows-package.md)**
   - **Blocked by:** 05, 08, 09.
   - **Delivers:** A Windows job seeker installs a local NSIS test package on Windows 11 x64 and completes the accepted workflows offline after runtime installation, with native verification rather than cross-build assumptions.

## Blocking rationale

- 01 gates the full editor because the selected renderer, font bundle, and viewer must work end to end before editor investment.
- 02 introduces the executable library/edit/save/reopen path; 03 extends it to all career sections and date rules; 04 adds the remaining structures and retained skill evidence.
- 05 needs complete content/structures for independent language copies and all headings. 06 needs complete project data to make backups round-trip every section. 09 needs all structures for freshness and PDF/accessibility acceptance. These three branches do not depend on one another.
- 07 needs 06 because its failed-close workflow offers export of the unsaved project backup; 08 needs the recovery lifecycle so permanent deletion removes the correct internal recovery state.
- 10 and 11 depend on 05, 08, and 09 to verify the complete accepted feature set, not just an early scaffold. Their remaining feature prerequisites are transitive. Linux and Windows package verification are independent branches.
- External environment availability gates actual execution: native Windows and Ubuntu are required for 01; Arch/Omarchy is additionally required for the Linux smoke test. No environment availability is assumed.

## Story traceability

The numbered stories in the spec are identified below as US-NN. Ticket coverage overlaps intentionally where an early tracer bullet is strengthened by a later complete workflow or release check.

| Ticket | Story coverage |
| --- | --- |
| 01 | US-04, US-30, US-34, US-35, US-36, US-37, US-47, US-54 |
| 02 | US-05, US-06, US-07, US-08, US-09, US-16, US-17, US-18, US-21, US-38, US-39 |
| 03 | US-10, US-11, US-12, US-13, US-14, US-15, US-16, US-17, US-18, US-19, US-20, US-58 |
| 04 | US-11, US-22, US-23, US-24, US-25 |
| 05 | US-26, US-27, US-28, US-29 |
| 06 | US-43, US-44, US-45, US-46, US-47, US-48, US-55 |
| 07 | US-38, US-39, US-40, US-41, US-42, US-59 |
| 08 | US-49, US-50, US-51, US-55 |
| 09 | US-30, US-31, US-32, US-33, US-34, US-35, US-36, US-37, US-48, US-52, US-53 |
| 10 | US-02, US-03, US-04, US-54, US-55, US-56 |
| 11 | US-01, US-04, US-54, US-55, US-56, US-57 |

## Completion and review

No ticket is implemented, complete, or runtime-verified. The local slice inventory covers the entire approved product; there are no existing routes/producer groups to migrate and no prefactor is required. There is no hidden downstream migration placeholder.

Before moving drafts to ready-for-agent, confirm:

1. The primary application test seam and narrow failure controls match expectations.
2. Ticket granularity is suitable for one fresh implementation context per slice.
3. Each blocking edge represents a real prerequisite; identify any desired merge or split.

Approving these documentation drafts does not itself authorize implementation, dependency installation, commits, or external publication. Both package branches and their actual acceptance evidence are required for release completion.
