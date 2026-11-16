#!/usr/bin/env python3
"""Ticket 03 driver: reuses the ticket 02 XTEST helper with its own data dir.

Usage: drive.py <command> [...]
  Same commands as ../ticket02/drive.py, plus:
  wheel <clicks>   scroll down (negative scrolls up)
  check            print saved projects with the ticket 03 sections
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
drive.DATA = pathlib.Path("/home/airzy/.hermes/cache/scratch/cvgen-ticket03")


def wheel(clicks):
    drive.display()
    button = 5 if clicks > 0 else 4
    for _ in range(abs(clicks)):
        drive.XTST.XTestFakeButtonEvent(drive.D, button, 1, 0)
        drive.XTST.XTestFakeButtonEvent(drive.D, button, 0, 0)
        drive.flush(0.05)


def check():
    projects = sorted((drive.DATA / "data/app.cvgen.desktop/projects").glob("*.json"))
    print(f"files={len(projects)}")
    for path in projects:
        data = json.loads(path.read_text())
        cv = data["cv"]
        print(f"--- {path.name} updated_at={data['updated_at']} title={data['title']!r} name={cv['name']!r}")
        for key in (
            "email",
            "phone",
            "location",
            "links",
            "summary",
            "experience",
            "education",
            "skills",
            "projects",
            "certifications",
            "languages",
        ):
            print(f"{key}={json.dumps(cv[key])}")


cmd = sys.argv[1]
if cmd == "wheel":
    wheel(int(sys.argv[2]))
elif cmd == "check":
    check()
else:
    drive.main()
