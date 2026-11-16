#!/usr/bin/env python3
"""Keyboard/mouse driver for headless CV Generator evidence (XTEST, no xdotool).

Usage: drive.py <command> [args...]
  start | restart | stop | shot <name> | click <x> <y>
  tab <n> | stab <n> | enter | key <keysym> | type <text>
  check            print the stored project files as JSON
"""
import ctypes
import json
import os
import pathlib
import signal
import subprocess
import sys
import time

DISPLAY = ":99"
ROOT = pathlib.Path(__file__).resolve().parents[3]
BIN = ROOT / "src-tauri/target/debug/cv-generator"
DATA = pathlib.Path("/home/airzy/.hermes/cache/scratch/cvgen-e2e")
SHOTS = pathlib.Path(__file__).resolve().parent

X11 = ctypes.CDLL("libX11.so.6")
XTST = ctypes.CDLL("libXtst.so.6")
X11.XOpenDisplay.restype = ctypes.c_void_p
X11.XOpenDisplay.argtypes = [ctypes.c_char_p]
X11.XStringToKeysym.restype = ctypes.c_ulong
X11.XStringToKeysym.argtypes = [ctypes.c_char_p]
X11.XKeysymToKeycode.restype = ctypes.c_ubyte
X11.XKeysymToKeycode.argtypes = [ctypes.c_void_p, ctypes.c_ulong]
X11.XFlush.argtypes = [ctypes.c_void_p]
XTST.XTestFakeMotionEvent.argtypes = [ctypes.c_void_p, ctypes.c_int, ctypes.c_int, ctypes.c_int, ctypes.c_ulong]
XTST.XTestFakeButtonEvent.argtypes = [ctypes.c_void_p, ctypes.c_uint, ctypes.c_int, ctypes.c_ulong]
XTST.XTestFakeKeyEvent.argtypes = [ctypes.c_void_p, ctypes.c_uint, ctypes.c_int, ctypes.c_ulong]

D = None


def display():
    global D
    if D is None:
        D = X11.XOpenDisplay(DISPLAY.encode())
        if not D:
            sys.exit(f"cannot open display {DISPLAY}")
    return D


def flush(pause=0.04):
    X11.XFlush(D)
    time.sleep(pause)


def _code(keysym_name):
    return X11.XKeysymToKeycode(D, X11.XStringToKeysym(keysym_name.encode()))


def tap(name, shift=False):
    code = _code(name)
    if not code:
        raise SystemExit(f"no keycode for {name!r}")
    if shift:
        XTST.XTestFakeKeyEvent(D, _code("Shift_L"), 1, 0)
    XTST.XTestFakeKeyEvent(D, code, 1, 0)
    XTST.XTestFakeKeyEvent(D, code, 0, 0)
    if shift:
        XTST.XTestFakeKeyEvent(D, _code("Shift_L"), 0, 0)
    flush(0.02)


SHIFTED = {"@": "at", "_": "underscore", ":": "colon", "–": None}
PLAIN = {" ": "space", "/": "slash", ".": "period", "-": "minus", ",": "comma", "(": "parenleft", ")": "parenright", "'": "apostrophe"}


def type_text(text):
    for ch in text:
        if ch in SHIFTED:
            tap(SHIFTED[ch], shift=True)
        elif ch in PLAIN:
            tap(PLAIN[ch])
        elif ch.isdigit():
            tap(ch)
        elif ch.isupper():
            tap(ch.lower(), shift=True)
        else:
            tap(ch)


def click(x, y):
    XTST.XTestFakeMotionEvent(D, -1, int(x), int(y), 0)
    flush()
    XTST.XTestFakeButtonEvent(D, 1, 1, 0)
    flush()
    XTST.XTestFakeButtonEvent(D, 1, 0, 0)
    flush()


def shot(name):
    path = SHOTS / f"{name}.png"
    subprocess.run(["import", "-display", DISPLAY, "-window", "root", str(path)], check=True)
    print(f"shot={path}")


def env():
    e = dict(os.environ)
    e.update(
        DISPLAY=DISPLAY,
        XDG_DATA_HOME=str(DATA / "data"),
        WEBKIT_DISABLE_COMPOSITING_MODE="1",
        LIBGL_ALWAYS_SOFTWARE="1",
    )
    return e


def start_app():
    DATA.mkdir(parents=True, exist_ok=True)
    log = DATA / "app.log"
    out = open(log, "ab")
    proc = subprocess.Popen([str(BIN)], env=env(), stdout=out, stderr=out, start_new_session=True)
    (DATA / "app.pid").write_text(str(proc.pid))
    print(f"app pid={proc.pid} log={log}")
    time.sleep(4)
    shot("live")


def kill_app():
    pidfile = DATA / "app.pid"
    if not pidfile.exists():
        return
    pid = int(pidfile.read_text())
    try:
        os.kill(pid, signal.SIGTERM)
    except ProcessLookupError:
        pass
    time.sleep(1.5)
    pidfile.unlink(missing_ok=True)


def xvfb():
    if pathlib.Path("/tmp/.X11-unix/X99").exists():
        print("Xvfb :99 already running")
        return
    subprocess.Popen(
        ["Xvfb", DISPLAY, "-screen", "0", "1400x1000x24"],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        start_new_session=True,
    )
    time.sleep(2)
    print("Xvfb :99 started")


def check():
    projects = sorted((DATA / "data/app.cvgen.desktop/projects").glob("*.json"))
    print(f"files={len(projects)}")
    for path in projects:
        data = json.loads(path.read_text())
        print(f"--- {path.name}")
        print(json.dumps({k: v for k, v in data.items() if k != "cv"}, indent=1))
        print(f"cv.name={data['cv']['name']!r} summary={data['cv']['summary']!r}")
        print(f"experience={json.dumps(data['cv']['experience'])}")


def main():
    global D
    cmd = sys.argv[1]
    if cmd == "xvfb":
        xvfb()
        return
    if cmd == "start":
        xvfb()
        D = display()
        start_app()
        return
    D = display()
    if cmd == "restart":
        kill_app()
        start_app()
    elif cmd == "stop":
        kill_app()
    elif cmd == "shot":
        shot(sys.argv[2])
    elif cmd == "click":
        click(sys.argv[2], sys.argv[3])
    elif cmd == "tab":
        for _ in range(int(sys.argv[2])):
            tap("Tab")
            time.sleep(float(sys.argv[3]) if len(sys.argv) > 3 else 0)
    elif cmd == "stab":
        for _ in range(int(sys.argv[2])):
            tap("Tab", shift=True)
    elif cmd == "enter":
        tap("Return")
    elif cmd == "key":
        tap(sys.argv[2])
    elif cmd == "ctrl":
        XTST.XTestFakeKeyEvent(D, _code("Control_L"), 1, 0)
        tap(sys.argv[2])
        XTST.XTestFakeKeyEvent(D, _code("Control_L"), 0, 0)
        flush()
    elif cmd == "type":
        type_text(sys.argv[2])
    elif cmd == "typewait":
        delay = float(sys.argv[3]) if len(sys.argv) > 3 else 0.12
        for ch in sys.argv[2]:
            type_text(ch)
            time.sleep(delay)
    elif cmd == "check":
        check()
    else:
        sys.exit(f"unknown command {cmd}")


if __name__ == "__main__":
    main()
