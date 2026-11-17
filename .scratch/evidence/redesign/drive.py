#!/usr/bin/env python3
"""Redesign evidence driver: ticket 02's guarded XTEST helpers, an isolated
library seeded with synthetic CVs, and a window resize for the narrow layout.

Usage: drive.py seed | start | stop | shot <name> | click <x> <y> | key <keysym>
       | resize <w> <h> | wheel <x> <y> <clicks>
"""
import ctypes
import importlib.util
import json
import pathlib
import sys
import time

HERE = pathlib.Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("drive", HERE.parent / "ticket02" / "drive.py")
drive = importlib.util.module_from_spec(spec)
spec.loader.exec_module(drive)
drive.SHOTS = HERE
drive.DATA = pathlib.Path("/home/airzy/.hermes/cache/scratch/cvgen-redesign")
PROJECTS = drive.DATA / "data/app.cvgen.desktop/projects"


def cv(name, summary):
    roles = [
        ("Senior Platform Engineer", "Example Logistics", "Jakarta", (2021, 3), None),
        ("Software Engineer", "Sample Payments", "Bandung", (2018, 7), (2021, 2)),
        ("Junior Developer", "Placeholder Studio", "Bandung", (2016, 8), (2018, 6)),
    ]
    return {
        "name": name,
        "headline": "Full Stack Developer / Hardware Programmer",
        "email": "candidate@example.invalid",
        "phone": "+62 800 0000 0000",
        "location": "Jakarta, Indonesia",
        "links": [{"label": "Portfolio", "url": "https://example.invalid/portfolio"}],
        "summary": summary,
        "experience": [
            {
                "title": title,
                "company": company,
                "location": place,
                "start": {"year": start[0], "month": start[1]},
                "end": None if end is None else {"year": end[0], "month": end[1]},
                "bullets": [
                    f"Synthetic achievement {i}-{b}: reduced a measured fixture metric by a stated amount."
                    for b in range(3)
                ],
            }
            for i, (title, company, place, start, end) in enumerate(roles)
        ],
        "education": [
            {
                "school": "Example Institute of Technology",
                "degree": "BSc Computer Science",
                "location": "Bandung",
                "start": {"year": 2012, "month": 8},
                "end": {"year": 2016, "month": 7},
                "bullets": ["Synthetic thesis on local-first document rendering."],
            }
        ],
        "skills": [
            {"name": "Backend systems", "bullets": ["Synthetic skill evidence: designed fixture services."]},
            {"name": "Delivery", "bullets": ["Synthetic skill evidence: shipped fixture releases."]},
        ],
        "projects": [{"name": "Offline CV renderer", "detail": "Personal project", "bullets": ["Synthetic project bullet."]}],
        "certifications": [{"name": "Certified fixture tester", "issuer": "Example Board", "date": {"year": 2023, "month": None}}],
        "languages": [{"name": "Indonesian", "level": "Native"}, {"name": "English", "level": "Professional"}],
    }


def project(pid, title, name, summary, structure, language, updated_at):
    return {
        "version": 1,
        "id": pid,
        "title": title,
        "structure": structure,
        "content_language": language,
        "updated_at": updated_at,
        "cv": cv(name, summary),
    }


def seed():
    PROJECTS.mkdir(parents=True, exist_ok=True)
    for path in PROJECTS.iterdir():
        if path.is_file():
            path.unlink()
    for p in [
        project("rdmain", "Platform role — English", "Synthetic Candidate",
                "Synthetic summary: backend engineer focused on reliable local-first tools.",
                "reverse-chronological", "english", 1763270600),
        project("rdcopy", "Platform role — Indonesian", "Synthetic Candidate",
                "Ringkasan sintetis: insinyur backend yang berfokus pada alat lokal yang andal.",
                "combination", "indonesian", 1763270500),
    ]:
        (PROJECTS / f"{p['id']}.json").write_text(json.dumps(p, indent=2) + "\n")
    print(f"seeded {PROJECTS}")


X11 = drive.X11
X11.XDefaultRootWindow.argtypes = [ctypes.c_void_p]
X11.XDefaultRootWindow.restype = ctypes.c_ulong
X11.XQueryTree.argtypes = [ctypes.c_void_p, ctypes.c_ulong, ctypes.POINTER(ctypes.c_ulong), ctypes.POINTER(ctypes.c_ulong),
                           ctypes.POINTER(ctypes.POINTER(ctypes.c_ulong)), ctypes.POINTER(ctypes.c_uint)]
X11.XGetGeometry.argtypes = [ctypes.c_void_p, ctypes.c_ulong, ctypes.POINTER(ctypes.c_ulong), ctypes.POINTER(ctypes.c_int),
                             ctypes.POINTER(ctypes.c_int), ctypes.POINTER(ctypes.c_uint), ctypes.POINTER(ctypes.c_uint),
                             ctypes.POINTER(ctypes.c_uint), ctypes.POINTER(ctypes.c_uint)]
X11.XResizeWindow.argtypes = [ctypes.c_void_p, ctypes.c_ulong, ctypes.c_uint, ctypes.c_uint]


def area(window):
    vals = [ctypes.c_ulong(), ctypes.c_int(), ctypes.c_int(), ctypes.c_uint(), ctypes.c_uint(), ctypes.c_uint(), ctypes.c_uint()]
    X11.XGetGeometry(drive.display(), window, *map(ctypes.byref, vals))
    return vals[3].value * vals[4].value


def resize(width, height):
    root = X11.XDefaultRootWindow(drive.display())
    r, p, children, count = ctypes.c_ulong(), ctypes.c_ulong(), ctypes.POINTER(ctypes.c_ulong)(), ctypes.c_uint()
    X11.XQueryTree(drive.display(), root, ctypes.byref(r), ctypes.byref(p), ctypes.byref(children), ctypes.byref(count))
    window = max((children[i] for i in range(count.value)), key=area)
    X11.XResizeWindow(drive.display(), window, int(width), int(height))
    X11.XFlush(drive.display())
    time.sleep(1.5)


def wheel(x, y, clicks):
    D = drive.display()
    button = 5 if int(clicks) > 0 else 4
    drive.XTST.XTestFakeMotionEvent(D, -1, int(x), int(y), 0)
    drive.X11.XFlush(D)
    for _ in range(abs(int(clicks))):
        drive.XTST.XTestFakeButtonEvent(D, button, 1, 0)
        drive.XTST.XTestFakeButtonEvent(D, button, 0, 0)
        drive.X11.XFlush(D)
        time.sleep(0.05)
    time.sleep(0.6)


if __name__ == "__main__":
    cmd, args = sys.argv[1], sys.argv[2:]
    if cmd not in ("seed", "start", "stop"):
        drive.display()  # ticket 02's click/tap read the opened display global
    if cmd == "seed":
        seed()
    elif cmd == "start":
        drive.xvfb()
        drive.start_app()
    elif cmd == "stop":
        drive.kill_app()
    elif cmd == "shot":
        drive.shot(args[0])
    elif cmd == "click":
        drive.click(int(args[0]), int(args[1]))
        time.sleep(0.8)
    elif cmd == "key":
        drive.tap(args[0])
        time.sleep(0.5)
    elif cmd == "resize":
        resize(*args)
    elif cmd == "wheel":
        wheel(*args)
