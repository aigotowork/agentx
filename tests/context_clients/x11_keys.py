#!/usr/bin/env python3
"""Send actual X11 keys to the disposable Xvfb host, including browser accelerators."""
import ctypes
from ctypes.util import find_library
import os
import sys
import json


class WindowAttributes(ctypes.Structure):
    _fields_ = [(name, kind) for name, kind in [
        ("x", ctypes.c_int), ("y", ctypes.c_int), ("width", ctypes.c_int), ("height", ctypes.c_int),
        ("border_width", ctypes.c_int), ("depth", ctypes.c_int), ("visual", ctypes.c_void_p),
        ("root", ctypes.c_ulong), ("window_class", ctypes.c_int), ("bit_gravity", ctypes.c_int),
        ("win_gravity", ctypes.c_int), ("backing_store", ctypes.c_int), ("backing_planes", ctypes.c_ulong),
        ("backing_pixel", ctypes.c_ulong), ("save_under", ctypes.c_int), ("colormap", ctypes.c_ulong),
        ("map_installed", ctypes.c_int), ("map_state", ctypes.c_int), ("all_event_masks", ctypes.c_long),
        ("your_event_mask", ctypes.c_long), ("do_not_propagate_mask", ctypes.c_long),
        ("override_redirect", ctypes.c_int), ("screen", ctypes.c_void_p)]]

assert os.environ.get("DISPLAY"), "owned Xvfb DISPLAY required"
x11 = ctypes.CDLL(find_library("X11"))
xtst = ctypes.CDLL(find_library("Xtst"))
x11.XOpenDisplay.argtypes = [ctypes.c_char_p]
x11.XOpenDisplay.restype = ctypes.c_void_p
x11.XStringToKeysym.argtypes = [ctypes.c_char_p]
x11.XStringToKeysym.restype = ctypes.c_ulong
x11.XKeysymToKeycode.argtypes = [ctypes.c_void_p, ctypes.c_ulong]
x11.XKeysymToKeycode.restype = ctypes.c_uint
x11.XFlush.argtypes = [ctypes.c_void_p]
x11.XCloseDisplay.argtypes = [ctypes.c_void_p]
x11.XDefaultRootWindow.argtypes = [ctypes.c_void_p]
x11.XDefaultRootWindow.restype = ctypes.c_ulong
x11.XQueryTree.argtypes = [ctypes.c_void_p, ctypes.c_ulong, ctypes.POINTER(ctypes.c_ulong), ctypes.POINTER(ctypes.c_ulong), ctypes.POINTER(ctypes.POINTER(ctypes.c_ulong)), ctypes.POINTER(ctypes.c_uint)]
x11.XFetchName.argtypes = [ctypes.c_void_p, ctypes.c_ulong, ctypes.POINTER(ctypes.c_char_p)]
x11.XSetInputFocus.argtypes = [ctypes.c_void_p, ctypes.c_ulong, ctypes.c_int, ctypes.c_ulong]
x11.XFree.argtypes = [ctypes.c_void_p]
x11.XGetWindowAttributes.argtypes = [ctypes.c_void_p, ctypes.c_ulong, ctypes.POINTER(WindowAttributes)]
xtst.XTestFakeKeyEvent.argtypes = [ctypes.c_void_p, ctypes.c_uint, ctypes.c_int, ctypes.c_ulong]
display = x11.XOpenDisplay(None)
assert display
try:
    windows = []
    root, parent, children, count = ctypes.c_ulong(), ctypes.c_ulong(), ctypes.POINTER(ctypes.c_ulong)(), ctypes.c_uint()
    x11.XQueryTree(display, x11.XDefaultRootWindow(display), ctypes.byref(root), ctypes.byref(parent), ctypes.byref(children), ctypes.byref(count))
    for index in range(count.value):
        name = ctypes.c_char_p()
        if x11.XFetchName(display, children[index], ctypes.byref(name)) and name.value:
            title = name.value.decode(errors="replace")
            attributes = WindowAttributes()
            x11.XGetWindowAttributes(display, children[index], ctypes.byref(attributes))
            windows.append({"id": children[index], "title": title, "mapped": attributes.map_state, "width": attributes.width})
            if ("Chrom" in title or "Google Chrome" in title) and attributes.map_state == 2 and attributes.width > 100:
                x11.XSetInputFocus(display, children[index], 2, 0)
            x11.XFree(name)
    if children:
        x11.XFree(children)
    keys = [x11.XKeysymToKeycode(display, x11.XStringToKeysym(key.encode())) for key in sys.argv[1:]]
    assert keys and all(keys), "unknown X11 keysym"
    for key in keys:
        xtst.XTestFakeKeyEvent(display, key, 1, 0)
    for key in reversed(keys):
        xtst.XTestFakeKeyEvent(display, key, 0, 0)
    x11.XFlush(display)
    print(json.dumps({"windows": windows, "keys": keys}))
finally:
    x11.XCloseDisplay(display)
