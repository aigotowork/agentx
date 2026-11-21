"""Finite real-HTTP driver consuming the native workbench declarations.

Wire/domain shapes belong to src/collaboration/{service.rs,web/api.rs}; this
driver accepts their JSON values and retains actual traffic without defining
another editable DTO. Python integers retain native precision exactly.
"""

import http.client
import json
from pathlib import Path
from urllib.parse import urlsplit


def encode(value):
    if isinstance(value, bool) or value is None or isinstance(value, str):
        return value
    if isinstance(value, int):
        return {"$integer": str(value)}
    if isinstance(value, list):
        return [encode(item) for item in value]
    if isinstance(value, dict):
        return {key: encode(item) for key, item in value.items()}
    raise TypeError(f"unsupported native wire value: {type(value).__name__}")


def decode(value):
    if isinstance(value, dict):
        if "$integer" in value:
            if len(value) != 1 or not isinstance(value["$integer"], str):
                raise ValueError("invalid native integer marker")
            return int(value["$integer"])
        return {key: decode(item) for key, item in value.items()}
    if isinstance(value, list):
        return [decode(item) for item in value]
    if isinstance(value, (int, float)) and not isinstance(value, bool):
        raise ValueError("unmarked native numeric value")
    return value


class WorkbenchHttp:
    """One explicit disposable endpoint and new per-request retained receipts."""

    def __init__(self, url, evidence):
        self.endpoint = urlsplit(url)
        if self.endpoint.scheme != "http" or self.endpoint.path not in ("", "/"):
            raise ValueError("supply one exact disposable numeric HTTP origin")
        self.origin = url.rstrip("/")
        self.evidence = Path(evidence)
        self.evidence.mkdir(parents=True, exist_ok=False)

    def post(self, label, route, value, expected_status=200):
        if route not in {"connection", "read", "apply"}:
            raise ValueError("not a released finite route")
        if Path(label).name != label:
            raise ValueError("receipt label must be one path component")
        request = json.dumps(encode(value), ensure_ascii=False, separators=(",", ":"))
        client = http.client.HTTPConnection(self.endpoint.hostname, self.endpoint.port, timeout=20)
        target = self.evidence / f"{label}.json"
        if target.exists():
            raise FileExistsError(target)
        try:
            client.request("POST", f"/api/v1/{route}", body=request.encode(),
                           headers={"Origin": self.origin, "Content-Type": "application/json"})
            response = client.getresponse()
            body = response.read().decode()
            record = {"origin": self.origin, "route": route, "request": request,
                      "status": response.status, "headers": dict(response.getheaders()), "response": body}
            with target.open("x") as receipt:
                receipt.write(json.dumps(record, ensure_ascii=False, indent=2) + "\n")
            if response.status != expected_status:
                raise AssertionError((route, response.status, body[:1500], str(target)))
            return decode(json.loads(body))
        finally:
            client.close()
