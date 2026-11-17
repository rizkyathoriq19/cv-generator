#!/usr/bin/env python3
"""Ticket 09 driver: ticket 02's XTEST helper plus window resizing and a seed.

Adds `seed` (two synthetic CVs, the first with long multipage content so a
render takes longer than one screenshot) and `resize` (narrow/window widths
exercise the tab layout). Everything else runs ticket 02's dispatcher.

Usage: drive.py <command> [...]
  start | restart | stop | shot <name> | click <x> <y> | move <x> <y>
  tab <n> | stab <n> | enter | key <keysym> | type <text> | typewait <text> [delay]
  resize <width> <height> | seed | check
"""
import ctypes
import importlib.util
import json
import pathlib
import subprocess
import sys
import time

HERE = pathlib.Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("drive", HERE.parent / "ticket02" / "drive.py")
drive = importlib.util.module_from_spec(spec)
spec.loader.exec_module(drive)

drive.SHOTS = HERE
drive.DATA = pathlib.Path("/home/airzy/.hermes/cache/scratch/cvgen-ticket09")
PROJECTS = drive.DATA / "data/app.cvgen.desktop/projects"

X11 = drive.X11
X11.XQueryTree.argtypes = [
    ctypes.c_void_p,
    ctypes.c_ulong,
    ctypes.POINTER(ctypes.c_ulong),
    ctypes.POINTER(ctypes.c_ulong),
    ctypes.POINTER(ctypes.POINTER(ctypes.c_ulong)),
    ctypes.POINTER(ctypes.c_uint),
]
X11.XResizeWindow.argtypes = [ctypes.c_void_p, ctypes.c_ulong, ctypes.c_uint, ctypes.c_uint]
X11.XDefaultRootWindow.argtypes = [ctypes.c_void_p]
X11.XDefaultRootWindow.restype = ctypes.c_ulong


def geometry(window):
    root = ctypes.c_ulong()
    x = ctypes.c_int()
    y = ctypes.c_int()
    w = ctypes.c_uint()
    h = ctypes.c_uint()
    border = ctypes.c_uint()
    depth = ctypes.c_uint()
    X11.XGetGeometry.argtypes = [
        ctypes.c_void_p,
        ctypes.c_ulong,
        ctypes.POINTER(ctypes.c_ulong),
        ctypes.POINTER(ctypes.c_int),
        ctypes.POINTER(ctypes.c_int),
        ctypes.POINTER(ctypes.c_uint),
        ctypes.POINTER(ctypes.c_uint),
        ctypes.POINTER(ctypes.c_uint),
        ctypes.POINTER(ctypes.c_uint),
    ]
    X11.XGetGeometry(
        drive.display(),
        window,
        ctypes.byref(root),
        ctypes.byref(x),
        ctypes.byref(y),
        ctypes.byref(w),
        ctypes.byref(h),
        ctypes.byref(border),
        ctypes.byref(depth),
    )
    return w.value, h.value


def top_level():
    """The app window is the largest mapped child of the root display (no WM here)."""
    root = X11.XDefaultRootWindow(drive.display())
    root_return = ctypes.c_ulong()
    parent = ctypes.c_ulong()
    children = ctypes.POINTER(ctypes.c_ulong)()
    count = ctypes.c_uint()
    X11.XQueryTree(
        drive.display(), root, ctypes.byref(root_return), ctypes.byref(parent), ctypes.byref(children), ctypes.byref(count)
    )
    if count.value == 0:
        sys.exit("no application window found")
    best = max((children[i] for i in range(count.value)), key=lambda window: geometry(window)[0] * geometry(window)[1])
    print(f"windows: {[(children[i], geometry(children[i])) for i in range(count.value)]}")
    return best


def resize(width, height):
    window = top_level()
    X11.XResizeWindow(drive.display(), window, int(width), int(height))
    X11.XFlush(drive.display())
    time.sleep(1.5)  # let the webview relayout and settle its media query
    print(f"resized window {window} to {width}x{height}")


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
                "title": f"Synthetic role {i}",
                "company": "Example Employer",
                "location": "Testville",
                "start": {"year": 2010 + i, "month": 5},
                "end": {"year": 2011 + i, "month": 4},
                "bullets": [
                    f"Authored bullet {i}-{b}: a measurable synthetic achievement written by the fixture author."
                    for b in range(5)
                ],
            }
            for i in range(12)
        ],
        "education": [
            {
                "school": "Test University",
                "degree": "BSc Testing",
                "location": "Testville",
                "start": {"year": 2006, "month": 8},
                "end": {"year": 2009, "month": 5},
                "bullets": ["Authored education bullet."],
            }
        ],
        "skills": [
            {"name": f"Synthetic skill group {s}", "bullets": [f"Authored skill bullet {s}-{b}" for b in range(3)]}
            for s in range(4)
        ],
        "projects": [{"name": "Synthetic project", "detail": "Local tooling", "bullets": ["Authored project bullet."]}],
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
    PROJECTS.mkdir(parents=True, exist_ok=True)
    for path in PROJECTS.iterdir():
        if path.is_file():
            path.unlink()
    (PROJECTS / "t09fresh.json").write_text(
        json.dumps(project("t09fresh", "Freshness demo", "Synthetic Candidate", "Synthetic summary for ticket 09.", 1763270600), indent=2)
        + "\n"
    )
    (PROJECTS / "t09other.json").write_text(
        json.dumps(project("t09other", "Neighbour CV", "Grace Hopper", "Neighbour CV stays put.", 1763270500), indent=2)
        + "\n"
    )
    print(f"seeded {PROJECTS}")


def check():
    print(f"PROJECTS: {PROJECTS}")
    for path in sorted(PROJECTS.iterdir()):
        if not path.is_file():
            continue
        data = json.loads(path.read_text())
        summary = data.get("cv", {}).get("summary", "<none>")
        print(f"  {path.name} size={path.stat().st_size} id={data.get('id')!r} summary={summary!r}")


def wheel(x, y, clicks):
    """Scroll the pane under (x, y): positive scrolls down, negative up."""
    D = drive.display()
    button = 5 if int(clicks) > 0 else 4
    drive.XTST.XTestFakeMotionEvent(D, -1, int(x), int(y), 0)
    drive.X11.XFlush(D)
    time.sleep(0.1)
    for _ in range(abs(int(clicks))):
        drive.XTST.XTestFakeButtonEvent(D, button, 1, 0)
        drive.XTST.XTestFakeButtonEvent(D, button, 0, 0)
        drive.X11.XFlush(D)
        time.sleep(0.05)
    time.sleep(0.6)


command = sys.argv[1]
if command == "seed":
    seed()
elif command == "check":
    check()
elif command == "resize":
    resize(sys.argv[2], sys.argv[3])
elif command == "wheel":
    wheel(sys.argv[2], sys.argv[3], sys.argv[4])
elif command == "tools":
    for tool in ("pdftotext", "pdfinfo", "sha256sum"):
        print(tool, subprocess.run(["which", tool], capture_output=True, text=True).stdout.strip())
else:
    drive.main()
