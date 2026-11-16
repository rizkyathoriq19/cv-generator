#!/usr/bin/env python3
"""Ticket 05 driver: ticket 02's XTEST helper with its own library and commands.

Usage: drive.py <command> [...]
  xvfb | start | restart | stop | shot <name> | click <x> <y>
  tab <n> | stab <n> | enter | key <keysym> | type <text> | typewait <text> [delay]
  wheel <clicks> | seed | check
"""
import importlib.util
import json
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("drive", HERE.parent / "ticket02" / "drive.py")
drive = importlib.util.module_from_spec(spec)
spec.loader.exec_module(drive)

drive.SHOTS = HERE
drive.DATA = pathlib.Path("/home/airzy/.hermes/cache/scratch/cvgen-ticket05")
PROJECTS = drive.DATA / "data/app.cvgen.desktop/projects"


def wheel(clicks):
    drive.display()
    button = 5 if clicks > 0 else 4
    for _ in range(abs(clicks)):
        drive.XTST.XTestFakeButtonEvent(drive.D, button, 1, 0)
        drive.XTST.XTestFakeButtonEvent(drive.D, button, 0, 0)
        drive.flush(0.05)


def move(x, y):
    drive.display()
    drive.XTST.XTestFakeMotionEvent(drive.D, -1, int(x), int(y), 0)
    drive.flush()


def seed():
    """One complete synthetic CV; all later copies are made through the UI."""
    cv = {
        "name": "Synthetic Candidate",
        "email": "candidate@example.invalid",
        "phone": "",
        "location": "Testville",
        "links": [{"label": "Portfolio", "url": "https://example.invalid/portfolio"}],
        "summary": "Authored English summary stays as written.",
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
    project = {
        "version": 1,
        "id": "t05source",
        "title": "English CV",
        "structure": "reverse-chronological",
        "content_language": "english",
        "updated_at": 1763270400,
        "cv": cv,
    }
    PROJECTS.mkdir(parents=True, exist_ok=True)
    for path in PROJECTS.glob("*.json"):
        path.unlink()
    (PROJECTS / "t05source.json").write_text(json.dumps(project, indent=2) + "\n")
    print(f"seeded {PROJECTS / 't05source.json'}")


def check():
    projects = sorted(PROJECTS.glob("*.json"))
    print(f"files={len(projects)}")
    for path in projects:
        data = json.loads(path.read_text())
        cv = data["cv"]
        print(
            f"--- {path.name} id={data['id']} title={data['title']!r} "
            f"structure={data['structure']!r} content_language={data['content_language']!r} "
            f"updated_at={data['updated_at']}"
        )
        print(f"name={cv['name']!r} summary={cv['summary']!r}")
        print(f"experience={json.dumps(cv['experience'])}")
        print(f"skills={json.dumps(cv['skills'])}")


cmd = sys.argv[1]
if cmd == "wheel":
    wheel(int(sys.argv[2]))
elif cmd == "move":
    move(sys.argv[2], sys.argv[3])
elif cmd == "seed":
    seed()
elif cmd == "check":
    check()
else:
    drive.main()
