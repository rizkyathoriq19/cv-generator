# Evidence — ticket 08 Trash, restore, explicit permanent deletion

Synthetic fixtures only. The app ran against an isolated `XDG_DATA_HOME`
(`/home/airzy/.hermes/cache/scratch/cvgen-ticket08/data`), never the real library.
Screenshots are gitignored; commands and observed values are recorded here.

## Environment

Omarchy (Arch-based), Rust, Node, Tauri 2, Xvfb `:99`, local XTEST driver
[`drive.py`](drive.py) (ticket 02's helper plus `seed`/`check` for two synthetic
CVs). The debug binary embeds the built frontend
(`npx tauri build --debug --no-bundle`).

Seeded library before the run:

```
projects/t08demo.json                        1716 bytes  current revision
projects/t08demo.prev.json                   1717 bytes  previous revision
projects/t08demo.corrupt-1763270450000000000.json 18 bytes  kept damaged copy
projects/t08keep.json                        1701 bytes  neighbouring CV
projects/t08keep.prev.json                   1701 bytes  its previous revision
```

`external-backup.json` (541 bytes, sha256
`ce5215a667e2b16a54cbcf167de84391c12b5c34b68298cc610eccad66787d8b`) stands in for
an exported project backup outside the library.

## Code checks

```bash
cargo test --manifest-path src-tauri/Cargo.toml                 # 74 checks, all pass (66 before + 8 new)
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets # no warnings
npm run build                                                   # tsc --noEmit + vite build
npx tauri build --debug --no-bundle                             # succeeds
```

Ticket-08 checks: `delete_moves_the_saved_file_previous_revision_and_damaged_copy_to_trash`,
`restore_returns_the_cv_and_its_recovery_state_and_still_renders`,
`a_damaged_cv_survives_trash_restore_and_recovery`,
`restore_refuses_to_overwrite_an_active_project`,
`permanent_deletion_removes_only_the_selected_cv_and_leaves_exported_backups`,
`a_failed_internal_cleanup_keeps_the_trash_row_retryable`,
`a_late_save_cannot_resurrect_a_deleted_or_permanently_deleted_cv`,
`trash_controls_and_refusals_follow_the_interface_language`.

The checks cover the move (saved file, previous revision, kept damaged copy; a
neighbouring id that is a prefix of the deleted id is not claimed), restore
(byte-identical files, leftover deletion marker cleared, reopen, PDF generation),
restore refusal with both sides byte-identical, permanent deletion of one CV while
another trashed CV and an external backup stay intact, retryable cleanup when one
internal file cannot be removed, and the Trash/permanent-deletion save guard. The
ticket-07 `a_save_never_overwrites_a_refused_project_file` check is retained.

## Application run (isolated library)

1. Library with two CVs: `Trash demo` and `Keep me`; toolbar shows `Trash (0)`.
   Clicked `Move to Trash` on `Trash demo` (`01-library-after-delete.png`).
   Active library then lists only `t08keep.json` and its previous revision; this is
   the "Delete" behaviour — a reversible move, not an erase.
2. Trash folder after the move, all three files unchanged in size:

   ```
   trash/t08demo.corrupt-1763270450000000000.json   18 bytes
   trash/t08demo.json                             1716 bytes
   trash/t08demo.prev.json                        1717 bytes
   ```

   Clicked `Trash (1)` (`02-trash-view.png`): the row reads `Trash demo`, the
   note says *“CVs in Trash and their recovery data are unencrypted plain files.
   Permanent deletion removes them from this device but is not secure erasure.
   Exported PDFs and project backups outside the library are not touched.”*, and
   the row offers `Restore` and `Delete permanently`.
3. Stopped and restarted the app; `Trash (1)` and the same row were still there
   (`03-trash-after-restart.png`). No automatic cleanup removed anything.
4. `Restore` moved all three files back (`04-trash-empty-after-restore.png` shows
   Trash (0)); the library listed both CVs again
   (`05-library-after-restore.png`). File sizes and bytes after restore were
   identical to step 1.
5. Opened the restored CV: it opened with no recovery notice and the actual PDF
   preview rendered the authored content (`06-restored-editor-preview.png`),
   proving reopen and PDF generation after restoration.
6. Deleted the CV again, then clicked `Delete permanently`:
   `07-permanent-confirm-dialog.png` shows the accessible confirmation —
   title *“Delete this CV permanently?”*, the body naming the saved file, the
   previous revision, and kept damaged files, the CV name `Trash demo`, and the
   buttons `Cancel` (focused) and `Delete permanently`. `Escape` cancelled it
   (`08-cancelled-permanent-delete.png`); the three Trash files were still present
   and byte-identical.
