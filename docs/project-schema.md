# Project file schema (version 1)

One CV project is one JSON file in the application-managed local library:

```
<app data dir>/projects/<id>.json
```

`<app data dir>` is the platform application data directory for `app.cvgen.desktop`
(for example `~/.local/share/app.cvgen.desktop` on Linux). The application creates
the `projects` directory itself; users do not choose working-file locations.

The library folder also holds the `trash/` folder (see below) and two non-project
files per CV, which are never listed as CVs because their names are not project
ids:

| File | Meaning |
| --- | --- |
| `<id>.prev.json` | The one rolling previous valid saved revision, replaced on every save. |
| `<id>.corrupt-<nanoseconds>.json` | Damaged bytes preserved when an unreadable current file was recovered. |

One running instance owns the library through `<app data dir>/instance.lock`; the
operating system releases the lock when the process exits, so a crash leaves no
stale lock.

## Shape

```json
{
  "version": 1,
  "id": "19a7d3c9f21b0042",
  "title": "",
  "structure": "reverse-chronological",
  "content_language": "english",
  "updated_at": 1763270400,
  "cv": {
    "name": "Ada Lovelace",
    "headline": "Analytical Engine Programmer",
    "email": "ada@example.invalid",
    "phone": "",
    "location": "London",
    "links": [{ "label": "Web", "url": "https://example.invalid/portfolio" }],
    "summary": "Analytical engine programmer.",
    "experience": [
      {
        "title": "Engineer",
        "company": "Analytical Engines Ltd",
        "location": "London",
        "start": { "year": 2020, "month": 1 },
        "end": null,
        "bullets": ["Achievement text."]
      }
    ],
    "education": [
      {
        "school": "Test University",
        "degree": "BSc Testing",
        "location": "Testville",
        "start": { "year": 2016, "month": 8 },
        "end": { "year": 2019, "month": null },
        "bullets": ["Graduated with honours."]
      }
    ],
    "skills": [{ "name": "Test automation", "bullets": ["Wrote regression checks."] }],
    "projects": [{ "name": "Synthetic project", "detail": "Local tooling", "bullets": [] }],
    "certifications": [
      { "name": "Certified tester", "issuer": "Test board", "date": { "year": 2023, "month": null } }
    ],
    "languages": [{ "name": "Indonesian", "level": "Native" }]
  }
}
```

## Fields

| Field | Meaning |
| --- | --- |
| `version` | Required file format version. Only `1` is read; a newer version is refused with an explicit error instead of being partially loaded. |
| `id` | Generated lowercase hex project identity. It is also the file name, and it is validated (ASCII alphanumerics and `-` only) before it is ever used as a path. Duplicating a project writes a second file under a fresh id, so the two are independent from that point on; a duplicate never shares or synchronizes data with its source. |
| `title` | Optional project name shown in the library. Empty means the library shows the CV owner's name instead. |
| `structure` | Selected content structure: `reverse-chronological` (detailed employment first, skills supporting), `functional` (skill-group achievements first, compact employer/title/date employment), or `combination` (skill-group achievements first plus detailed employment). It is a presentation choice over the same `cv` data: switching structures never edits, deletes, or requires re-entering content, including content a structure does not display. |
| `content_language` | Selected content language for standard headings: `english` or `indonesian`. It controls the PDF section headings, month abbreviations, and the ongoing-role wording (`Present` / `Sekarang`); authored text is copied verbatim and is never translated. It is independent of the interface language, which is stored outside the project file. |
| `updated_at` | Unix seconds written by the application on every save; the library lists the most recently changed project first. |
| `cv` | The authored CV data. Every section is optional: incomplete drafts are valid files. |

`cv` sections: contact details (`name`, optional `headline`, `email`, `phone`, `location`, `links`),
`summary`, `experience`, `education`, `skills`, `projects`, `certifications`, and
`languages`. Each entry type is a plain list the user can add to, remove from, and
reorder; stored order is the user's manual order.

## Dates

A career date is `{ "year": number | null, "month": number | null }`, so a
month/year date, a year-only date, and no date at all stay distinct:

- `{ "year": 2020, "month": 1 }` renders `Jan 2020`.
- `{ "year": 2020, "month": null }` renders `2020` — a missing month is never
  materialized into January or any other month.
- `{ "year": null, "month": null }` renders no date text.
- `month` is only ever stored when `year` is present.

`Experience.end` (and `Education.end`) distinguishes three cases: `null` means an
ongoing role rendered as "Present"; a date object with a `year` is the recorded
end; an empty date object (`year: null`) means no end date was recorded and no end
is rendered.

Ordering in the PDF: ongoing entries first in manual order, then dated entries by
newest start year. Within one year, months order the entries only when every entry
in that year has a month; otherwise the whole year keeps manual order. Undated
entries come last in manual order. Project files never re-order entries, so ties
survive save/reopen.

## Durability and validation rules

- Saves retain exactly one rolling previous valid revision. The sequence is: prepare
  the new bytes, copy a valid current file to `<id>.prev.json` (temp file + rename),
  then rename the new bytes over `<id>.json`. A failure at any step leaves the current
  file byte-identical, so a crash or a denied write cannot lose the last saved CV. A
  current file that no longer parses is never rotated, because that would overwrite
  the only recoverable revision. The previous revision is one save behind, not full
  version history, and edits that never reached a save are not recoverable.
