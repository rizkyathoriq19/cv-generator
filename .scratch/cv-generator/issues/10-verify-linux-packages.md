# 10 — Install and verify the Ubuntu package and Arch/Omarchy AppImage

**What to build:** A Linux job seeker installs a local test package and completes the accepted CV workflows offline, with real Ubuntu release evidence and an additional Arch/Omarchy AppImage smoke test.

**Blocked by:** 05 — Duplicate CVs and separate interface/content languages; 08 — Move CVs to Trash, restore, and permanently delete safely; 09 — Keep PDF export current and the full editor keyboard-accessible.

**Status:** draft — awaiting approval of granularity, blocking edges, and the test seam.

**Proposed triage:** ready-for-agent after approval. Execution still requires explicit implementation authorization.

**Story coverage:** US-02, US-03, US-04, US-54, US-55, US-56.

- [ ] Build a Debian package and AppImage against the accepted Ubuntu 24.04 LTS x64 baseline with bundled templates/fonts/PDF.js assets and required third-party notices.
- [ ] Install and launch the Debian package in a native Ubuntu 24.04 LTS x64 environment; document OS/runtime dependencies and any first-install network requirement without claiming air-gapped provisioning.
- [ ] After prerequisites are installed, disable network and complete library editing, save/reopen, all structures/content languages, interface switching, current PDF preview/export, and backup transfer.
- [ ] Run the safety and accessibility acceptance matrix against the packaged application, including recovery/save failures, invalid imports, Trash/Restore, current-revision races, keyboard operation, and narrow layout.
- [ ] Extract and visually inspect packaged-output PDFs for fresh-graduate, complete, and multipage cases; verify no missing fonts, lost text, remote requests, telemetry, or CV content in logs.
- [ ] Execute the AppImage on Arch/Omarchy as the additional smoke test, recording environment, startup, editing, save/reopen, and PDF preview/export results without promising universal Linux compatibility.
- [ ] Attach exact build/install/test commands, artifact locations, dependency/license records, and observed results. Missing test environments or failed checks leave the slice incomplete; do not mark unrun tests passed.
- [ ] Produce local test artifacts only, explain unencrypted local data, and perform no public upload, updater configuration, publication, commit, or signing purchase.
