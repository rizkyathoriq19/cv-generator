# Evidence — ticket 07 recovery, safe close, single instance

Synthetic fixtures only. The app ran against an isolated `XDG_DATA_HOME`
(`/home/airzy/.hermes/cache/scratch/cvgen-ticket07/data`), never the real library.

## Environment

Omarchy (Arch-based), Rust, Node, Tauri 2, Xvfb `:99`, local XTEST driver
[`drive.py`](drive.py) (ticket 06's, plus `close` — a real `WM_DELETE_WINDOW`
client message, the same request a window manager sends — and `windows`).
The debug binary embeds the built frontend (`npx tauri build --debug --no-bundle`).
No window manager is installed under Xvfb, so the close request is sent as the
protocol message rather than by clicking a title-bar button.
Screenshots are gitignored; commands and observed values are recorded here.

## Code checks

```bash
cargo test --manifest-path src-tauri/Cargo.toml                 # 66 checks, all pass (51 before + 15 new)
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets # no warnings
npm run build                                                   # tsc --noEmit + vite build
npx tauri build --debug --no-bundle                             # succeeds
```

Ticket-07 checks: `each_save_keeps_exactly_one_previous_valid_revision`,
`a_damaged_current_file_recovers_the_previous_revision_and_keeps_the_evidence`,
`a_damaged_project_with_no_valid_previous_revision_is_reported_and_left_alone`,
`a_denied_save_keeps_the_current_and_previous_revisions`,
`an_interrupted_replacement_leaves_the_last_saved_revision_readable`,
`a_corrupt_current_file_never_replaces_the_previous_valid_revision`,
`the_library_lists_a_damaged_project_so_it_can_be_recovered`,
`recovery_and_unreadable_notices_follow_the_interface_language`,
`one_running_instance_holds_the_library`,
`a_newer_project_file_is_refused_and_left_alone`,
`a_foreign_project_file_is_refused_without_rewriting`,
`damage_found_during_a_save_is_preserved`,
`preserving_damage_keeps_the_current_file_in_place`,
`a_save_reports_the_damage_it_preserved`,
`a_save_never_overwrites_a_refused_project_file`.

Fault controls are a `Faults { deny_write, interrupt_replace }` parameter used only
by checks; the running application always passes the default. Storage failures in
the application run were produced for real with `chmod 500` on the library folder.

Capability fix found during the run: `core:default` does not grant
`window.destroy`, so the close path needs `core:window:allow-destroy`. Without it
the window stayed open after a successful flush; with it, the same close request
flushes and closes.

## Application run (isolated library)

1. Opened `Recovery demo`, renamed it to `Recovery demo saved twice`, waited for
   autosave (`02-second-saved.png`). Library files then: `t07source.json`
   (1,736 bytes, new title, `updated_at=1794882350`) and `t07source.prev.json`
   (1,725 bytes, previous title, `updated_at=1763270400`).
2. Stopped the app and replaced `t07source.json` with
   `{"version": 1, "id": "t07source", "cv": {` (41 bytes). Restart: the row reads
   `Recovery demo · Damaged` in red (`03-damaged-library.png`).
3. Opening it recovered the previous revision and showed the notice
   (`04-recovered-editor.png`), verbatim:
   *“Your saved file was damaged, so the previous saved revision was recovered. This
   application keeps one previous revision, not full history. Review the CV after
   opening it. The damaged file was kept at
   …/projects/t07source.corrupt-1794882427265315398.json.”*
   Files after: `t07source.corrupt-1794882427265315398.json` still 41 bytes and
   unchanged, `t07source.json` restored byte-identically from the previous revision
   (1,725 bytes, same `updated_at`), `t07source.prev.json` unchanged. Reopening
   after a restart is an ordinary open (no notice).
4. `chmod 500` on the projects folder, edited the project name, waited 2.2 s:
   header shows `Save failed` (`05-save-failed.png`). `Library` then opened the
   close dialog (`06-close-dialog.png`): title *“Unsaved changes could not be
   saved”*, the four choices, and the detail
   `Save failed: writing the file failed: Permission denied (os error 13)`.
   `Cancel` returned to the editor with the edit still present (`07-cancelled.png`).
   `chmod 755`, `Library` → `Retry save` wrote the pending edit and returned to the
   library (`08-retry-saved-library.png`); the stored title contains the edit.
5. Window close with a failed save: `chmod 500`, edited, waited for `Save failed`,
   then sent `WM_DELETE_WINDOW`. The window stayed open with the same dialog
   (`09-save-failed-before-window-close.png`, `10-window-close-dialog.png`);
   `Cancel` kept the editor (`11-cancelled-close.png`). With writes allowed, the same
   close request flushed the edit and the window/process closed (`windows` list
   empty; the edit is in `t07source.json`).
6. `Discard and close`: reopened, `chmod 500`, typed ` DISCARDED`, waited for
   `Save failed`, sent `WM_DELETE_WINDOW` (`12-discard-dialog.png`), clicked
   `Discard and close`. The app exited and `t07source.json` is byte-identical to
   before that edit — only the explicit discard abandoned it.
