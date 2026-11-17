# Evidence — ticket 06 portable project backups

Synthetic fixtures only. The app ran against an isolated `XDG_DATA_HOME`
(`/home/airzy/.hermes/cache/scratch/cvgen-ticket06/data`), never the real library.

## Environment

Omarchy (Arch-based), Rust, Node, Tauri 2, Xvfb `:99`, local XTEST driver
[`drive.py`](drive.py) (ticket 05's, seeded with one Combination / Indonesian CV
`t06source`) plus [`click_text.py`](click_text.py) (OCR click).
The debug binary embeds the built frontend
(`npx tauri build --debug --no-bundle`); native GTK save/open dialogs are real.
Screenshots are gitignored; the commands and observed values are recorded here.

## Code checks

```bash
cargo test --manifest-path src-tauri/Cargo.toml     # 51 checks, all pass (44 before + 7 new)
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets   # no warnings
npm run build                                       # tsc --noEmit + vite build
npx tauri build --debug --no-bundle                 # succeeds
```

Ticket-06 checks: `export_then_import_twice_gives_independent_copies_with_all_content`,
`unsafe_backups_are_rejected_and_leave_the_library_untouched` (16 shapes × both
interface languages, library byte-compared after each),
`a_newer_backup_format_is_refused_by_name_whatever_its_shape`,
`export_refuses_a_document_import_would_refuse`,
`backup_size_limit_is_exact_for_import_and_export`,
`markup_looking_text_imports_verbatim_into_the_library_only`,
`failed_or_misdirected_backup_writes_keep_the_destination`.

Resource limit chosen: **2 MiB** (`MAX_BACKUP_BYTES = 2,097,152`). A CV is kilobytes
of text (the synthetic backup is 1,713 bytes); the cap bounds read and parse work.
Boundary: a valid file padded to exactly 2,097,152 bytes imports; 2,097,153 is refused.

## Application run (isolated library)

1. Opened `Backup source` → form bottom shows `Export project backup…` and the note
   *“A project backup is an editable snapshot, not a PDF. It is unencrypted…”*
   (`02-editor-actions.png`); library shows `Import project backup…` and the same note
   (`live.png`).
2. `Export project backup…` → native save dialog, default `Synthetic Candidate-backup.json`,
   filter `CV Generator backup` (`03-export-dialog.png`). Saved to
   `files/source-backup.json`: status `Backup saved: 1713 bytes → …/source-backup.json`
   (`04-exported.png`). File keys: `format, version, title, structure, content_language, cv`;
   no `id`, no `updated_at`.
3. Exported to the same path again: GTK asked *“A file named “source-backup.json” already
   exists. Do you want to replace it?”* (`05-overwrite-confirm.png`); `Cancel` then
   `Cancel` left the file's sha256 and mtime unchanged and showed no “Backup saved” line
   (`06-overwrite-cancelled.png`).
4. Library → `Import project backup…` (open dialog, filter `CV Generator backup`) twice.
   Library went from 1 to 3 files: `t06source.json`, `18e8b17868ea4a4e0000000000000.json`,
   `18e8b17bfd81e8cd0000000000001.json`. The second import opened with the full content
   and Indonesian headings, Combination structure (`09-imported-second.png`).
5. Edited only the second copy's project name (`… second import edited`), waited for
   autosave, restarted the app. All three files reload; the `cv` object hashes identically
   across source and both imports; only the second title differs; the source file's sha256
   is `8937396029eb…` before and after (`10`, `11`).
6. Rejections through the real dialog, each with the red `role=alert` message in the
   library and the library staying at 3 files:
   malformed file → `This is not a valid CV Generator backup: expected ident at line 1 column 2`
   (`12-reject-malformed.png`); with the interface switched to Bahasa Indonesia,
   `version: 2` → `Cadangan ini memakai format versi 2; aplikasi ini membaca versi 1. Perbarui aplikasi untuk membukanya.`
   (`13-…`) and a 2,097,206-byte file → `Cadangan melebihi batas 2097152 byte.` (`14-…`).
   The open dialog's filter name was also localized (`Cadangan CV Generator`).

## Not proven here

- No native Windows/Ubuntu run (tickets 10/11). Linux GTK dialogs only.
- Path-traversal, wrong-type, bad-month, unknown-field (`id`, `path`, `updated_at`)
  rejections and the write-failure/temp-file behaviour are proven at the Rust seam only,
  not clicked through the UI.
- A write failure mid-export was not forced in the UI; it is covered by
  `failed_or_misdirected_backup_writes_keep_the_destination`.
