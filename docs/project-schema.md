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
        "start": "Jan 2020",
        "end": "Dec 2021",
        "bullets": ["Achievement text."]
      }
    ]
  }
}
```

## Fields

| Field | Meaning |
| --- | --- |
| `version` | Required file format version. Only `1` is read; a newer version is refused with an explicit error instead of being partially loaded. |
| `id` | Generated lowercase hex project identity. It is also the file name, and it is validated (ASCII alphanumerics and `-` only) before it is ever used as a path. |
| `title` | Optional project name shown in the library. Empty means the library shows the CV owner's name instead. |
| `structure` | Selected content structure: `reverse-chronological`, `functional`, or `combination`. Version 1 stores it even though only reverse chronological is presented so far; switching structures must never require re-entering content. |
| `content_language` | Selected content language for authored content and standard headings: `english` or `indonesian`. Stored from version 1; the editor currently authors English only. |
| `updated_at` | Unix seconds written by the application on every save; the library lists the most recently changed project first. |
| `cv` | The authored CV data. Every section is optional: incomplete drafts are valid files. |

`Experience.end` distinguishes two cases: `null` means an ongoing role and is
rendered as "Present"; `""` means no end date was recorded and is rendered without
an end. Dates stay free text with the precision the user typed, so no month or day
is ever inferred.

## Durability rules

- Saves write a temporary file in the same directory and rename it over the project
  file, so a failed write leaves the previous version intact.
- Completely blank entries, bullets, and sections are omitted from the PDF rather
  than rendered as empty rows or headings.
- Drafts may be incomplete; export additionally requires an owner name and at least
  one substantive non-contact section (a summary or a non-blank work experience
  entry), enforced in Rust.
- Files are unencrypted plain JSON. Corrupted-file recovery, one rolling previous
  revision, and project backup import/export are later slices and are not part of
  version 1 behaviour yet.
