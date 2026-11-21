"""CS-04: discard one actual admitted response to a public package consumer."""

import http.client
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import ipaddress
import json
from pathlib import Path
import socket
import threading
from urllib.parse import urlsplit


class ResponseLossProxy:
    """Own a fresh loopback listener; retain upstream admission, never retry writes."""

    MAX_BYTES = 2 * 1024 * 1024
    HOP_HEADERS = {"connection", "keep-alive", "proxy-authenticate", "proxy-authorization",
                   "te", "trailer", "transfer-encoding", "upgrade", "host", "content-length"}

    def __init__(self, upstream_url, evidence):
        upstream = urlsplit(upstream_url)
        if (upstream.scheme != "http" or not upstream.port
                or not ipaddress.ip_address(upstream.hostname).is_loopback
                or upstream.path not in ("", "/") or upstream.username or upstream.password
                or upstream.query or upstream.fragment):
            raise ValueError("fault upstream must be a numeric loopback HTTP origin")
        self.upstream = upstream
        self.origin = upstream_url.rstrip("/")
        self.evidence = Path(evidence)
        self.evidence.mkdir(parents=True, exist_ok=False)
        self.lock = threading.Lock()
        self.dropped = threading.Event()
        self.sequence = 0
        owner = self

        class Handler(BaseHTTPRequestHandler):
            def do_GET(self):
                owner.forward(self)

            def do_POST(self):
                owner.forward(self)

            def log_message(self, *_args):
                pass

        self.server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        self.server.daemon_threads = False
        self.thread = threading.Thread(target=self.server.serve_forever, name="cs04-response-loss")
        self.url = "http://127.0.0.1:" + str(self.server.server_port)

    def __enter__(self):
        self.thread.start()
        return self

    def __exit__(self, *_args):
        self.server.shutdown()
        self.server.server_close()
        self.thread.join(timeout=5)
        if self.thread.is_alive():
            raise RuntimeError("owned response-loss listener did not stop")

    def forward(self, handler):
        with self.lock:
            self.sequence += 1
            sequence = self.sequence
        record = {"method": handler.command, "path": handler.path, "upstream": self.origin}
        connection = None
        handler.connection.settimeout(15)
        try:
            if handler.headers.get("Transfer-Encoding"):
                handler.send_error(400, "bounded Content-Length body required")
                return
            length = int(handler.headers.get("Content-Length", "0"))
            if not 0 <= length <= self.MAX_BYTES:
                handler.send_error(413)
                return
            body = handler.rfile.read(length)
            if len(body) != length:
                raise ValueError("incomplete client request body")
            record["request_body"] = body.decode("utf-8")
            headers = {key: self.origin if key.lower() == "origin" else value for key, value in handler.headers.items()
                       if key.lower() not in self.HOP_HEADERS}
            connection = http.client.HTTPConnection(self.upstream.hostname, self.upstream.port, timeout=15)
            connection.request(handler.command, handler.path, body=body, headers=headers)
            response = connection.getresponse()
            data = response.read(self.MAX_BYTES + 1)
            if len(data) > self.MAX_BYTES:
                raise ValueError("unbounded upstream response")
            record.update(upstream_status=response.status, upstream_body=data.decode("utf-8"))
            with self.lock:
                drop = (handler.command == "POST" and response.status == 202
                        and urlsplit(handler.path).path.startswith("/api/v1/agents/")
                        and urlsplit(handler.path).path.endswith("/messages") and not self.dropped.is_set())
                if drop:
                    self.dropped.set()
            record["downstream_response_lost"] = drop
            if drop:
                handler.close_connection = True
                handler.connection.shutdown(socket.SHUT_RDWR)
                return
            handler.send_response(response.status)
            for key, value in response.getheaders():
                if key.lower() not in self.HOP_HEADERS:
                    handler.send_header(key, value)
            handler.send_header("Content-Length", str(len(data)))
            handler.end_headers()
            handler.wfile.write(data)
        except Exception as error:
            record["transport_error"] = str(error)
            handler.close_connection = True
        finally:
            if connection is not None:
                connection.close()
            (self.evidence / f"proxy-{sequence:04d}.json").write_text(json.dumps(record, indent=2) + "\n")
