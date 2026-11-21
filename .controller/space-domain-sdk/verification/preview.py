#!/usr/bin/env python3
"""Keep a native, isolated Space available for hands-on review after its browser check."""
import argparse
from ipaddress import IPv4Address
import json
from pathlib import Path
import signal
import subprocess
import sys
from threading import Event
from urllib.parse import urlencode

REPO = Path(__file__).resolve().parents[3]
CHECK = REPO / '.controller/space-ui-styles/style-library/native.browser.mjs'
sys.path.insert(0, str(REPO / 'tests/live_ui_integration'))
from acceptance import Cases
from live_fixture import LiveFixture


class PreviewFixture(LiveFixture):
    def prepare(self, name, via_cli=False):
        self.configure(name)
        config = self.root / name / 'scope/config.toml'
        config.write_text(config.read_text().replace('host = "127.0.0.1"\nport = 0', f'host = "{self.host}"\nport = {self.port}', 1))
        return self.start(name)

    def connect(self, name, project=None):
        return super().connect(name, project, expected_host=self.host)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('candidate', type=Path)
    parser.add_argument('evidence', type=Path)
    parser.add_argument('--host', type=IPv4Address, default='127.0.0.1')
    parser.add_argument('--port', type=int, required=True)
    parser.add_argument('--hours', type=int, default=2, help='hours to keep the preview open after verification')
    args = parser.parse_args()
    if args.host.is_unspecified or not (args.host.is_private or args.host.is_loopback):
        parser.error('Use a single local or private LAN interface')
    if not 1024 <= args.port <= 65535:
        parser.error('Use a dedicated preview port in 1024..65535')
    if not 1 <= args.hours <= 24:
        parser.error('Preview duration must be between 1 and 24 hours')
    lifetime = args.hours * 3600

    stop = Event()
    for event in (signal.SIGINT, signal.SIGTERM):
        signal.signal(event, lambda *_: stop.set())
    fixture = PreviewFixture(args.candidate, args.evidence)
    fixture.host, fixture.port = args.host, args.port
    try:
        # Extend only this private copy for hands-on use. The harness and Provider behavior are unchanged.
        qoder = fixture.root / 'bin/qoder'
        source = qoder.read_text()
        deadline = 'deadline = time.monotonic() + 900'
        assert source.count(deadline) == 1
        qoder.write_text(source.replace(deadline, f'deadline = time.monotonic() + {lifetime + 300}'))

        cases = Cases(fixture, setup_only=True)
        cases.launch()
        scope, office = str(fixture.root / 'primary/scope'), str(cases.paths['child'])
        url = cases.http.url.rstrip('/') + '/space/'
        config = fixture.evidence / 'browser-config.json'
        config.write_text(json.dumps(dict(url=url, workspace=office, reference=cases.reference,
                                          generation=cases.http.generation, scope=scope, embedded=True)))
        subprocess.run(['node', str(CHECK), str(config)], cwd=REPO, check=True, timeout=140)
        address = url + '?' + urlencode({'office': office, 'style': 'classic'})
        (fixture.evidence / 'ready.json').write_text(json.dumps({'url': address, 'agent': cases.reference['name'],
                                                                'lifetime_seconds': lifetime}))
        print(address, flush=True)
        stop.wait(lifetime)
    finally:
        fixture.close()


if __name__ == '__main__':
    main()
