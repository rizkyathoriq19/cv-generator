# CV Generator

Offline desktop app (Tauri + React + Typst) for writing ATS-friendly CVs and exporting text-based PDFs.
Domain terms: [CONTEXT.md](CONTEXT.md). Project file format: [docs/project-schema.md](docs/project-schema.md).
Linux install: [docs/linux-install.md](docs/linux-install.md).

## Development

```bash
npm install
npm run app          # desktop app (tauri dev)
npm run test         # frontend tests
npm run build        # type check + vite build
cd src-tauri && cargo test
```

## MCP server (generate a CV from an agent)

The same binary runs as a stdio [MCP](https://modelcontextprotocol.io) server with `--mcp`.
It uses the same validation and Typst template as the app's PDF export, and needs no window or network.

Build it:

```bash
cd src-tauri && cargo build --release   # -> src-tauri/target/release/cv-generator
```

Register it with your agent, for example in an `mcpServers` config:

```json
{
  "mcpServers": {
    "cv-generator": {
      "command": "/absolute/path/to/src-tauri/target/release/cv-generator",
      "args": ["--mcp"]
    }
  }
}
```

### Tool: `generate_cv`

| Argument | Required | Notes |
|---|---|---|
| `cv` | yes | CV content, same shape as `cv` in [docs/project-schema.md](docs/project-schema.md). `name` plus at least one substantive section is required. |
| `output_path` | yes | Absolute `.pdf` path. The file is replaced atomically. |
| `structure` | no | `reverse-chronological` (default), `functional`, or `combination` |
| `content_language` | no | `english` (default) or `indonesian`. Sets the section headings and the "Present" wording. Your content is not translated. |
| `backup_path` | no | Absolute `.json` path for a project backup. Import it in the app to keep editing. |
| `title` | no | Project title stored in the backup. Defaults to `cv.name`. |

Dates are `{ "year": 2022, "month": 3 }`, year only, or empty. An `end` of `null` means an ongoing role (Present).
The tool returns `{ path, bytes, sha256, warnings }`. Validation failures come back as tool errors (`isError: true`).

Example prompt: *"Generate my CV as a combination structure in Indonesian to ~/cv.pdf, and save a backup to ~/cv.json."*

The server does not read or write the app's local library. To edit the result in the app, use `backup_path` and import that file.
