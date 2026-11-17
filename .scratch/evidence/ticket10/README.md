# Evidence — ticket 10 Ubuntu 24.04 package and Arch/Omarchy AppImage

Synthetic fixtures only. The packaged app ran against an isolated
`XDG_DATA_HOME` (`/home/airzy/.hermes/cache/scratch/cvgen-ticket10/ubuntu/data`),
never the real library. Screenshots and exported PDFs are kept outside the
repository in that scratch directory; their observed values are quoted here.

**Scope limit:** there is no native Ubuntu host or VM on this machine, so the
Debian package was installed and driven in a `ubuntu:24.04` container. That proves
the package, its dependencies and the application workflows on that userspace; it
does not prove a desktop session, GPU or native-host integration. The native
Ubuntu acceptance line in the ticket is left unticked.

## Environment

- Host: Omarchy (Arch-based), Linux 7.2.15-1-omarchy, Docker 29.9.2.
- Build image: `ubuntu:24.04` (24.04.5 LTS), Rust stable via rustup, Node 22.23.5,
  `libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, `patchelf`, `libxdo-dev`
  (recipe: [`Dockerfile`](Dockerfile)).
- Runtime image `cvgen-u2404-app`: the `ubuntu:24.04` install container with the
  package installed, plus `xvfb`, `imagemagick`, `python3`, `fonts-dejavu-core`,
  `libxtst6` for the headless driver.
- Display: Xvfb `:99` inside the runtime container; the app ran as uid 1000.

## Why the host build is not a deliverable

A package built on Omarchy installs on Ubuntu 24.04 and then dies at exec:

```
$ ldd /usr/bin/cv-generator | grep "not found"
/usr/bin/cv-generator: /lib/x86_64-linux-gnu/libm.so.6: version `GLIBC_2.43' not found (required by /usr/bin/cv-generator)
```

The deliverable packages are therefore built inside the baseline:

```bash
docker build -t cvgen-ubuntu2404-build .scratch/evidence/ticket10
docker run --rm --user 1000:1000 -e HOME=/home/ubuntu -v "$PWD:/work" -w /work \
  -e APPIMAGE_EXTRACT_AND_RUN=1 -e CARGO_TARGET_DIR=/work/src-tauri/target/ubuntu-24.04 \
  cvgen-ubuntu2404-build bash -lc 'npm ci && npx tauri build --bundles deb,appimage'
```

Observed: `Finished 2 bundles` — `CV Generator_0.1.0_amd64.deb` (20.05 MiB) and
`CV Generator_0.1.0_amd64.AppImage` (93.23 MiB) under
`src-tauri/target/ubuntu-24.04/release/bundle/`. The Linux target list lives in
`src-tauri/tauri.linux.conf.json` so a Windows build keeps its own bundle targets;
a plain `npx tauri build` in this image produced exactly those two bundles.

```text
deb      273e30145a6462dd9511d0c0c3a0c65f95e73dc5ee7edc299c19c9e7530fec1a
appimage 8b873f9150ce0e7e9c1a0632fd398f56519904e278f3583f7370f7709bb5bde9
```

After the final rebuild the install (`install-exit=0`, `ldd … not found` → `0`) and
the AppImage smoke test were repeated against these artifacts.

## Packaged payload

`dpkg-deb -I` reports `Depends: libwebkit2gtk-4.1-0, libgtk-3-0`; `apt` resolves the
second to `libgtk-3-0t64` on 24.04. `dpkg-deb -c` lists:

```
usr/bin/cv-generator
usr/share/applications/CV Generator.desktop
usr/share/icons/hicolor/…/apps/cv-generator.png
usr/share/doc/cv-generator/third-party-notices.md
usr/share/doc/cv-generator/install.md
usr/share/doc/cv-generator/LICENSE-Apache-2.0.txt
usr/share/doc/cv-generator/LICENSE-MIT-React.txt
usr/share/doc/cv-generator/LICENSE-LiberationSerif.txt
```

The templates and Liberation Serif faces are `include_str!`/`include_bytes!` in the
Rust binary and PDF.js is bundled by Vite, so there is no separate resource
directory to ship; the `bundle.linux.*.files` entries add what the user otherwise
would not receive: the notices table, the Linux install note, the Apache-2.0 text
for Typst/PDF.js/Tauri (Tauri is dual MIT/Apache-2.0), the MIT text for the bundled
React runtime, and the OFL text for the font. The extracted AppImage AppDir carries
the same five files under `usr/share/doc/cv-generator/`.

## Install (network required for dependencies)

The runtime image is built once from the plain baseline plus the headless tooling:

```bash
docker run --name cvgen-u2404-base ubuntu:24.04 bash -lc '
  apt-get update && apt-get install -y --no-install-recommends \
    xvfb imagemagick python3 fonts-dejavu-core dbus-x11 libxtst6 x11-utils procps'
docker commit cvgen-u2404-base cvgen-u2404-base
```

