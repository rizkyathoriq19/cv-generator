# 08 — Move CVs to Trash, restore, and permanently delete safely

**What to build:** A job seeker removes a CV from the active library without losing the ability to restore it, then explicitly removes its internal files when permanent deletion is intended.

**Blocked by:** 07 — Recover saved CVs and close safely after storage failures.

**Status:** draft — awaiting approval of granularity, blocking edges, and the test seam.

**Proposed triage:** ready-for-agent after approval. Execution still requires explicit implementation authorization.

**Story coverage:** US-49, US-50, US-51, US-55.

- [ ] Delete moves the project and its associated internal recovery state into local Trash; the active library no longer lists it and no automatic cleanup runs.
- [ ] Restore returns the independent CV and its data to the active library without overwriting another project or silently losing content; demonstrate reopen and PDF generation after restoration.
- [ ] Permanent deletion requires an explicit accessible confirmation and removes the selected project plus its internal recovery copy, not unrelated projects or exported backups outside the library.
- [ ] Flush or explicitly resolve pending edits before a move/delete action; late autosave completion must not resurrect a permanently deleted project.
- [ ] Explain that Trash and recovery data are unencrypted and that deletion is not secure erasure. Keep all controls/notices within the shared interface-language mechanism.
- [ ] Exercise delete/restore across restart, cancelled permanent deletion, successful internal cleanup, and untouched external backup fixtures through public application behavior.
