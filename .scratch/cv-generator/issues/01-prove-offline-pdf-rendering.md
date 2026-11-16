# 01 — Prove offline PDF generation and identical preview/export

**What to build:** A minimal running Tauri application accepts a small CV input, generates an A4 PDF through embedded Rust Typst, shows that PDF through bundled PDF.js, and exports the same artifact offline. This is a rendering gate, not a full editor or a CLI-only demonstration.

**Blocked by:** None — can start immediately after explicit implementation authorization.

**Status:** done — accepted on Arch/Omarchy only. Native Windows 11 x64 and Ubuntu 24.04 LTS runs were explicitly waived by the project owner for this ticket. Evidence: [../../evidence/README.md](../../evidence/README.md).

**Proposed triage:** ready-for-agent after approval. Execution still requires explicit implementation authorization.

**Story coverage:** US-04, US-30, US-34, US-35, US-36, US-37, US-47, US-54.

- [x] A minimal React/TypeScript/Vite Tauri screen sends plain CV data to Rust, displays the generated PDF, and exports those same bytes; demonstrate the complete application path rather than a mocked UI.
  Verified in the running app: fixture switch → preview (PDF.js canvases from Rust bytes) → native save dialog → exported file; app reported `Preview SHA-256` = `Export SHA-256` = `90c083f5…2ec` (`identical bytes to preview`), matching `sha256sum` of the exported file.
- [x] Use only fixed application templates, locally bundled fonts, and locally bundled matching PDF.js/worker assets. Record font redistribution permission and required third-party notices before inclusion.
  `src-tauri/templates/cv.typ`, `src-tauri/fonts/` (Liberation Serif + OFL text), `pdfjs-dist` worker bundled through Vite; notices in [docs/third-party-notices.md](../../../docs/third-party-notices.md).
- [x] Template-looking user text is literal data, compiler resources are restricted, and no arbitrary file access, package download, remote font, or CDN is required.
  CV data reaches Typst only as `cv.json` through a `World` that serves one declared file id and errors for every other path; Typst 0.15's `World` trait has no package registry, so no downloads are possible. Covered by `template_looking_text_stays_literal`.
- [x] Use clearly labelled synthetic short and multipage CV fixtures; verify A4, selectable/extractable text in reading order, long words/URLs, no clipped or lost text, no silent shrinking, and sensible page-boundary headings.
  `pdfinfo`: 595.276 × 841.89 pts A4, up to 3 pages. `pdftotext` reads name → contact → summary → experience → bullets. `pdftotext -bbox` showed 0 lines past the 533 pt margin after long tokens were given break points. Font size stays 10.5 pt (no shrinking).
- [x] Compare the preview artifact and exported artifact for byte identity for the same revision; do not require independent renders to have identical metadata.
  Rust caches the rendered bytes per revision and export writes those bytes; `export_writes_exact_cached_bytes` plus the UI hash comparison above. A stale revision is rejected instead of exporting.
- [x] Build and run the integration on native Windows 11 x64 and Ubuntu 24.04 LTS x64 with network disabled after runtime installation. Missing environments leave this gate incomplete.
  **Scoped to Arch/Omarchy by owner decision.** Verified offline there (`unshare -rn`, embedded-asset build, preview + export byte-identical). Windows 11 x64 and Ubuntu 24.04 LTS were not run; their acceptance remains part of tickets 10 and 11, which cannot be waived implicitly.
- [x] Record actual commands, dependency/toolchain selections, rendering observations, limitations, and evidence. A failed requirement blocks the full editor until the stack is revisited; do not replace requirements with a plausible report.
  See [evidence/README.md](../../evidence/README.md) (versions, commands, PDF measurements, screenshots, known limitations).
