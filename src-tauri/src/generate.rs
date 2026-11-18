// Headless render (`cv-generator --generate`): JSON arguments on stdin, a JSON
// report on stdout, or an error message on stderr with exit code 1. Driven by
// the TypeScript MCP server (mcp/server.ts); same validation and Typst
// template as the desktop export.

use std::io::{self, Read};
use std::path::Path;

use serde::Deserialize;
use serde_json::{json, Value};

use crate::{backup::export_backup, cv::*, files::*, render::*};

#[derive(Deserialize)]
struct GenerateArgs {
    cv: Cv,
    output_path: String,
    #[serde(default)]
    backup_path: Option<String>,
    #[serde(default)]
    title: String,
    #[serde(default)]
    structure: Structure,
    #[serde(default)]
    content_language: Language,
}

/// Same gates as the desktop export: validate, render, write .pdf atomically.
fn generate(args: Value) -> Result<Value, String> {
    let a: GenerateArgs =
        serde_json::from_value(args).map_err(|e| format!("invalid arguments: {e}"))?;
    let lang = a.content_language;
    if let Some(problem) = a.cv.export_problem(lang) {
        return Err(problem);
    }
    if !a.output_path.to_ascii_lowercase().ends_with(".pdf") {
        return Err("output_path must end with .pdf".into());
    }
    let pdf = render(&a.cv, a.structure, lang)?;
    save_atomically(Path::new(&a.output_path), &pdf, lang)?;
    let mut report = json!({
        "path": a.output_path,
        "bytes": pdf.len(),
        "sha256": hex(&pdf),
        "warnings": a.cv.export_warnings(lang, a.structure),
    });
    if let Some(path) = a.backup_path {
        let title = if a.title.trim().is_empty() {
            a.cv.name.clone()
        } else {
            a.title
        };
        let ack = export_backup(path, title, a.structure, lang, a.cv, lang)?;
        report["backup_path"] = json!(ack.path);
    }
    Ok(report)
}

/// Process exit code: 0 with the report on stdout, 1 with the error on stderr.
pub fn run() -> i32 {
    let mut input = String::new();
    let result = io::stdin()
        .read_to_string(&mut input)
        .map_err(|e| format!("could not read stdin: {e}"))
        .and_then(|_| serde_json::from_str(&input).map_err(|e| format!("invalid JSON: {e}")))
        .and_then(generate);
    match result {
        Ok(report) => {
            println!("{report}");
            0
        }
        Err(e) => {
            eprintln!("{e}");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_pdf_and_rejects_invalid_input() {
        let dir = std::env::temp_dir().join(format!("cvgen-generate-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let pdf = dir.join("cv.pdf");

        let ok = generate(json!({"output_path": pdf, "backup_path": dir.join("cv.json"),
            "cv": {"name": "Ada", "summary": "Programmer.", "experience": [{"title": "Engineer",
            "company": "AE", "start": {"year": 2020, "month": 1}, "end": null, "bullets": ["Shipped"]}]}}))
        .unwrap();
        assert!(ok["bytes"].as_u64().unwrap() > 0);
        assert!(std::fs::read(&pdf).unwrap().starts_with(b"%PDF"));
        assert!(dir.join("cv.json").exists());

        assert!(generate(json!({"output_path": pdf, "cv": {"name": ""}})).is_err());
        assert!(generate(
            json!({"output_path": dir.join("cv.txt"), "cv": {"name": "Ada", "summary": "x"}})
        )
        .is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