7. Backup export from the close dialog while saving still failed: `chmod 500`, edit,
   `WM_DELETE_WINDOW`, `Export project backup…` → native save dialog with
   `Synthetic Candidate-backup.json` and filter `CV Generator backup`
   (`13-backup-save-dialog.png`) → `Save`. The dialog then shows
   `Backup saved: 1785 bytes → …/ticket07/Synthetic Candidate-backup.json` beside
   `Save failed: … Permission denied (os error 13)` (`14-backup-exported-dialog.png`);
   the working project stayed unsaved.
8. Indonesian interface: switched to `Bahasa Indonesia` (`15-library-indonesian.png`),
   corrupted the current file again, restarted. The row shows `… · Rusak`
   (`16-rusak-library.png`); opening shows the Indonesian notice
   (`17-indonesian-recovery-notice.png`):
   *“Berkas tersimpan Anda rusak, sehingga revisi tersimpan sebelumnya dipulihkan.
   Aplikasi ini hanya menyimpan satu revisi sebelumnya, bukan seluruh riwayat.
   Periksa kembali CV setelah dibuka. Berkas yang rusak disimpan di …”*.
   The close dialog is localized too (`18-dialog-indonesian.png`), including
   `Gagal menyimpan: writing the file failed: Permission denied (os error 13)`.
9. Backup-export failure inside that dialog: exported to a path inside the read-only
   projects folder. The dialog added the Indonesian write error
   `penulisan berkas gagal: Permission denied (os error 13)` while the save-failure
   line stayed visible (`19-backup-target-readonly.png`).
10. Single instance: with the app running, a second launch of the same binary exited
    `1` with `Another CV Generator window is already open. Close it before starting a
    second one: only one application instance may edit the library.` (stderr), and
    only one window remained.

## Not proven here

- No native Windows/Ubuntu run (tickets 10/11). Linux GTK dialogs only.
- The window close was requested as a `WM_DELETE_WINDOW` client message because no
  window manager is installed under Xvfb; the title-bar button itself was not clicked.
- A real crash between the two replacement steps is covered by the
  `interrupt_replace` fault control at the Rust seam; the running app was not killed
  mid-write.
- "One active CV editor" is structural (library and editor are mutually exclusive
  views) and backed by the single-instance lock; it has no separate UI assertion.
- Path-traversal, wrong-version, both-revisions-damaged and interrupted-replacement
  cases are proven at the Rust seam only, not clicked through the UI.
- Close choices (Retry / Export backup / Discard / Cancel) have no automated
  frontend check: this project has no frontend test harness, and the approved test
  seam is the running application, so they are demonstrated in steps 4–9 rather than
  asserted by a test.

## Review fixes re-check

The two-axis review raised four real defects, fixed before publication: a readable
but refused project file (newer version, foreign id) was being quarantined and
rolled back; recovery moved the current file away before writing, so a failed
recovery write could unlist the CV; damage found while the editor was open was
overwritten by the next save; and leaving the editor kept a stale error visible.

Re-run on the rebuilt binary (fresh isolated library):

- Recovery after the fix restored the previous revision byte-identically
  (`t07source.json` and `t07source.prev.json` both 1,725 bytes with the same
  `updated_at=1763270400`), kept the 41-byte damaged copy, and showed the Indonesian
  notice (`21-recovery-after-review-fix.png`).
- A save with the library folder read-only showed `Gagal menyimpan`; `Pustaka`
  opened the dialog (`22-dialog-before-escape.png`), `Escape` closed it with the edit
  and failed state intact (`23-dialog-escaped.png`), and `Retry save` after restoring
  permissions wrote `Recovery demo escape check` and returned to the library
  (`24-retry-library.png`; `t07source.json` 1,737 bytes, `t07source.prev.json` the
  recovered revision).
- New checks cover the refused newer file, the refused foreign file, save-time
  damage preservation, and copy-not-move preservation.

## Second review round re-check

The spec axis then raised two more save-path defects: a save still overwrote a
readable file this app refuses (a newer version), and save-time damage was preserved
but never reported. Both fixed: `write_project_in` refuses a refused current file and
returns the preserved path, and `save_project` returns a localized notice that the
editor shows beside `Saved`.

- `a_save_never_overwrites_a_refused_project_file` (version-2 file stays
  byte-identical, error names the version) and `a_save_reports_the_damage_it_preserved`
  (English + Indonesian notice naming the kept file) cover the fixes.
- App run on the rebuilt binary: with the CV open, the current file was replaced by
  `{"version": 1, "id": "t07source", "cv": {` and the project name edited. Autosave
  reported `Tersimpan` and showed the banner *“Berkas tersimpan rusak saat Anda
  mengedit. Editan Anda sudah disimpan, dan berkas yang rusak disimpan di
  …/t07source.corrupt-1794886797966265268.json.”* (`25-save-damage-notice.png`).
  Files after: the edit in `t07source.json` (1,755 bytes), a 41-byte preserved copy,
  and the untouched previous revision.

