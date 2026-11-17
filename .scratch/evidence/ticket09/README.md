# Evidence — ticket 09 fresh PDF export and keyboard/narrow-window workflow

Synthetic fixtures only. The app ran against an isolated `XDG_DATA_HOME`
(`/home/airzy/.hermes/cache/scratch/cvgen-ticket09/data`), never the real library.
Screenshots are gitignored; the exported PDFs were moved out of the repository to
`/home/airzy/.hermes/cache/scratch/cvgen-ticket09/out/` and only their sha256 is
quoted here. Commands and observed values are recorded below.

## Environment

Omarchy (Arch-based), Rust, Node 26.12.0, Tauri 2, Xvfb `:99` (1400×1000),
local XTEST driver [`drive.py`](drive.py) (ticket 02's helper plus `resize`,
`wheel` and a long-content `seed`). The debug binary embeds the built frontend
(`npx tauri build --debug --no-bundle`).

Seeded library: `t09fresh.json` (`Freshness demo`, 12 roles × 5 bullets,
4 skill groups, 2 rendered pages) and `t09other.json` (`Neighbour CV`).

## Code checks

```bash
cargo test --manifest-path src-tauri/Cargo.toml                 # 79 checks, all pass (74 before + 5 new)
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets # no issues
npm run build                                                   # tsc --noEmit + vite build
npm test                                                        # node --test src/*.test.ts, 3 checks
npx tauri build --debug --no-bundle                             # succeeds
```

Ticket-09 checks:

- `long_content_keeps_a4_pages_readable_order_and_headings_with_their_content` —
  every structure with long content: >1 page, every page 595.276 × 841.89 pt (A4),
  every authored marker still present in the laid-out page frames (no loss),
  skills lead in functional/combination and employment leads in reverse
  chronological, and no page ends with a bare section heading.
- `a_late_render_result_cannot_replace_a_newer_preview` — an older revision
  arriving after a newer one is refused and the newer bytes and content stay
  cached, so export still matches them.
- `a_failed_render_keeps_the_previous_preview_and_blocks_exporting_newer_content` —
  a CV that cannot be rendered reports an error, leaves the previous cached
  preview bytes untouched, and the next export of content that never rendered is
  refused as out of date without creating the destination file.
- `a_failed_pdf_export_keeps_the_destination_and_writes_nothing_partial` —
  a destination that cannot be replaced reports an error, keeps the original
  content, leaves no temp file; a current preview replaces an existing file with
  exactly the previewed bytes; content that was never rendered is refused as
  out of date and the file is untouched.
- `cached_bytes_only_match_the_content_structure_and_language_they_were_rendered_from`
  (updated) — export identity is the rendered input, not the client render
  counter: a re-render of the same content at a newer revision still matches,
  while newer content, another structure or another content language never does.
- `writes_ticket09_structure_evidence_pdfs` — writes the three long PDFs below
  and asserts each is a PDF that spans pages.
- `src/freshness.test.ts` — `contentKey` changes on an edit, a structure switch
  and a content-language switch; `previewState` reports `current`/`outdated`/`none`;
  `canReusePreview` is false for stale content, an in-flight render, or nothing
  rendered, so export generates the current revision first.

## PDF observations (external tools)

`pdfinfo` on the ticket-09 evidence PDFs: `Page size: 595.276 x 841.89 pts (A4)`,
`Creator: Typst 0.15.1`.

| File | Pages | `pdftotext` order |
| --- | --- | --- |
| `long-reverse-chronological.pdf` | 4 | SUMMARY → WORK EXPERIENCE → EDUCATION → SKILLS → PROJECTS → CERTIFICATIONS → LANGUAGES |
| `long-functional.pdf` | 2 | SUMMARY → SKILLS → WORK EXPERIENCE (compact) → EDUCATION → PROJECTS → CERTIFICATIONS → LANGUAGES |
| `long-combination.pdf` | 4 | SUMMARY → SKILLS → WORK EXPERIENCE → EDUCATION → PROJECTS → CERTIFICATIONS → LANGUAGES |

