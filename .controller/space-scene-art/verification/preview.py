"""Serve only a built Space artifact and finite, read-only Studio Management fixtures."""
from datetime import datetime
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
import argparse
import json
from urllib.parse import parse_qs, urlsplit

SCOPE = "/fixture/agentx-space-studio/.agentx"
GENERATION = "studio-review-1"
CATALOG = "/studio/catalog"
ANNEX = "/studio/annex"
CAST = [
    ("mora", "scout", "来稿联络", "listening"),
    ("luna", "designer", "封面版式", "active"),
    ("riku", "keeper", "版片编号", "error"),
    ("sira", "keeper", "样张归档", "listening"),
    ("tobi", "maker", "试印木刻", "active"),
    ("vera", "maker", "调配色墨", "active"),
]
QUESTIONS = {
    "mora": ("图录里「河岸」那一页还缺一条出处。你这边有收到新版署名吗？", "收到。署名改成「北岸田野组」，版片编号是 R-14。"),
    "luna": ("封面试墨蓝和朱砂两版。", "墨蓝那张的细线更清楚，我在补角标。"),
    "riku": ("请对照 A-06 的木刻版和收件簿。", "两张标签用了同一个编号。年代还不能确认。"),
    "sira": ("本期退下来的试印需要保留一套。", "样张已经按纸纹归成一组。"),
    "tobi": ("把细线图先在旧纸上印一轮。", "网纹没有糊，等墨干了再补标号。"),
    "vera": ("先用半份灰蓝配一张色样。", "色样在晾架上，下一张会再淡一点。"),
}


def role(name):
    return {"name": name, "path": f"{CATALOG}/.agentx/roles/{name}.toml",
            "namespace": "local:/studio/catalog/.agentx/roles", "profile": "studio-sample",
            "description": "图录工坊样例岗位", "default": False}


def agent(index, part, stopped=False):
    name, costume, title, status = part
    status = "stopped" if stopped else status
    return {
        "agent": {"name": name, "incarnation": None if stopped else str(101 + index), "stopped_event": "80" if stopped else None},
        "collaboration": {"status": "adopted", "workspace": CATALOG, "role": role(costume)},
        "status": status, "stored_status": status, "status_context": "", "title": title, "detail": "",
        "directory": f"{CATALOG}/proofs", "tool": "fixture", "session_id": "studio-synthetic",
        "can_send": False, "can_stop": False, "can_resume": False, "has_transcript": False, "pending_messages": 0,
    }


ROWS = [agent(index, part) for index, part in enumerate(CAST)] + [agent(0, CAST[0], True)]


def exact(name, query):
    inc, event = query.get("incarnation"), query.get("stopped_event")
    if bool(inc) == bool(event):
        return None
    return next((row for row in ROWS if row["agent"] == {"name": name, "incarnation": inc, "stopped_event": event}), None)


def payload(path, query):
    version = {"generation": GENERATION}
    if path == "/api/v1/management":
        return 200, {**version, "scope": SCOPE, "project": "/studio", "max_page": 100, "max_text": 65536, "tools": [], "profiles": []}
    if query.get("generation") != GENERATION:
        return 409, {"error": {"code": "StaleReference", "message": "Preview generation changed"}}
    if path == "/api/v1/workspaces":
        return 200, {**version, "workspaces": [{"path": path, "sources": ["role-marker"]} for path in [CATALOG, ANNEX]],
                     "coverage": {"errors": [], "scan_limit_reached": False, "agent_limit_reached": False, "agent_pages": 1, "directories_examined": 2}}
    if path == "/api/v1/agents":
        cursor = query.get("after")
        if cursor not in (None, "studio-3"):
            return 400, {"error": {"code": "InvalidInput", "message": "Unknown fixture cursor"}}
        start = 3 if cursor else 0
        limit = min(3 if query.get("limit") == "3" else 7, len(ROWS) - start)
        end = start + limit
        return 200, {**version, "agents": ROWS[start:end], "next": "studio-3" if end == 3 else None, "current_complete": end >= 6}
    if path == "/api/v1/roles":
        workspace = query.get("workspace")
        return 200, {**version, "workspace": workspace if workspace != "global" else None,
                     "roles": [role(name) for name in ("maker", "designer", "keeper", "scout")] if workspace == CATALOG else []}
    if path == "/api/v1/office-activity":
        return 200, {**version, "created": [], "messages": [], "human_messages": [], "human_unhandled_count": 0,
                     "human_has_more": False, "has_more": False, "next_after": "0"}
    if path == "/api/v1/human-mailbox":
        return 200, {**version, "messages": [], "next_before": None, "unhandled_count": 0}
    bits = path.strip("/").split("/")
    if len(bits) in (4, 5) and bits[:3] == ["api", "v1", "agents"]:
        name = bits[3]
        row = exact(name, query)
        if not row:
            return 404, {"error": {"code": "NotFound", "message": "No exact Studio subject"}}
        if len(bits) == 4:
            return 200, row
        if bits[4] == "launch-relations":
            return 200, {**version, "origin": "unknown", "upstream": None, "downstream": [], "next": None}
        if bits[4] == "messages":
            history = ("上一期寄错的校样是否已经收回？", "旧校样已收回；这一实例到此结束。") if row["agent"]["stopped_event"] else QUESTIONS[name]
            cursor = query.get("after")
            if cursor not in (None, "201", "202"):
                return 400, {"error": {"code": "InvalidInput", "message": "Unknown fixture checkpoint"}}
            start = {None: 0, "201": 1, "202": 2}[cursor]
            messages = []
            for index in range(start, len(history)):
                messages.append({"id": str(201 + index), "at": datetime(2026, 6, 12, 10, 10 + index * 4).isoformat() + "Z",
                                 "from": "bigboss" if index == 0 else name, "recipients": [name if index == 0 else "bigboss"],
                                 "text": history[index], "thread": "图录校样"})
            return 200, {**version, "messages": messages, "has_more": False, "next_after": "202"}
    return 404, {"error": {"code": "NotFound", "message": "Unmodeled preview resource"}}


class Preview(SimpleHTTPRequestHandler):
    test_origin = None
    def do_GET(self):
        url = urlsplit(self.path)
        if not url.path.startswith("/api/"):
            return super().do_GET()
        query = {name: values[-1] for name, values in parse_qs(url.query).items()}
        status, value = payload(url.path, query)
        self.reply(status, value)

    def reply(self, status, value):
        body = json.dumps(value, ensure_ascii=False).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Cache-Control", "no-store")
        if self.test_origin and self.headers.get("Origin") == self.test_origin:
            self.send_header("Access-Control-Allow-Origin", self.test_origin)
            self.send_header("Vary", "Origin")
        self.end_headers()
        self.wfile.write(body)

    def deny(self):
        self.reply(405, {"error": {"code": "Forbidden", "message": "Studio preview is read-only"}})

    do_POST = do_PATCH = do_PUT = do_DELETE = deny

    def list_directory(self, path):
        self.send_error(404)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bind", required=True)
    parser.add_argument("--port", required=True, type=int)
    parser.add_argument("--directory", required=True)
    parser.add_argument("--test-origin")
    arguments = parser.parse_args()
    if arguments.test_origin:
        Preview.test_origin = arguments.test_origin
    with ThreadingHTTPServer((arguments.bind, arguments.port),
                             lambda *args, **kwargs: Preview(*args, directory=arguments.directory, **kwargs)) as server:
        server.serve_forever()


if __name__ == "__main__":
    main()