7. Confirmed permanent deletion (`09-permanent-deleted.png`). Trash then held only
   `t08demo.deleted.json` (0 bytes, the save guard marker); the active library
   still listed `t08keep.json` (1701 bytes) and the external backup was unchanged
   (541 bytes, same sha256). No other project or external file was touched.
8. Switched the interface to `Bahasa Indonesia`
   (`10-language-dropdown.png`). Trash view copy is localized
   (`11-indonesian-trash-empty.png`): *“CV di Sampah dan data pemulihannya adalah
   berkas biasa yang tidak dienkripsi…”*. Deleted `Keep me`
   (`12-indonesian-active-library.png`, `13-indonesian-trash.png`:
   `Pulihkan` / `Hapus permanen`).
9. `Hapus permanen` opened the Indonesian confirmation
   (`14-indonesian-confirm-dialog.png`): *“Hapus CV ini secara permanen?”* with
   `Batal` focused and `Hapus permanen`; `Batal` cancelled it. `Pulihkan`
   restored the CV and the Indonesian library listed it again with `Sampah (0)`
   (`15-indonesian-library-after-restore.png`).
10. Restarted once more: `Keep me` still listed, `Sampah (0)`, marker still the
    only Trash entry (`16-after-restart.png`). No deletion was undone by a save
    and nothing reappeared.

## Review-fix re-check (rebuilt binary, fresh isolated library)

The two-axis review raised four hard defects and some judgement calls; the hard
ones were fixed before publication and re-checked through the app. The interface
preference from the previous run persisted, so this pass ran in Indonesian.

1. Damaged recovery state travels with the CV: with `t08keep.json` replaced by a
   16-byte `corrupt current` file, the library row read `Keep me · Rusak`
   (`17-library-damaged.png`). It was moved to Trash, whose row also read
   `Keep me · Rusak` (`18-indonesian-trash-damaged.png`), then restored
   (`19-library-after-damaged-restore.png`). Opening it recovered the previous
   revision, kept the 16-byte damaged copy as
   `t08keep.corrupt-1794890545478946125.json`, showed the Indonesian recovery
   notice naming that file, and rendered the `Grace Hopper` preview
   (`20-damaged-restored-recovery.png`). Files after: current 1706 bytes
   (previous revision), previous 1706 bytes, damaged copy 16 bytes, Trash empty.
2. The confirmation body now states the erasure limit at the deciding moment:
   *“… Tindakan ini tidak dapat dibatalkan, dan penghapusan permanen bukan
   penghapusan aman. PDF dan cadangan proyek di luar pustaka tidak disentuh.”*
   (`21-confirm-dialog-not-secure-erasure.png`).
3. Focus stays inside that confirmation: one `Tab` put the ring on
   `Hapus permanen` (`22-dialog-tab-confirm.png`), a second returned it to
   `Batal` (`23-dialog-tab-back.png`), and `Escape` closed the dialog with the
   three Trash files still present (`24-dialog-escaped.png`). The same held after
   clicking the dialog body, which moves focus off the buttons: `Shift+Tab` put
   the ring back on `Batal` (`26-dialog-body-click-shifttab.png`) and `Escape`
   closed the dialog with the files unchanged (`27-dialog-body-click-escape.png`).
   Those two screenshots come from a second fresh seed of the same fixtures, so
   they show `Sampah (1)` again before the final confirmed deletion.
4. Confirming deletion again left only the 0-byte marker, kept the neighbouring
   CV and its recovery state, and left the external backup at 541 bytes with
   sha256 `ce5215a6…7d8b` (`25-permanent-cleanup-after-fix.png`).

## Not proven here

- No native Windows/Ubuntu run (tickets 10/11). Linux GTK dialogs only.
- A save already in flight at the instant of deletion cannot be produced through
  the public UI: leaving the editor awaits the pending write, the library has no
  open editor, and deletion waits for a successful flush. The "late save cannot
  resurrect a deleted or permanently deleted CV" rule is proven at the Rust seam
  (`a_late_save_cannot_resurrect_a_deleted_or_permanently_deleted_cv`) for both the
  Trash and the permanently-deleted state.
- The restore collision (an active file with the same id) has no UI path — ids are
  generated and never reused — and is proven at the Rust seam only.
- A cleanup failure mid-removal is induced with a folder in place of a recovery
  file (`a_failed_internal_cleanup_keeps_the_trash_row_retryable`); the running app
  was not driven through a real filesystem error for that path.
- The project has no frontend test harness; the approved test seam is the running
  application, so the Trash toggle, Restore, focus trap, and the confirmation
  dialog are demonstrated in the steps above rather than asserted by a test.
