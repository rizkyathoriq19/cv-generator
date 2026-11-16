# Project file schema (version 1)

One CV project is one JSON file in the application-managed local library:

```
<app data dir>/projects/<id>.json
```

`<app data dir>` is the platform application data directory for `app.cvgen.desktop`
(for example `~/.local/share/app.cvgen.desktop` on Linux). The application creates
the `projects` directory itself; users do not choose working-file locations.

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
| `id` | Generated lowercase hex project identity. It is also the file name, and it is validated (ASCII alphanumerics and `-` only) before it is ever used as a path. |
| `title` | Optional project name shown in the library. Empty means the library shows the CV owner's name instead. |
| `structure` | Selected content structure: `reverse-chronological` (detailed employment first, skills supporting), `functional` (skill-group achievements first, compact employer/title/date employment), or `combination` (skill-group achievements first plus detailed employment). It is a presentation choice over the same `cv` data: switching structures never edits, deletes, or requires re-entering content, including content a structure does not display. |
| `content_language` | Selected content language for authored content and standard headings: `english` or `indonesian`. Stored from version 1; the editor currently authors English only. |
| `updated_at` | Unix seconds written by the application on every save; the library lists the most recently changed project first. |
| `cv` | The authored CV data. Every section is optional: incomplete drafts are valid files. |

`cv` sections: contact details (`name`, `email`, `phone`, `location`, `links`),
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

- Saves write a temporary file in the same directory and rename it over the project
  file, so a failed write leaves the previous version intact.
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
- Files are unencrypted plain JSON. Corrupted-file recovery, one rolling previous
  revision, and project backup import/export are later slices and are not part of
  version 1 behaviour yet.
