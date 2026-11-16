# 11 — Install and verify the Windows NSIS package natively

**What to build:** A Windows job seeker installs a local NSIS test package on Windows 11 x64 and completes the accepted workflows offline after runtime installation, with native verification rather than cross-build assumptions.

**Blocked by:** 05 — Duplicate CVs and separate interface/content languages; 08 — Move CVs to Trash, restore, and permanently delete safely; 09 — Keep PDF export current and the full editor keyboard-accessible.

**Status:** draft — awaiting approval of granularity, blocking edges, and the test seam.

**Proposed triage:** ready-for-agent after approval. Execution still requires explicit implementation authorization.

**Story coverage:** US-01, US-04, US-54, US-55, US-56, US-57.

- [ ] Build an NSIS setup executable on native Windows with the required C++/Rust/WebView2 toolchain and bundled templates/fonts/PDF.js assets plus required third-party notices.
- [ ] Install and launch on Windows 11 x64; record WebView2 handling and any first-install network requirement. Do not claim an embedded bootstrapper alone supports air-gapped installation.
- [ ] After prerequisites are installed, disable network and complete library editing, save/reopen, all structures/content languages, interface switching, current PDF preview/export, and backup transfer.
- [ ] Run the safety and accessibility acceptance matrix against the installed application, including failed/atomic replacement behavior on Windows, recovery, close choices, invalid import, Trash, rendering races, keyboard operation, and narrow layout.
- [ ] Extract and visually inspect generated fresh-graduate, complete, and multipage PDFs; verify correct reading order, no clipping/lost fonts, current preview/export identity, and no network resources or career content in logs.
- [ ] Document unsigned test-build warnings without bypassing OS protections. No certificate purchase, public upload, store publication, automatic updater, or security-control weakening is authorized.
- [ ] Attach real native build/install/test commands, artifact locations, environment/dependency/license records, and observed results. Missing Windows access or failed checks leaves the slice incomplete; a Linux build is not substitute evidence.
- [ ] Explain that local projects and exported backups are unencrypted and keep all artifacts local until separately authorized.