Then the package is installed into that image (this step needs the archive network):

```bash
docker run --name cvgen-u2404-install -v "$PWD/src-tauri/target/ubuntu-24.04/release/bundle:/pkg:ro" \
  cvgen-u2404-base bash -lc 'apt-get update && apt-get install -y --no-install-recommends "/pkg/deb/CV Generator_0.1.0_amd64.deb"'
docker commit cvgen-u2404-install cvgen-u2404-app
docker rm -f cvgen-u2404-install
```

`install-exit=0`, `dpkg -l cv-generator` → `0.1.0 amd64 Offline desktop CV generator`,
`ldd /usr/bin/cv-generator | grep -c "not found"` → `0`. First install therefore
needs the archive network; nothing below does. The packaged
`/usr/share/doc/cv-generator/install.md` records that dependency requirement, the
tested baseline and the unencrypted local data location for the user (US-56).

## Offline workflow run

The install container was committed and rerun with `--network none` and the app
executable, then driven with [`drive.py`](drive.py) (ticket 02's XTEST helper plus
`seed`/`seed-id`/`resize`/`wheel`/`check`) and [`export_case.sh`](export_case.sh):

```bash
docker run -d --name cvgen-u2404-run --network none --user 1000:1000 -e HOME=/home/ubuntu \
  -v "$PWD:/work:ro" -v /home/airzy/.hermes/cache/scratch/cvgen-ticket10:/data \
  cvgen-u2404-app sleep infinity
docker exec cvgen-u2404-run sh -lc 'Xvfb :99 -screen 0 1600x2400x24 & sleep 2'
# seed writes into CVGEN_DATA/data/app.cvgen.desktop/projects, so it must use the
# same CVGEN_DATA the app runs with (/data/ubuntu here).
docker exec -e CVGEN_DATA=/data/ubuntu \
  cvgen-u2404-run python3 /work/.scratch/evidence/ticket10/drive.py seed
docker exec -e CVGEN_DATA=/data/ubuntu \
  cvgen-u2404-run python3 /work/.scratch/evidence/ticket10/drive.py seed-id
docker exec -e CVGEN_BIN=/usr/bin/cv-generator -e CVGEN_DATA=/data/ubuntu \
  -e CVGEN_SHOTS=/data/ubuntu/shots -e WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS=1 \
  cvgen-u2404-run python3 /work/.scratch/evidence/ticket10/drive.py start
```

```text
/proc/net/dev:  lo only (no other interface)
app log:        AT-SPI accessibility-bus warnings and two Gtk-CRITICAL
                gtk_widget_event assertions (headless GTK, no window manager);
                no network, telemetry or CV-content lines
```

1. Library seeded with four synthetic projects; rows render
   (`01-library.png`, `open-library.png`).
2. Opened `Complete CV`; autosaved edits and a rolling `.prev.json` were observed
   (`26-retry-saved.png`, project JSON).
3. Structures and content languages switched through the real selects and exported:
   reverse chronological, functional, combination, and combination with Indonesian
   headings (`complete-*.pdf` below).
4. Reopen after restart: the edited project title persisted
   (`t10complete.json` `"title": "Blocked save CV"`), preview regenerated.
5. Damaged current revision: library row read `Complete CV - Damaged`; opening it
   showed *“Your saved file was damaged, so the previous saved revision was
   recovered…”* naming the kept copy
   `t10complete.corrupt-1794905508433900864.json` (15 bytes) beside the recovered
   `t10complete.json` (3370 bytes).
6. Save failure: with the projects directory `chmod 0555`, an edit left the stored
   title unchanged and the toolbar showed `Save failed` with `Retry save`
   (`25-blocked-save.png`). Restoring write permission and pressing `Retry save`
   persisted the edit.
7. Trash: `Move to Trash` on `Keep me` moved it to the Trash view (`Trash (1)`,
   row with `Restore` / `Delete permanently`); `Restore` returned it and the
   library showed `Trash (0)` again.
8. Invalid import: `{not json` through the real open dialog produced
   *“This is not a valid CV Generator backup: key must be a string at line 1
   column 2”* with the library unchanged.
9. Narrow window: at 780×900 the editor shows `Edit CV` / `PDF preview` tabs
   (`34-narrow-editor.png`); keyboard focus traversal was only partially walked
   (see gaps).
10. Interface language switched to Bahasa Indonesia: toolbar read `Antarmuka`,
   `CV Baru`, `Muat ulang`, `Impor cadangan proyek…`, `Sampah (0)`, rows
   `Duplikat` / `Pindahkan ke Sampah` (`41-interface-indonesian.png`).
11. Backup transfer: `Ekspor cadangan proyek…` wrote `complete-backup.json`
   (3.3 KB, keys `format, version, title, structure, content_language, cv`), and
   `Impor cadangan proyek…` created an independent copy
   `18e8c9e151be2a5f0000000000000.json` with an identical `cv` object, its own id
   and the source project untouched.
