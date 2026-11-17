#!/usr/bin/env python3
"""Ticket 08 driver: ticket 07's XTEST helper with its own isolated library.

Adds `seed` (two synthetic CVs, one with a kept previous revision and a kept
damaged copy) and `check` (the library and Trash folders, byte counts included).

Usage: drive.py <command> [...]
  xvfb | start | restart | stop | shot <name> | click <x> <y> | move <x> <y>
  tab <n> | stab <n> | enter | key <keysym> | type <text> | typewait <text> [delay]
  wheel <clicks> | close | windows | seed | check
"""
import importlib.util
import json
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parent
# ticket 02's driver defines the XTEST helpers under an import guard; ticket 07's
# executes its own dispatch on import, so it is not importable from here.
spec = importlib.util.spec_from_file_location("drive", HERE.parent / "ticket02" / "drive.py")
drive = importlib.util.module_from_spec(spec)
spec.loader.exec_module(drive)

drive.SHOTS = HERE
drive.DATA = pathlib.Path("/home/airzy/.hermes/cache/scratch/cvgen-ticket08")
PROJECTS = drive.DATA / "data/app.cvgen.desktop/projects"
TRASH = PROJECTS / "trash"


def cv(name, summary):
    return {
        "name": name,
        "email": "candidate@example.invalid",
        "phone": "",
        "location": "Testville",
        "links": [{"label": "Portfolio", "url": "https://example.invalid/portfolio"}],
        "summary": summary,
        "experience": [
            {
                "title": "Synthetic Engineer",
                "company": "Example Employer",
                "location": "Testville",
                "start": {"year": 2020, "month": 5},
                "end": None,
                "bullets": ["Authored achievement bullet."],
            }
        ],
        "education": [
            {
                "school": "Test University",
                "degree": "BSc Testing",
                "location": "Testville",
                "start": {"year": 2016, "month": 8},
                "end": {"year": 2019, "month": 5},
                "bullets": ["Authored education bullet."],
            }
        ],
        "skills": [{"name": "Test automation", "bullets": ["Authored skill bullet."]}],
        "projects": [{"name": "Synthetic project", "detail": "Local tooling", "bullets": []}],
        "certifications": [{"name": "Certified tester", "issuer": "Test board", "date": {"year": 2023, "month": None}}],
        "languages": [{"name": "Indonesian", "level": "Native"}],
    }


def project(pid, title, name, summary, updated_at):
    return {
        "version": 1,
        "id": pid,
        "title": title,
        "structure": "reverse-chronological",
        "content_language": "english",
        "updated_at": updated_at,
        "cv": cv(name, summary),
    }


def seed():
    """Two CVs; the one to delete also carries its previous revision and a kept
    damaged copy, so the move to Trash can be checked file by file."""
    PROJECTS.mkdir(parents=True, exist_ok=True)
    for folder in (PROJECTS, TRASH):
        if folder.exists():
            for path in folder.iterdir():
                if path.is_file():
                    path.unlink()
    if TRASH.exists():
        TRASH.rmdir()
    (PROJECTS / "t08demo.json").write_text(
        json.dumps(project("t08demo", "Trash demo", "Synthetic Candidate", "Trash demo current revision.", 1763270500), indent=2) + "\n"
    )
    (PROJECTS / "t08demo.prev.json").write_text(
        json.dumps(project("t08demo", "Trash demo", "Synthetic Candidate", "Trash demo previous revision.", 1763270400), indent=2) + "\n"
    )
    (PROJECTS / "t08demo.corrupt-1763270450000000000.json").write_bytes(b"corrupt kept copy\n")
    (PROJECTS / "t08keep.json").write_text(
        json.dumps(project("t08keep", "Keep me", "Grace Hopper", "Neighbour CV stays put.", 1763270300), indent=2) + "\n"
    )
    (PROJECTS / "t08keep.prev.json").write_text(
        json.dumps(project("t08keep", "Keep me", "Grace Hopper", "Neighbour previous revision.", 1763270200), indent=2) + "\n"
    )
    print(f"seeded {PROJECTS}")


def show(folder, label):
    if not folder.exists():
        print(f"{label}: <missing>")
        return
    print(f"{label}: {folder}")
    for path in sorted(folder.iterdir()):
        if not path.is_file():
            print(f"  {path.name} <directory>")
            continue
        print(f"  {path.name} size={path.stat().st_size}")
        if path.suffix != ".json":
            continue
        try:
            data = json.loads(path.read_text())
        except (json.JSONDecodeError, UnicodeDecodeError):
            print(f"    <unreadable> {path.read_bytes()[:40]!r}")
            continue
        summary = data.get("cv", {}).get("summary", "<none>")
        print(f"    id={data.get('id')!r} title={data.get('title')!r} summary={summary!r} updated_at={data.get('updated_at')}")


def check():
    show(PROJECTS, "PROJECTS")
    show(TRASH, "TRASH")
    print("external backup fixture:", HERE / "external-backup.json")


command = sys.argv[1]
if command == "seed":
    seed()
elif command == "check":
    check()
else:
    drive.main()
