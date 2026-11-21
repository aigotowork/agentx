#!/usr/bin/env python3
"""Bounded standalone embedded-resource and numeric-LAN admission evidence."""
import argparse
import hashlib
import http.client
import json
from pathlib import Path
import re
from urllib.parse import urlsplit


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("origin"); parser.add_argument("evidence", type=Path)
    parser.add_argument("--diagnostic", action="store_true")
    args = parser.parse_args(); endpoint = urlsplit(args.origin); args.evidence.mkdir()
    receipts = []
    def request(path, expected, method="GET", body=None, origin=None):
        client = http.client.HTTPConnection(endpoint.hostname, endpoint.port, timeout=15)
        headers = {} if method == "GET" else {"Content-Type": "application/json", "Origin": origin or args.origin}
        try:
            client.request(method, path, body=body, headers=headers); response = client.getresponse(); data = response.read()
            receipt = {"method": method, "path": path, "status": response.status, "headers": dict(response.getheaders()), "bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()}
            receipts.append(receipt)
            (args.evidence / f"response-{len(receipts):02}.bin").write_bytes(data)
            (args.evidence / "receipts.json").write_text(json.dumps(receipts, indent=2) + "\n")
            assert response.status == expected, receipt
            assert response.getheader("cache-control") == "no-store", receipt
            assert response.getheader("content-security-policy"), receipt
            return data, receipt
        finally:
            client.close()
    html, _ = request("/", 200)
    assets = re.findall(r'(?:src|href)="([^"#]+\.(?:js|css))"', html.decode())
    assert len(assets) >= (1 if args.diagnostic else 2), html[:1500]
    # The unchanged diagnostic entry loads its stylesheet after connection.
    if args.diagnostic:
        assets.append('/app.css')
    for asset in assets:
        data, receipt = request(asset, 200)
        assert data
        assert ("javascript" if asset.endswith(".js") else "css") in receipt["headers"]["content-type"]
    request("/not-in-embedded-manifest.js", 404)
    route = "/api/v1/scope" if args.diagnostic else "/api/v1/connection"
    data, _ = request(route, 200, "POST", "{}")
    descriptor = json.loads(data)
    request(route, 403, "POST", "{}", "http://192.0.2.1:12345")
    if not args.diagnostic:
        assert descriptor["available"] and descriptor["scope"] and descriptor["project"]
        request("/api/v1/read", 400, "POST", json.dumps({"generation": descriptor["generation"], "read": {"List": {"limit": 64, "after": None}}}))
    print(json.dumps({"origin": args.origin, "embedded_assets": assets, "checks": len(receipts), "diagnostic": args.diagnostic}))


if __name__ == "__main__":
    main()