- An unreadable current file is not silently replaced. Opening the CV copies the
  damaged bytes to `<id>.corrupt-<nanoseconds>.json`, restores `<id>.prev.json`'s own
  bytes as the current file (same content and `updated_at`, not a re-serialised
  edit), and returns an explicit notice naming the kept file. The copy, not a move,
  keeps the CV at its current path until the replacement lands, so a failed recovery
  write still leaves a listed, recoverable project. If no valid previous revision
  exists, nothing is written or blanked: opening reports that the file is missing or
  damaged and where it remains. A damaged CV stays listed (marked `Damaged`) so it
  can be opened, recovered, or reported. Damage found while the editor is open is
  preserved the same way before the next save replaces it, and that save reports the
  kept file in the editor instead of silently replacing it.
- A project file this app refuses to read — a newer `version` or another project's
  `id` — is reported and left byte-identical. It is never quarantined, rolled back to
  an older revision, or labelled damaged, and a save refuses to overwrite it; the
  library keeps listing it and opening explains the refusal in the interface language.
- A failed save keeps the edits in process memory and reports `Save failed`; retrying
  writes the same edits, and exporting a project backup is a separate operation that
  does not mark the CV saved. On a normal close pending edits are written first; if
  that write fails, the application offers Retry / Export backup / Discard and close /
  Cancel, and only the explicit discard abandons unsaved edits.
- Completely blank entries, bullets, and sections are omitted from the PDF rather
  than rendered as empty rows or headings.
- Drafts may be incomplete; export additionally requires an owner name and at least
  one substantive non-contact section (summary, experience, education, skills,
  projects, certifications, or languages), enforced in Rust.
- Export is blocked by provably contradictory date ranges (both years known and the
  start is later, or the same year with both months known and the start month is
  later), by a malformed supplied email address, by a supplied web link that does
  not start with `http://`/`https://` or contains whitespace, and by a stored date
  shape that breaks this contract (month outside 1–12, or a month without a year).
  Missing optional content only produces warnings: missing contact details, undated
  entries, and — for the `functional` and `combination` structures only — when no
  skill group has an achievement bullet. The last warning never blocks export.
- Files are unencrypted plain JSON. No application password, encryption, or
  secure-erasure guarantee exists; the OS account protects local files.

## Trash

Deleting a CV moves it into `projects/trash/`, a folder inside the library that the
library listing never reads as active CVs. The move carries every internal file
that belongs to that CV:

| File | Meaning |
| --- | --- |
| `<id>.json` | The CV as it was when deleted (possibly the damaged current file). |
| `<id>.prev.json` | Its one rolling previous revision, unchanged. |
| `<id>.corrupt-<nanoseconds>.json` | Any damaged copy kept during an earlier recovery. |

- Moving is a rename on the same filesystem; if a file cannot be moved, the files
  that already moved are moved back, so a CV is never split between the library and
  Trash.
- Nothing in Trash is erased or rewritten, and no automatic cleanup runs: a CV stays
  restorable until it is permanently deleted.
- Restore moves the same files back. It refuses — changing nothing — when the
  library already holds any file for that id, so it can never overwrite or merge
  another CV. A restored damaged CV opens through the normal recovery path.
- Permanent deletion is only offered for a CV in Trash and requires an explicit
  confirmation in the interface. It removes that CV's saved file, previous revision,
  and kept damaged copies, and nothing else: other CVs and exported PDFs or project
  backups outside the library are untouched.
- Permanent deletion leaves an empty `<id>.deleted.json` marker, so a save that was
  already scheduled when the CV was deleted cannot recreate the project. The
  application refuses a save for a CV that is in Trash or permanently deleted.
- Trash and recovery files are unencrypted plain JSON protected only by OS account
  permissions. Permanent deletion is not secure erasure, and the interface says so.

## Project backup (version 1)

A backup is a portable, editable, **unencrypted** snapshot of one project, distinct
from a PDF. It is the project shape without identity or save state:

```json
{
  "format": "cv-generator-backup",
  "version": 1,
  "title": "",
  "structure": "reverse-chronological",
  "content_language": "english",
  "cv": { "...": "same as the project file" }
}
```

- Export snapshots the editor's current in-memory CV (including unsaved edits),
  structure, content language, and title. Destination must end in `.json` (the import
  picker filters to the same type). On Linux the save dialog asks before replacing a
  file; other platforms are unverified until the package slices. The write is temp
  file + rename, so a cancelled or failed export leaves the existing destination
  untouched.
- Import **always** creates a new project with a freshly generated id, so repeated
  imports never share identity and never replace a project. A backup has no `id`,
  `updated_at`, or path field: unknown top-level fields (including those) are rejected
  rather than trusted.
- Everything is validated before the library is touched: the file is read with a
  hard cap of **2 MiB** (`MAX_BACKUP_BYTES`, 2,097,152 bytes; exactly that size is
  accepted, one byte more is refused; export refuses to write a larger file), the
  `format` marker must match, `version` greater than 1 is refused by name as a newer
  format before its shape is read, and the remaining fields must deserialize into the
  schema: `cv` is required (an empty `{}` CV is a valid draft), and stored date shapes
  must be valid. Export applies the same date rule, so a written backup always imports.
  Rejection leaves the library byte-identical.
- All text is plain data: it is stored verbatim and rendered by the same
  data-only template path as authored text, never evaluated.
- Import/export controls and errors follow the interface language, including read,
  parse, size, and destination-write failures; the backup stores only the content
  language. Saving a working project keeps the operating-system detail of a write
  failure in English (ticket 05 limit).
