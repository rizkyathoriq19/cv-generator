#!/usr/bin/env python3
"""Ticket 10 driver: the packaged application on Ubuntu 24.04 and Arch/Omarchy.

Reuses ticket 02's XTEST helper (start/restart/stop/shot/click/tab/stab/enter/
key/type/typewait) and adds `resize`, `wheel`, `seed` and `check`.

Environment overrides:
  CVGEN_BIN   packaged executable (installed deb binary, or extracted AppRun)
  CVGEN_DATA  isolated data root; the app gets CVGEN_DATA/data as XDG_DATA_HOME
  CVGEN_SHOTS screenshot directory (defaults to this directory)

Usage: drive.py <command> [...]
  xvfb | start | restart | stop | shot <name> | click <x> <y>
  tab <n> | stab <n> | enter | key <keysym> | type <text> | typewait <text> [delay]
  resize <width> <height> | wheel <x> <y> <clicks> | seed | seed-id | check
"""
import importlib.util
import json
import os
import pathlib
import sys
import time

HERE = pathlib.Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("drive", HERE.parent / "ticket02" / "drive.py")
drive = importlib.util.module_from_spec(spec)
spec.loader.exec_module(drive)

X11 = drive.X11
import ctypes  # noqa: E402  (after the library probe in ticket 02's helper)

drive.BIN = pathlib.Path(os.environ.get("CVGEN_BIN", drive.BIN))
drive.DATA = pathlib.Path(os.environ.get("CVGEN_DATA", "/home/airzy/.hermes/cache/scratch/cvgen-ticket10"))
drive.SHOTS = pathlib.Path(os.environ.get("CVGEN_SHOTS", HERE))
PROJECTS = drive.DATA / "data/app.cvgen.desktop/projects"

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
    time.sleep(1.5)
    print(f"resized window {window} to {width}x{height}")


def wheel(x, y, clicks):
    D = drive.display()
    button = 5 if int(clicks) > 0 else 4
    drive.XTST.XTestFakeMotionEvent(D, -1, int(x), int(y), 0)
    X11.XFlush(D)
    time.sleep(0.1)
    for _ in range(abs(int(clicks))):
        drive.XTST.XTestFakeButtonEvent(D, button, 1, 0)
        drive.XTST.XTestFakeButtonEvent(D, button, 0, 0)
        X11.XFlush(D)
        time.sleep(0.05)
    time.sleep(0.6)


def cv(name, summary, bullets_per_role, skill_groups, education=True):
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
                    for b in range(bullets_per_role[i])
                ],
            }
            for i in range(len(bullets_per_role))
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
        ]
        if education
        else [],
        "skills": [
            {"name": f"Synthetic skill group {s}", "bullets": [f"Authored skill bullet {s}-{b}" for b in range(n)]}
            for s, n in enumerate(skill_groups)
        ],
        "projects": [{"name": "Synthetic project", "detail": "Local tooling", "bullets": ["Authored project bullet."]}],
        "certifications": [{"name": "Certified tester", "issuer": "Test board", "date": {"year": 2023, "month": None}}],
        "languages": [{"name": "Indonesian", "level": "Native"}],
    }


def project(pid, title, name, summary, updated_at, bullets_per_role, skill_groups, education=True):
    return {
        "version": 1,
        "id": pid,
        "title": title,
        "structure": "reverse-chronological",
        "content_language": "english",
        "updated_at": updated_at,
        "cv": cv(name, summary, bullets_per_role, skill_groups, education),
    }


def seed():
    PROJECTS.mkdir(parents=True, exist_ok=True)
    for path in PROJECTS.iterdir():
        if path.is_file():
            path.unlink()
    fixtures = {
        # Fresh graduate: one education entry, no employment, two skill groups.
        "t10fresh.json": project(
            "t10fresh", "Fresh graduate CV", "Fresh Graduate", "Synthetic fresh-graduate summary for ticket 10.",
            1763270700, [], [2, 1],
        ),
        # Complete: every section populated, two pages of career content.
        "t10complete.json": project(
            "t10complete", "Complete CV", "Complete Candidate", "Synthetic complete summary for ticket 10.",
            1763270600, [2, 2, 2], [2, 2, 2],
        ),
        # Multipage: twelve roles, five bullets each.
        "t10long.json": project(
            "t10long", "Multipage CV", "Synthetic Candidate", "Synthetic multipage summary for ticket 10.",
            1763270500, [5] * 12, [3, 3, 3, 3],
        ),
        "t10other.json": project(
            "t10other", "Keep me", "Grace Hopper", "Neighbour CV stays put.",
            1763270400, [1], [1],
        ),
    }
    for filename, data in fixtures.items():
        (PROJECTS / filename).write_text(json.dumps(data, indent=2) + "\n")
    print(f"seeded {len(fixtures)} projects in {PROJECTS}")


def seed_indonesian():
    """Add the two Indonesian matrix fixtures without touching the other seeds."""
    PROJECTS.mkdir(parents=True, exist_ok=True)
    for pid, title, structure in (
        ("t10idrev", "Indonesian reverse", "reverse-chronological"),
        ("t10idfun", "Indonesian functional", "functional"),
    ):
        data = project(
            pid, title, "Complete Candidate",
            f"Synthetic {structure} summary for ticket 10.",
            1763270900, [2, 2, 2], [2, 2, 2],
        )
        data["structure"] = structure
        data["content_language"] = "indonesian"
        (PROJECTS / f"{pid}.json").write_text(json.dumps(data, indent=2) + "\n")
    print(f"seeded 2 Indonesian fixtures in {PROJECTS}")


def check():
    print(f"PROJECTS: {PROJECTS}")
    for path in sorted(PROJECTS.iterdir()):
        if not path.is_file():
            continue
        try:
            data = json.loads(path.read_text())
        except ValueError:
            print(f"  {path.name} unreadable ({path.stat().st_size} bytes)")
            continue
        print(
            f"  {path.name} size={path.stat().st_size} id={data.get('id')!r} "
            f"structure={data.get('structure')!r} content={data.get('content_language')!r} "
            f"name={data.get('cv', {}).get('name')!r}"
        )


command = sys.argv[1] if len(sys.argv) > 1 else "help"
if command == "seed":
    seed()
elif command == "seed-id":
    seed_indonesian()
elif command == "check":
    check()
elif command == "resize":
    resize(sys.argv[2], sys.argv[3])
elif command == "wheel":
    wheel(sys.argv[2], sys.argv[3], sys.argv[4])
else:
    drive.main()
