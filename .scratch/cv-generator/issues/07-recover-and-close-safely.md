# 07 — Recover saved CVs and close safely after storage failures

**What to build:** A job seeker recovers saved work from a rolling previous revision, retains unsaved edits after storage errors, chooses how to close safely, and avoids concurrent application instances overwriting projects.

**Blocked by:** 06 — Export and safely import independent project backups.

**Status:** draft — awaiting approval of granularity, blocking edges, and the test seam.

**Proposed triage:** ready-for-agent after approval. Execution still requires explicit implementation authorization.

**Story coverage:** US-38, US-39, US-40, US-41, US-42, US-59.

- [ ] Retain exactly one rolling previous valid saved revision using a replacement sequence that preserves recoverability; do not promise full history or zero loss of not-yet-saved edits.
- [ ] Detect current-file corruption, preserve the corrupted evidence rather than silently overwrite it, and recover a valid previous revision with an explicit notice. If neither is valid, report the condition without fabricating or blanking the CV.
- [ ] On a failed save, keep current edits in memory, show a persistent Save failed state, and support retry or exporting the current project backup through the existing transfer workflow.
- [ ] Normal close immediately saves pending changes. On failure provide Retry / Export backup / Discard and close / Cancel; only explicit discard may abandon unsaved changes. Successful backup export and saving the working project remain distinct.
- [ ] Enforce one application instance and one active CV editor so normal use cannot race two writers against the same library.
- [ ] Use deterministic test-only failure controls to exercise denied writes, interrupted replacement, corrupt current/previous revisions, pending close, cancelled close, and backup-export failure; assert observable preservation and notices.
- [ ] Demonstrate recovery/restart and each close choice in the running application, with accurate save indicators and controls/errors covered by the shared interface languages.
