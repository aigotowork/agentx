"""Run under xvfb-run using system Python with GTK3, VTE 2.91 and XTest."""
import argparse
import ctypes
from pathlib import Path
import time

import gi

gi.require_version("Gdk", "3.0")
gi.require_version("Gtk", "3.0")
gi.require_version("Vte", "2.91")
from gi.repository import Gdk, GLib, Gtk, Vte
from smoke import fixture


def pump():
    while Gtk.events_pending():
        Gtk.main_iteration_do(False)


def until(probe):
    deadline = time.monotonic() + 10
    while time.monotonic() < deadline:
        pump()
        result = probe()
        if result:
            return result
        time.sleep(0.015)
    raise AssertionError(f"VTE did not reach {probe.__name__}")


class Pointer:
    """XTest sends pointer gestures to the actual terminal widget through X11."""

    def __init__(self, terminal):
        self.terminal = terminal
        self.x11 = ctypes.CDLL("libX11.so.6")
        self.xt = ctypes.CDLL("libXtst.so.6")
        self.x11.XOpenDisplay.restype = ctypes.c_void_p
        self.display = ctypes.c_void_p(self.x11.XOpenDisplay(None))
        if not self.display:
            raise RuntimeError("Run under xvfb-run with GDK_BACKEND=x11")

    def move(self, x, y):
        _, origin_x, origin_y = self.terminal.get_window().get_origin()
        width, height = self.terminal.get_char_width(), self.terminal.get_char_height()
        self.xt.XTestFakeMotionEvent(self.display, -1, origin_x + 1 + x * width + width // 2,
                                     origin_y + 1 + y * height + height // 2, 0)
        self.flush()

    def button(self, pressed):
        self.xt.XTestFakeButtonEvent(self.display, 1, int(pressed), 0)
        self.flush()

    def shift(self, pressed):
        shift = self.x11.XKeysymToKeycode(self.display, 0xffe1)
        self.xt.XTestFakeKeyEvent(self.display, shift, int(pressed), 0)
        self.flush()

    def flush(self):
        self.x11.XSync(self.display, 0)
        pump()

    def click(self, cell):
        self.move(*cell)
        self.button(True)
        self.button(False)

    def drag(self, cell, count, shift=False):
        x, y = cell
        if shift:
            self.shift(True)
        self.move(x, y)
        self.button(True)
        for i in range(1, count + 1):
            self.move(x + i, y)
        self.button(False)
        if shift:
            self.shift(False)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    binary = args.binary.resolve(strict=True)
    evidence = args.output.resolve()
    evidence.mkdir(parents=True, exist_ok=True)
    window = Gtk.Window()
    terminal = Vte.Terminal()
    window.add(terminal)
    terminal.set_size(100, 28)
    window.show_all()
    terminal.grab_focus()
    pump()

    def cell_of(label):
        rows = (terminal.get_text_format(Vte.Format.TEXT) or "").splitlines()
        for row, line in enumerate(rows):
            if label in line:
                # The matched labels and their prefixes in this fixture are ASCII
                # or single-cell punctuation.
                return line.index(label), row
        return None

    def shot(name):
        clock = terminal.get_frame_clock()
        painted = []
        handler = clock.connect("after-paint", lambda *_: painted.append(True))
        try:
            terminal.queue_draw()
            until(lambda: painted)
        finally:
            clock.disconnect(handler)
        surface = terminal.get_window()
        surface.get_display().sync()
        image = Gdk.pixbuf_get_from_window(surface, 0, 0, surface.get_width(), surface.get_height())
        image.savev(str(evidence / (name + ".png")), "png", [], [])

    def selected():
        return (terminal.get_text_selected(Vte.Format.TEXT) or "").strip()

    try:
        with fixture(binary, evidence) as (root, env):
            sequence = ('"$1"; echo POST_EXIT_SELECTION; read -r line; '
                        '"$1" launch --tool --dry-run > preview.txt && echo PICKER_RETURNED; read -r line')
            args = ["/usr/bin/env", "-i", *(f"{k}={v}" for k, v in env.items()),
                    "/bin/sh", "-c", sequence, "sh", str(binary)]
            started, _ = terminal.spawn_sync(Vte.PtyFlags.DEFAULT, str(root), args, None, GLib.SpawnFlags.DEFAULT, None, None, None)
            assert started
            pointer = Pointer(terminal)
            until(lambda: cell_of("No agents running"))
            shot("dashboard")
            # VTE delegates a modified drag to its own text selection while capture is on.
            pointer.drag(cell_of("No agents running"), 8, shift=True)
            until(lambda: "agents" in selected())
            active_selection = selected()
            shot("native-selection")
            pointer.click(until(lambda: cell_of("help")))
            until(lambda: cell_of("select all"))
            shot("help-click")
            terminal.feed_child(b"\x1b")
            until(lambda: cell_of("No agents running"))
            terminal.feed_child(b"\x04")
            marker = until(lambda: cell_of("POST_EXIT_SELECTION"))
            pointer.drag(marker, 8)
            shot("shell-selection")
            assert "EXIT" in selected(), f"Unexpected post-exit selection at {marker}: {selected()!r}; window text: {terminal.get_text_format(Vte.Format.TEXT)!r}"
            (evidence / "selection.txt").write_text(f"Shift-drag inside TUI: {active_selection}\nPlain drag after exit: {selected()}\n")
            terminal.feed_child(b"\n")
            until(lambda: cell_of("Select tool"))
            terminal.feed_child(b"codex")
            until(lambda: cell_of("Search: codex"))
            # Click the result row under the search box, rather than the query.
            rows = terminal.get_text_format(Vte.Format.TEXT).splitlines()
            row = next(y for y, line in enumerate(rows) if "codex" in line.lower() and "Search:" not in line)
            shot("filtered-picker")
            pointer.click((3, row))
            until(lambda: cell_of("PICKER_RETURNED"))
            preview = (root / "preview.txt").read_text()
            assert "Tool: codex" in preview.splitlines() and "\x1b" not in preview, preview
            terminal.feed_child(b"\n")
            (evidence / "failed-screen.txt").unlink(missing_ok=True)
            print(f"VTE selection, click, shell recovery and stderr picker verified: {evidence}")
    except Exception:
        (evidence / "failed-screen.txt").write_text(terminal.get_text_format(Vte.Format.TEXT) or "")
        raise
    finally:
        window.destroy()
        pump()


if __name__ == "__main__":
    main()
