# 03 — Edit all career sections with precise dates and validation

**What to build:** A job seeker completes education, skills, projects, certifications, and languages alongside employment, manages entries and bullets, and exports a validated reverse chronological CV without invented date precision.

**Blocked by:** 02 — Create, edit, autosave, and reopen a basic CV.

**Status:** draft — awaiting approval of granularity, blocking edges, and the test seam.

**Proposed triage:** ready-for-agent after approval. Execution still requires explicit implementation authorization.

**Story coverage:** US-10, US-11, US-12, US-13, US-14, US-15, US-16, US-17, US-18, US-19, US-20, US-58.

- [ ] Add structured education, supporting skills, projects, certifications, and languages to the existing editor, durable project data, reopening workflow, and PDF output.
- [ ] Provide explicit add/remove/reorder buttons for relevant entries and bullets with accessible labels; preserve saved order for non-chronological lists.
- [ ] Preserve month/year, year-only, omitted dates, and Present for ongoing roles across editing, save/reopen, and PDF output. Never materialize a missing month.
- [ ] Display ongoing histories first, then newest known start dates, then undated histories. Preserve manual order for ties or date precision that cannot establish order; avoid an inconsistent date comparator.
- [ ] Save partial entries as drafts. Ignore completely blank entries during export, warn on missing optional content, and require only the accepted name plus substantive non-contact content baseline.
- [ ] Block export for provably contradictory date ranges and malformed supplied email or web-link values. Validation is authoritative in Rust and errors are accessible in the UI.
- [ ] Demonstrate a fresh-graduate CV using education/projects without employment and an experienced multipage CV with all sections. Verify selected text reaches the PDF and survives restart.
- [ ] Add focused edge checks for mixed date precision, ongoing roles, equal dates, missing dates, and invalid provided contact/link values, while retaining application-level acceptance evidence.
