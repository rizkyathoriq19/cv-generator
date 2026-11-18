// MCP server over stdio (`cv-generator --mcp`): lets an agent render a CV
// through the same validation and Typst template as the desktop export.
// Newline-delimited JSON-RPC 2.0; stdout carries protocol messages only.

use std::io::{self, BufRead, Write};
use std::path::Path;

use serde::Deserialize;
use serde_json::{json, Value};

use crate::{backup::export_backup, cv::*, files::*, render::*};

const DATE: &str = r#"{"type":"object","properties":{"year":{"type":["integer","null"]},"month":{"type":["integer","null"],"minimum":1,"maximum":12}}}"#;

fn input_schema() -> Value {
    let date: Value = serde_json::from_str(DATE).unwrap();
    let strings = json!({"type": "array", "items": {"type": "string"}});
    let s = json!({"type": "string"});
    json!({
        "type": "object",
        "required": ["cv", "output_path"],
        "properties": {
            "output_path": {"type": "string", "description": "Absolute path of the .pdf to write (replaced atomically)."},
            "backup_path": {"type": "string", "description": "Optional absolute .json path: a project backup the desktop app can import for later editing."},
            "title": {"type": "string", "description": "Project title stored in the backup."},
            "structure": {"type": "string", "enum": ["reverse-chronological", "functional", "combination"], "default": "reverse-chronological"},
            "content_language": {"type": "string", "enum": ["english", "indonesian"], "default": "english", "description": "Decides standard headings and 'Present' wording; content is not translated."},
            "cv": {
                "type": "object",
                "required": ["name"],
                "properties": {
                    "name": s, "headline": s, "email": s, "phone": s, "location": s, "summary": s,
                    "links": {"type": "array", "items": {"type": "object", "properties": {"label": s, "url": s}}},
                    "experience": {"type": "array", "items": {"type": "object", "properties": {
                        "title": s, "company": s, "location": s, "start": date,
                        "end": {"description": "null = Present (ongoing role)", "anyOf": [date, {"type": "null"}]},
                        "bullets": strings}}},
                    "education": {"type": "array", "items": {"type": "object", "properties": {
                        "school": s, "degree": s, "location": s, "start": date,
                        "end": {"anyOf": [date, {"type": "null"}]}, "bullets": strings}}},
                    "skills": {"type": "array", "description": "Skill groups; lead functional/combination CVs.", "items": {"type": "object", "properties": {"name": s, "bullets": strings}}},
                    "projects": {"type": "array", "items": {"type": "object", "properties": {"name": s, "detail": s, "bullets": strings}}},
                    "certifications": {"type": "array", "items": {"type": "object", "properties": {"name": s, "issuer": s, "date": date}}},
                    "languages": {"type": "array", "items": {"type": "object", "properties": {"name": s, "level": s}}}
                }
            }
        }
    })
}

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
fn generate(args: Value) -> Result<String, String> {
    let a: GenerateArgs = serde_json::from_value(args).map_err(|e| format!("invalid arguments: {e}"))?;
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
        let title = if a.title.trim().is_empty() { a.cv.name.clone() } else { a.title };
        let ack = export_backup(path, title, a.structure, lang, a.cv, lang)?;
        report["backup_path"] = json!(ack.path);
    }
    Ok(report.to_string())
}

/// One JSON-RPC message in, at most one response out (notifications get none).
pub(crate) fn handle(msg: &Value) -> Option<Value> {
    let id = msg.get("id")?.clone();
    let params = msg.get("params").cloned().unwrap_or(Value::Null);
    let result = match msg["method"].as_str().unwrap_or("") {
        "initialize" => json!({
            "protocolVersion": params["protocolVersion"].as_str().unwrap_or("2025-06-18"),
            "capabilities": {"tools": {}},
            "serverInfo": {"name": "cv-generator", "version": env!("CARGO_PKG_VERSION")}
        }),
        "ping" => json!({}),
        "tools/list" => json!({"tools": [{
            "name": "generate_cv",
            "description": "Render an ATS-friendly CV to a text-based A4 PDF (and optionally an importable project backup). Bullets are user-authored achievement lines; dates use month/year, year only, or empty.",
            "inputSchema": input_schema()
        }]}),
        "tools/call" if params["name"] == "generate_cv" => {
            match generate(params.get("arguments").cloned().unwrap_or(json!({}))) {
                Ok(text) => json!({"content": [{"type": "text", "text": text}]}),
                Err(e) => json!({"content": [{"type": "text", "text": e}], "isError": true}),
            }
        }
        "tools/call" => return Some(error(id, -32602, "unknown tool")),
        _ => return Some(error(id, -32601, "method not found")),
    };
    Some(json!({"jsonrpc": "2.0", "id": id, "result": result}))
}

fn error(id: Value, code: i32, message: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
}

pub fn serve() {
    let mut out = io::stdout().lock();
    for line in io::stdin().lock().lines().map_while(Result::ok) {
        if line.trim().is_empty() {
            continue;
        }
        let reply = match serde_json::from_str::<Value>(&line) {
            Ok(msg) => handle(&msg),
            Err(_) => Some(error(Value::Null, -32700, "parse error")),
        };
        if let Some(reply) = reply {
            let _ = writeln!(out, "{reply}");
            let _ = out.flush();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_pdf_and_reports_tool_errors() {
        let dir = std::env::temp_dir().join(format!("cvgen-mcp-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let pdf = dir.join("cv.pdf");
        let call = |args: Value| handle(&json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": {"name": "generate_cv", "arguments": args}})).unwrap();

        let ok = call(json!({"output_path": pdf, "backup_path": dir.join("cv.json"),
            "cv": {"name": "Ada", "summary": "Programmer.", "experience": [{"title": "Engineer",
            "company": "AE", "start": {"year": 2020, "month": 1}, "end": null, "bullets": ["Shipped"]}]}}));
        assert!(ok["result"].get("isError").is_none(), "{ok}");
        assert!(std::fs::read(&pdf).unwrap().starts_with(b"%PDF"));
        assert!(dir.join("cv.json").exists());

        let bad = call(json!({"output_path": pdf, "cv": {"name": ""}}));
        assert_eq!(bad["result"]["isError"], true);
        assert!(handle(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"})).is_none());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