12. Indonesian reverse-chronological: `drive.py seed-id` adds two fixtures with
   `content_language: indonesian` (not switched in the UI for these exports);
   exporting the reverse one rendered `RINGKASAN PROFESIONAL` then `PENGALAMAN
   KERJA` before the skill section.
13. Indonesian functional: the same `seed-id` fixture with `structure: functional`
   rendered `RINGKASAN PROFESIONAL` then `KEAHLIAN` before employment. The UI switch
   itself is demonstrated in steps 3 and 10 (Indonesian combination and interface
   language); these two files close the Indonesian side of the structure matrix.

## Packaged-output PDFs

Exported through the app's own GTK save dialog from the installed package; inspected
outside the app:

```text
fresh-graduate.pdf           1 page  A4  d7d4e579198fb9cd07a3e2f70b6942c7e301c08fe281ddb4ae9fe97ce80100c8
complete-reverse.pdf         1 page  A4  57bb0320e0beb6ebdc6373c9e4f51238917c943b0109dc21318432f9de9e25d7
complete-functional.pdf      1 page  A4  a55c0e2033ee231f05aaf7bc6c477698a15f4c09f90f64a62bfba8cce477e464
complete-combination.pdf     1 page  A4  fdfd90c9b0025a14048ee5a19847e09e8542489c15acaed45a0bf072a077f8a8
complete-combination-id.pdf  1 page  A4  99ce4bc04257936d88339fc7689875daec9c0c5dd56e334a0df87c2f37b4fee9
complete-reverse-id.pdf      1 page  A4  eea8bb71bee55164fab09045cf8db028281e1d038e81748b0190f8179cca6846
complete-functional-id.pdf   1 page  A4  89ad3e513e3af9b3590c94af5267c47194fad4ec66f485e00d29bbd9e1527681
multipage.pdf                2 pages A4  82e9d34e54876a568be9e1c12cfe64055264ac080a843357b62a291d6203dfa5
```

Every page is `595.276 x 841.89 pts`; `pdffonts` reports `LiberationSerif` and
`LiberationSerif-Bold` with `emb=yes sub=yes uni=yes`. `pdftotext` order follows the
selected structure (`WORK EXPERIENCE` before `SKILLS` for reverse; `SKILLS` before
`WORK EXPERIENCE` for functional and combination) and the Indonesian run uses
`RINGKASAN PROFESIONAL`, `KEAHLIAN`, `PENGALAMAN KERJA`, `PENDIDIKAN`, `PROYEK`,
`SERTIFIKASI`, `BAHASA`. The multipage PDF's page 2 begins with a role and its
bullets, not a bare section heading.

The three required cases were also rendered and inspected as pixels
(`pdftoppm -r 90`, `-r 200` for a skills-block crop): no overlapping or struck-through
text, no clipping at the page edges, no tofu/missing glyphs, and no empty sections.
The only ambiguity raised on a 90 dpi render — a rule appearing to cross
`Synthetic skill group 1` — disappeared at 200 dpi, where the rules sit in whitespace
above headings. The rendered images stay in scratch
(`.../ubuntu/out/render/*.png`), not in the repository.

## Arch/Omarchy AppImage smoke test

The delivered AppImage (`8b873f91…`, the package containing all five doc files) was
extracted (`--appimage-extract`, no FUSE) and run natively on Omarchy with an
isolated data directory:

- `squashfs-root/usr/share/doc/cv-generator/` contains `third-party-notices.md`,
  `install.md`, `LICENSE-Apache-2.0.txt`, `LICENSE-MIT-React.txt` and
  `LICENSE-LiberationSerif.txt`;
- library starts, `Complete CV` opens, the PDF preview canvas rendered the CV text
  (OCR read `Complete Candidate` / `PROFESSIONAL SUMMARY`);
- the project title was edited to `Delivered appimage CV`, autosaved, and the app was
  restarted; the reopened library listed `Delivered appimage CV` with a `.prev.json`
  revision;
- `Export current PDF…` produced `delivered.pdf` (34,541 bytes) whose text starts
  `Complete Candidate`; its sha256
  `57bb0320e0beb6ebdc6373c9e4f51238917c943b0109dc21318432f9de9e25d7` is identical to
  `complete-reverse.pdf` exported from the installed deb, so the two packaged builds
  render byte-identical output for the same CV. App log empty (0 lines).

This is one distribution smoke test, not a promise of universal Linux support.

## Not proven here

- Native Ubuntu 24.04 install, launch and desktop integration: no native host or VM.
  All Ubuntu evidence is container-based.
- Current-revision render races and the stale-preview export branch: no public UI
  trigger, still proven only at the Rust/TS seam (ticket 09).
- Complete keyboard walkthrough of the packaged app: only the narrow-window pane tabs
  were reached; the full focus order and reorder/remove controls were not re-walked
  in this package.
- Inno Setup/NSIS and Windows: ticket 11.
