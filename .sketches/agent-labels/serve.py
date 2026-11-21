"""Serve only the illustrative labels page and its owning design on the LAN."""

from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import urlsplit

ROOT = Path(__file__).resolve().parents[2]
ROUTES = {
    "/": (ROOT / ".sketches/agent-labels/index.html", "text/html; charset=utf-8"),
    "/.sketches/agent-labels/": (ROOT / ".sketches/agent-labels/index.html", "text/html; charset=utf-8"),
    "/.docs/agent-classification.md": (ROOT / ".docs/agent-classification.md", "text/plain; charset=utf-8"),
}


class PreviewHandler(BaseHTTPRequestHandler):
    def do_GET(self):
        route = ROUTES.get(urlsplit(self.path).path)
        if route is None:
            self.send_error(404)
            return
        path, mime = route
        body = path.read_bytes()
        self.send_response(200)
        self.send_header("Content-Type", mime)
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Cache-Control", "no-store")
        self.end_headers()
        self.wfile.write(body)


if __name__ == "__main__":
    ThreadingHTTPServer(("0.0.0.0", 18967), PreviewHandler).serve_forever()
