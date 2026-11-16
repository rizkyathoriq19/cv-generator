# 06 — Export and safely import independent project backups

**What to build:** A job seeker exports a portable editable snapshot and imports it into a separate CV, while malformed or incompatible files cannot overwrite projects or partially mutate the local library.

**Blocked by:** 04 — Switch between all three structures without losing content.

**Status:** draft — awaiting approval of granularity, blocking edges, and the test seam.

**Proposed triage:** ready-for-agent after approval. Execution still requires explicit implementation authorization.

**Story coverage:** US-43, US-44, US-45, US-46, US-47, US-48, US-55.

- [ ] Export the current in-memory CV as a portable snapshot of all supported sections, skill groups, dates/precision, content language, and structure; explain that the backup is unencrypted and distinct from a PDF.
- [ ] Import always creates a new independent CV, including repeated imports of the same file and imports whose source identity matches an existing project. Verify existing projects remain unchanged.
- [ ] Validate supported version, schema, and bounded file size before mutating storage; reject malformed JSON, invalid data shapes, oversized files, and unsupported newer formats with accessible errors.
- [ ] Choose, document, and boundary-test explicit resource limits during implementation; the design does not supply an invented numeric product limit.
- [ ] Treat markup-looking strings as plain text and reject path traversal or imported path/identity values that attempt to escape application-controlled storage.
- [ ] Confirm destination overwrites; cancellation or write failure preserves an existing destination and does not leave a partial snapshot reported as complete.
- [ ] Demonstrate export from one project, import twice, edit/reopen the copies independently, and render their full content through the desktop application.
- [ ] Expose controls/errors through the shared interface-language mechanism when present; later localization acceptance must include this workflow.
