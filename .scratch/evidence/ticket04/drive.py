#!/usr/bin/env python3
"""Ticket 04 driver: reuses the ticket 02 XTEST helper with its own data dir.

Usage: drive.py <command> [...]
  Same commands as ../ticket02/drive.py, plus:
  wheel <clicks>   scroll down (negative scrolls up)
  check            print saved projects with structure and skill groups
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
drive.DATA = pathlib.Path("/home/airzy/.hermes/cache/scratch/cvgen-ticket04")


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


def check():
    projects = sorted((drive.DATA / "data/app.cvgen.desktop/projects").glob("*.json"))
    print(f"files={len(projects)}")
    for path in projects:
        data = json.loads(path.read_text())
        cv = data["cv"]
        print(
            f"--- {path.name} updated_at={data['updated_at']} title={data['title']!r} "
            f"structure={data['structure']!r} name={cv['name']!r}"
        )
        print(f"experience={json.dumps(cv['experience'])}")
        print(f"skills={json.dumps(cv['skills'])}")


cmd = sys.argv[1]
if cmd == "wheel":
    wheel(int(sys.argv[2]))
elif cmd == "move":
    move(sys.argv[2], sys.argv[3])
elif cmd == "check":
    check()
else:
    drive.main()
