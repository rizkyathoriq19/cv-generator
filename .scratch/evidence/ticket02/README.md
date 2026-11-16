# Evidence — ticket 02 local library, autosave, reopen

Synthetic fixtures only. No real career data was entered, logged, or exported.
The application ran against an isolated `XDG_DATA_HOME`
(`/home/airzy/.hermes/cache/scratch/cvgen-e2e/data`), so the real local library
was never touched.

## Environment

Same host and toolchain as [ticket 01](../README.md): Omarchy (Arch-based), Rust
1.99.0, Node 26.12.0, Tauri 2.12.4, Typst 0.15.1, PDF.js 6.5.438. Runs used
`npx tauri build --debug --no-bundle` (frontend embedded) with Xvfb `:99` and
the local XTEST driver [`drive.py`](drive.py).

## Commands

```bash
cargo test --manifest-path src-tauri/Cargo.toml     # 14 checks, all pass
npm run build                                       # tsc --noEmit + vite build
npx tauri build --debug --no-bundle
python3 .scratch/evidence/ticket02/drive.py start   # Xvfb + app with isolated data dir
```

## Engine checks added by this slice

| Check | What it pins |
| --- | --- |
| `project_file_round_trips_with_its_schema_version` | one JSON file per project with `version: 1`, `structure: reverse-chronological`, `content_language: english`; reopening returns the same content; no temp files left behind |
| `autosave_replaces_the_existing_project_file` | a second save replaces the file in place (one file per project, latest content) |
| `unsupported_or_mismatched_project_files_are_refused` | a newer schema version is refused on read and never rewritten on write; a file whose `id` disagrees with its name is refused |
| `project_ids_never_escape_the_library_folder` | ids are validated before they become paths (`../escape`, `sub/dir`, `a.json` rejected) |
| `export_requires_a_name_and_a_non_contact_section` | whitespace/blank entries are not content; name plus a summary is enough |
| `blank_draft_entries_and_sections_do_not_render` | blank entries/bullets/summary produce no heading, row or "Present" |
| `a_missing_end_date_is_not_reported_as_present` | `end: ""` renders the start alone; only `end: null` renders Present |

## Application path (real UI, headless X)

Screenshots in this directory: `01-start` … `22-dialog-open`.

1. Empty library → `New CV` → editor. Project JSON appears immediately with
   `version: 1`, `structure: "reverse-chronological"`, `content_language: "english"`.
2. Filled project name, full name, email, location and summary by keyboard. Status
   badge went `Unsaved changes` → `Saving…` → `Saved`; the project JSON then held
   the typed values and a new `updated_at`.
3. Exported the fresh-graduate CV (no work-experience entry): native save dialog
   pre-filled `Synthetic Graduate.pdf`; export row read
   `22500 bytes → …/fresh-graduate.pdf — identical bytes to preview`, and the
   displayed SHA-256 matched `sha256sum` of the file. `pdfinfo`: 1 page,
   `595.276 x 841.89 pts (A4)`. `pdftotext`:
   `Synthetic Graduate` → `graduate@example.invalid · Testville` →
   `PROFESSIONAL SUMMARY` → summary text, with no empty work-experience heading.
4. Made the library directory read-only, then edited again: badge `Save failed`
   plus the Rust error text, the file on disk still held the previous content, and
   the new text stayed in the form. After `chmod u+w`, `Retry save` wrote it and
   the badge returned to `Saved`.
5. Restarted the app: the library listed `Fresh graduate CV` with the last-saved
   time; opening it restored every field and re-rendered the preview.
6. Added a role (title, employer, start `Jan 2024`, one bullet). Saved JSON:
   `{"title":"Information Systems Intern","company":"Synthetic Employer Ltd","start":"Jan 2024","end":"","bullets":["Synthetic internship achievement text."]}`.
   Preview and export showed `Information Systems Intern, Synthetic Employer Ltd (Jan 2024)`
   with no "Present", and the second export (`26404 bytes`, sha256 `4ea60c23…`)
   was again identical to the preview bytes.

## Limitations / not yet met

- The export rejection for an empty CV was verified at the Rust seam only
  (`export_requires_a_name_and_a_non_contact_section`). The identical UI path is
  proven for save failures (red alert with the Rust message), but the
  export-rejection message was not clicked through in the UI: the blind keyboard
  driving used here drifted between controls on an empty form.
- Contact links and the `structure` / `content_language` values are stored but have
  no editing control yet; switching them belongs to tickets 04 and 05.
- No rolling previous revision, corrupted-file recovery, or close-time choices yet —
  those are ticket 07, and `list_projects` currently skips unreadable project files.
- Only Arch/Omarchy was exercised. Native Windows 11 x64 and Ubuntu 24.04 LTS runs
  remain tickets 10 and 11.
- PDF export still uses `rename` over an existing destination; on Windows this can
  fail while another process holds the file. Project saves were verified on Linux.
