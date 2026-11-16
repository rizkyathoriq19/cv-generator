"""Minimal XTEST driver for headless UI evidence (no xdotool on this host)."""
import ctypes
import sys
import time

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

DISPLAY = X11.XOpenDisplay((sys.argv[1] if len(sys.argv) > 1 else ":99").encode())


def flush():
    X11.XFlush(DISPLAY)
    time.sleep(0.05)


def click(x, y):
    XTST.XTestFakeMotionEvent(DISPLAY, -1, x, y, 0)
    flush()
    XTST.XTestFakeButtonEvent(DISPLAY, 1, 1, 0)
    flush()
    XTST.XTestFakeButtonEvent(DISPLAY, 1, 0, 0)
    flush()


def key(name, shift=False):
    if shift:
        key("Shift_L")
    code = X11.XKeysymToKeycode(DISPLAY, X11.XStringToKeysym(name.encode()))
    XTST.XTestFakeKeyEvent(DISPLAY, code, 1, 0)
    XTST.XTestFakeKeyEvent(DISPLAY, code, 0, 0)
    flush()


def type_text(text):
    for ch in text:
        if ch == "/":
            key("slash")
        elif ch == ".":
            key("period")
        elif ch == "-":
            key("minus")
        elif ch == "_":
            key("underscore", shift=True)
        elif ch.isupper():
            key(ch.lower(), shift=True)
        elif ch == " ":
            key("space")
        else:
            key(ch)


if __name__ == "__main__":
    action = sys.argv[2]
    if action == "click":
        click(int(sys.argv[3]), int(sys.argv[4]))
    elif action == "type":
        type_text(sys.argv[3])
    elif action == "key":
        key(sys.argv[3])
    elif action == "ctrl":
        code = X11.XKeysymToKeycode(DISPLAY, X11.XStringToKeysym(b"Control_L"))
        XTST.XTestFakeKeyEvent(DISPLAY, code, 1, 0)
        flush()
        key(sys.argv[3])
        XTST.XTestFakeKeyEvent(DISPLAY, code, 0, 0)
        flush()
    print("ok")
