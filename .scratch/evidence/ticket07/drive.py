#!/usr/bin/env python3
"""Ticket 07 driver: ticket 06's XTEST helper with its own isolated library.

Adds `close` (a real WM_DELETE_WINDOW client message, the same request a window
manager sends when the user presses the title-bar close button) and `windows`.

Usage: drive.py <command> [...]
  xvfb | start | restart | stop | shot <name> | click <x> <y> | move <x> <y>
  tab <n> | stab <n> | enter | key <keysym> | type <text> | typewait <text> [delay]
  wheel <clicks> | close | windows | seed | check
"""
import ctypes
import importlib.util
import json
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("drive", HERE.parent / "ticket02" / "drive.py")
drive = importlib.util.module_from_spec(spec)
spec.loader.exec_module(drive)

drive.SHOTS = HERE
drive.DATA = pathlib.Path("/home/airzy/.hermes/cache/scratch/cvgen-ticket07")
PROJECTS = drive.DATA / "data/app.cvgen.desktop/projects"

X11 = drive.X11
X11.XDefaultRootWindow.restype = ctypes.c_ulong
X11.XDefaultRootWindow.argtypes = [ctypes.c_void_p]
X11.XQueryTree.argtypes = [
    ctypes.c_void_p,
    ctypes.c_ulong,
    ctypes.POINTER(ctypes.c_ulong),
    ctypes.POINTER(ctypes.c_ulong),
    ctypes.POINTER(ctypes.POINTER(ctypes.c_ulong)),
    ctypes.POINTER(ctypes.c_uint),
]
X11.XFetchName.argtypes = [ctypes.c_void_p, ctypes.c_ulong, ctypes.POINTER(ctypes.c_char_p)]
X11.XFree.argtypes = [ctypes.c_void_p]
X11.XInternAtom.restype = ctypes.c_ulong
X11.XInternAtom.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_int]
X11.XSendEvent.argtypes = [ctypes.c_void_p, ctypes.c_ulong, ctypes.c_int, ctypes.c_long, ctypes.c_void_p]


class ClientData(ctypes.Union):
    _fields_ = [("b", ctypes.c_char * 20), ("s", ctypes.c_short * 10), ("l", ctypes.c_long * 5)]


class ClientMessage(ctypes.Structure):
    _fields_ = [
        ("type", ctypes.c_int),
        ("serial", ctypes.c_ulong),
        ("send_event", ctypes.c_int),
        ("display", ctypes.c_void_p),
        ("window", ctypes.c_ulong),
        ("message_type", ctypes.c_ulong),
        ("format", ctypes.c_int),
        ("data", ClientData),
    ]


class XEvent(ctypes.Union):
    _fields_ = [("xclient", ClientMessage), ("pad", ctypes.c_long * 24)]


def windows():
    drive.display()
    root = X11.XDefaultRootWindow(drive.D)
    returned, parent = ctypes.c_ulong(), ctypes.c_ulong()
    kids = ctypes.POINTER(ctypes.c_ulong)()
    count = ctypes.c_uint()
    X11.XQueryTree(drive.D, root, ctypes.byref(returned), ctypes.byref(parent), ctypes.byref(kids), ctypes.byref(count))
    found = []
    for index in range(count.value):
        window = kids[index]
        name = ctypes.c_char_p()
        if X11.XFetchName(drive.D, window, ctypes.byref(name)) and name.value:
            found.append((window, name.value.decode()))
            X11.XFree(name)
    return found


def close():
    """Ask the app to close exactly like a window manager's close button."""
    drive.display()
    targets = [w for w, name in windows() if "CV Generator" in name or name.startswith("probe:")]
    if not targets:
        sys.exit(f"no CV Generator window: {windows()}")
    protocols = X11.XInternAtom(drive.D, b"WM_PROTOCOLS", False)
    delete = X11.XInternAtom(drive.D, b"WM_DELETE_WINDOW", False)
    event = XEvent()
    event.xclient.type = 33  # ClientMessage
    event.xclient.window = targets[0]
    event.xclient.message_type = protocols
    event.xclient.format = 32
    event.xclient.data.l[0] = delete
    event.xclient.data.l[1] = 0  # CurrentTime
    X11.XSendEvent(drive.D, targets[0], False, 0, ctypes.byref(event))
    X11.XFlush(drive.D)
    print(f"sent WM_DELETE_WINDOW to {targets[0]}")


def seed():
    """One complete synthetic CV, written once so a save creates its previous revision."""
    cv = {
        "name": "Synthetic Candidate",
        "email": "candidate@example.invalid",
        "phone": "",
        "location": "Testville",
        "links": [{"label": "Portfolio", "url": "https://example.invalid/portfolio"}],
        "summary": "First saved revision stays here.",
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
        "id": "t07source",
        "title": "Recovery demo",
        "structure": "reverse-chronological",
        "content_language": "english",
        "updated_at": 1763270400,
        "cv": cv,
    }
    PROJECTS.mkdir(parents=True, exist_ok=True)
    for path in PROJECTS.iterdir():
        if path.is_file():
            path.unlink()
    path = PROJECTS / "t07source.json"
    path.write_text(json.dumps(project, indent=2) + "\n")
    print(f"seeded {path}")


def check():
    for path in sorted(PROJECTS.iterdir()):
        if path.suffix != ".json":
            continue
        print(f"--- {path.name} size={path.stat().st_size}")
        try:
            data = json.loads(path.read_text())
        except json.JSONDecodeError:
            print("   <unreadable>", path.read_text()[:60].replace("\n", " "))
            continue
        if "cv" not in data:
            print("   <no cv>")
            continue
        print(
            f"   id={data['id']} title={data['title']!r} summary={data['cv']['summary']!r} "
            f"updated_at={data['updated_at']}"
        )


command = sys.argv[1]
if command == "seed":
    seed()
elif command == "check":
    check()
elif command == "close":
    drive.display()
    close()
elif command == "windows":
    drive.display()
    print(windows())
else:
    drive.main()
