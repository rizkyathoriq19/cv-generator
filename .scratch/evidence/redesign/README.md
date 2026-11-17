# Redesign evidence (UI + PDF + architecture split)

Arch/Omarchy, debug build (`npx tauri build --debug --no-bundle`), Xvfb :99, isolated
`XDG_DATA_HOME=~/.hermes/cache/scratch/cvgen-redesign`, two **seeded** synthetic CVs (`drive.py seed`).

- 01 library · 02 wide editor · 03 structure switched to Combination (persisted `structure: combination`)
- 04 month/year selects not clipped · 05 narrow bug found (both panes shown) → fixed `[hidden]`
- 06–08 narrow library / form pane / preview pane (760 px)
- 10–11 Export PDF through the GTK dialog → `Synthetic Candidate.pdf`, success banner (bytes match preview)
- `pdf-before.png` / `pdf-after.png`, `ui-before.png` / `02-editor.png`

Not re-verified here: Trash/restore, recovery, close-on-failed-save dialogs (code moved, logic unchanged;
Rust suite 79/79 and TS freshness tests cover their seams).

## v2 — ATS reference template (user-supplied Wozber CV, not copied into the repo)

- Font: Liberation Sans (same Liberation 2.1.5 family/OFL text as the old Serif; license renamed `LICENSE-Liberation.txt`).
- Optional `headline` under the name (schema v1, `serde(default)`, old files still load).
- Headings: uppercase, bold, dotted rule, **no letter-spacing** — 0.1em tracking made `pdftotext` emit
  `PRO FESSIO NA L S UM M A RY` (the reference PDF itself extracts as `E D U C AT I O N`).
- Entry: bold title + grey date right, grey employer/school second line; `n / N` page footer only when >1 page.
- Kept from the design contract, unlike the reference: no contact icons, no two-column header, no skill-level table.
- 12–15: library/editor with headline, export through GTK dialog → `Synthetic Candidate.pdf`, `pdf-after.png`.
