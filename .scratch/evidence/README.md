# Evidence — ticket 01 rendering gate

Synthetic fixtures only. No real career data was entered, logged, or exported.

## Environment actually used

| Item | Value |
| --- | --- |
| OS | Omarchy (Arch-based), kernel 7.2.15-1-omarchy |
| Rust | 1.99.0 (2026-09-28) |
| Node | 26.12.0, npm 12.2.1 |
| WebKitGTK | 2.54.1, libsoup 3.8.0 |
| Typst | 0.15.1 (`typst`, `typst-pdf`, `typst-layout` crates) |
| Tauri | 2.12.4 (`@tauri-apps/cli`, `@tauri-apps/api`), plugin-dialog 2.8.4 |
| PDF.js | pdfjs-dist 6.5.438 (bundled `pdf.mjs` + `pdf.worker.min.mjs`, no CDN) |
| Fonts | Liberation Serif Regular/Bold/Italic, copied from `ttf-liberation`; SIL OFL 1.1 text kept at `src-tauri/fonts/LICENSE-LiberationSerif.txt` |

## Commands

```bash
npm install
cargo test --manifest-path src-tauri/Cargo.toml     # 5 checks, all pass
npx tauri dev                                       # dev run
npx tauri build --debug --no-bundle                 # bundled-asset build
DISPLAY=:99 ./src-tauri/target/debug/cv-generator    # headless X run used for screenshots
```

## Engine checks (`cargo test`)

- `renders_a4_multipage_pdf` — first page is 595.276 × 841.89 pt (A4), long fixture yields >1 page, output starts with `%PDF`.
- `template_looking_text_stays_literal` — `#eval(...)`, `@markup`, `*bold*` in user data render as literal characters; no template execution.
- `export_writes_exact_cached_bytes` — exported file equals rendered bytes; existing destination replaced only after a full temp write.
- `different_content_renders_different_documents` — catches renderer cache reuse across different CV data (a real bug found during this gate).
- `writes_evidence_pdfs` — writes `short.pdf`, `multipage.pdf`, `longtoken.pdf` here.

## PDF observations (external tools)

`pdfinfo` / `pdftotext` on the evidence PDFs:

- `Page size: 595.276 x 841.89 pts (A4)`, `Creator: Typst 0.15.1`, multipage fixture = 3 pages.
- Text extracts in reading order: name → contact line → PROFESSIONAL SUMMARY → WORK EXPERIENCE → companies/titles/dates → bullets.
- No line crosses the right text margin: `pdftotext -bbox` max `xMax` 510 pt against a 533 pt margin (0 overflowing lines across all fixtures). Overlong tokens wrap at 24-cluster break points instead of running off the page.
- Long URLs and unbroken tokens stay extractable and in order; they wrap at line ends, so extraction inserts a line break there, as for any wrapped text.
- Page 1 ends mid bullet list, page 2 continues it; no heading is left alone at a page foot.

## Application path (real UI, headless X)

Screenshots in this directory: `app-preview.png`, `app-multipage.png`, `app-export-dialog.png`, `app-exported.png`, `export-status2.png` (dev run), `offline-*.png` (network-isolated run).

1. Launch → `Render: ready`, `Preview: 1 page(s), revision 2`.
2. Switch to the multipage fixture → `Preview: 2 page(s), revision 3`, preview shows real A4 pages rendered by bundled PDF.js from Rust-produced bytes.
3. Tab to `Export current PDF…` (visible blue focus ring), Enter → native GTK save dialog with `Synthetic Test CV — Multipage.pdf` suggested.
4. Typed `/tmp/cv-export-test.pdf`, Enter → file written, 36105 bytes.
5. App status: `Preview SHA-256` = `Export SHA-256` = `90c083f555149a9c3a234a208f984c5fba2412c08c7dd92d5b944b4edb88f2ec`, labelled **identical bytes to preview**. Local `sha256sum` of the exported file matches.

## Offline run (no network)

`npx tauri build --debug --no-bundle` produced a binary with the frontend embedded (`dist/`). It was started inside an isolated network namespace:

```bash
unshare -rn env DISPLAY=:99 GDK_BACKEND=x11 WEBKIT_DISABLE_COMPOSITING_MODE=1 \
  LIBGL_ALWAYS_SOFTWARE=1 ./src-tauri/target/debug/cv-generator
```

Inside that namespace only loopback exists and it is down (`curl https://example.com` → 000). Screenshots `offline-start.png`, `offline-exported.png`, `offline-status.png`.

- Multipage fixture → `2 page(s)`, export to `/tmp/cv-offline-final.pdf` → `36061 bytes … identical bytes to preview`.
- Preview, export, and `sha256sum` of the file all read `c82eefd0a0ef61b731e77470ce1495c2b2528c94953986cba836e24b2e50b79e`; the same hash came from the non-isolated run, so rendering is deterministic and needs no network.
- (Superseded hashes from the pre-review template: `90c083f5…2ec`.)

## Post-review fixes

Applied after the two-axis review: monotonic render-revision guard, `.pdf` destination check, `not_found()` dedupe, real injection test (rendered-text extraction + world file rejection), PDF.js loading-task `destroy()`, heading-only `sticky`, link labels rendered, evidence notes un-ignored, packaging `targets` removed.

## Residual gaps

- Page-boundary heading protection is enforced by `sticky` on the heading block, but no fixture can reach a page bottom with a heading while only two sections exist; a real boundary fixture belongs with ticket 03. A test that passed with `sticky` removed was deleted rather than kept as false assurance.

## Limitations / not yet met

- The isolated run still logs GTK `GVFS-WARNING` lines: the desktop file-portal/DBus attempts fail and fall back to the session bus over its unix socket. That is local IPC, not network; rendering and export completed anyway. A fully air-gapped claim would still need the portal question answered.
- Only Arch/Omarchy was exercised. Ticket 01's native Windows 11 x64 and Ubuntu 24.04 LTS runs are **not** done, so the gate is not complete.
- `tauri.conf.json` has `csp: null` and dev runs use the Vite server; hardening CSP belongs with the editor/packaging tickets.
- Export replaces an existing file via temp write + rename; on Windows `rename` over an existing file needs the pre-delete path in a later ticket.
- Wrapping uses fixed 24-cluster chunks; no hyphenation dictionaries are bundled.
- Screenshots were captured under Xvfb with a ctypes XTEST helper (`xtest_ui.py`) because `xdotool` is unavailable on this host.