## Application run (isolated library)

1. Library lists `Freshness demo` and `Neighbour CV`; opened the first
   (`01-wide-editor.png`). Wide window (1240×860) shows the form and the real
   PDF preview side by side; the state table reads `Preview 2 page(s), revision 2`.
2. Edited `Full name` from `Synthetic Candidate Zq9` to `Synthetic Candidate Zq9Q1`.
   The immediate screenshot (`33-outdated-banner.png`) shows the yellow banner
   *“This preview is not the current CV.”* above the **older** preview page,
   whose header still reads `Synthetic Candidate Zq9`; the save pill reads
   `Unsaved changes`. A few seconds later (`34-current-after-banner.png`) the
   banner is gone, the preview header reads `Synthetic Candidate Zq9Q1`, and the
   pill reads `Saved`.
3. `Export current PDF…` with a current preview (`export-current.pdf`):
   `sha256sum` = `7f50b98058a7149cf099c05b978aa9ae750a6ac7329d1d59bec25eaa464359f5`,
   identical to the `Preview SHA-256` in the state table
   (`17-exported-current.png`); `pdftotext` starts with
   `Synthetic Candidate Zq9` — the exported artifact is the current content.
4. Exported to the same path again: GTK asked
   *“A file named “export-current.pdf” already exists. Do you want to replace it?”*
   with `Cancel` / `Replace` (`23-overwrite-confirm.png`). `Cancel` left the file
   at mtime 1794898433, 43241 bytes, same sha256 (`24`, `25`).
5. Narrow window (780×900): the layout becomes one pane with `Edit CV` and
   `PDF preview` buttons. `Edit CV` is pressed and shows the form only
   (`27-narrow-tabs.png`); `PDF preview` shows the PDF canvases only
   (`28-narrow-preview.png`). No separate HTML preview is used.
6. Keyboard: with the `Edit CV` pane button focused, three `Tab` presses moved
   the visible focus outline to the project name field and then to the
   `Content structure` select, scrolling the form back to the top
   (`32-keyboard-tab-traversal.png`). Labels are visible on every control,
   reorder/remove are native buttons (`↑`/`↓`/`Remove` with accessible names),
   and validation/render/save errors render in `role="alert"`/`role="status"` boxes.

## Not proven here

- The stale-preview export branch was not clicked through. Reaching the Export
  button needs scrolling the form, which takes longer than the 400 ms render
  debounce, so the preview was current again by the time the button was pressed.
  The behaviour is proven at the seam: `canReusePreview` (freshness test) is
  false for stale content, and Rust refuses to export content that was never
  rendered (`a_failed_pdf_export_keeps_the_destination_and_writes_nothing_partial`).
- A failed PDF write through the real dialog (permission denied / full disk) was
  not forced in the UI; it is covered by the same Rust check. Overwrite refusal
  by `Cancel` was clicked through.
- Out-of-order (late) renders cannot be produced through the public UI; they are
  covered by `a_late_render_result_cannot_replace_a_newer_preview` and by the
  frontend inserting detached canvases only for the newest revision.
- A rendering failure (Typst/PDF.js error) has no public trigger: invalid dates
  cannot be entered through the selects. Its consequence — the previous preview
  bytes survive and the next export is refused without creating a file — is
  covered by `a_failed_render_keeps_the_previous_preview_and_blocks_exporting_newer_content`;
  the frontend's early return before the save dialog is by inspection only.
- Reordering and the library operations were not driven by keyboard end to end.
  They are native buttons and selects with accessible names, and were exercised
  by click in tickets 02–08; only focus order and labelling are keyboard-proven
  here (`32-keyboard-tab-traversal.png`).
- Glyph clipping and font shrinking are not measured directly; loss is inferred
  from the laid-out page frames plus `pdftotext` extraction of the written PDFs.
- No native Windows/Ubuntu run (tickets 10/11). Linux GTK dialogs only.
