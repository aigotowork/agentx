"""Owned loopback static host maps /space/ to the immutable external build."""
from http.server import ThreadingHTTPServer, SimpleHTTPRequestHandler
import sys
from urllib.parse import urlsplit
class Handler(SimpleHTTPRequestHandler):
    def translate_path(self, path):
        if urlsplit(path).path.startswith('/space/'):
            path = path[len('/space'):]
        return super().translate_path(path)
    def log_message(self, *_):
        pass
ThreadingHTTPServer(('127.0.0.1', int(sys.argv[2])), lambda *a, **k: Handler(*a, directory=sys.argv[1], **k)).serve_forever()
