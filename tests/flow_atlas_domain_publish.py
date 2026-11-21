#!/usr/bin/env python3
"""Publish accepted new offline examples and one intentional single-artifact server only."""
import hashlib
import json
import os
from pathlib import Path
import socket
import subprocess
import sys
import time
import urllib.error
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
EVIDENCE = Path(sys.argv[1]).resolve()
EXAMPLES = ROOT / 'tools/flow-atlas/examples'


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def identity(pid):
    proc = Path('/proc') / str(pid)
    return {'pid': pid, 'start_ticks': int((proc / 'stat').read_text().split(') ', 1)[1].split()[19]), 'argv': (proc / 'cmdline').read_bytes().replace(b'\x00', b' ').decode(), 'cwd': os.readlink(proc / 'cwd')}


def main():
    baseline = json.loads((EVIDENCE / 'preservation.json').read_text())
    for name, record in baseline['files'].items():
        assert sha(ROOT / name) == record['sha256'], f'Protected baseline changed: {name}'
    for server in baseline['servers']:
        assert identity(server['pid']) == server, f'Existing server changed: {server}'
    published = []
    for name, source, candidate in [
        ('domain-workbench', 'order-stock-binding.json', 'candidate-final.html'),
        ('domain-workbench-technical', 'technical.json', 'technical-final.html'),
    ]:
        model = EXAMPLES / (name + '.json')
        html = EXAMPLES / (name + '.html')
        receipt = html.with_suffix('.receipt.json')
        assert not any(path.exists() for path in [model, html, receipt]), 'Publication must use new artifact names'
        with model.open('xb') as stream:
            stream.write((EVIDENCE / source).read_bytes())
        result = subprocess.run([sys.executable, str(ROOT / 'tools/flow-atlas/workbench/build.py'), str(model), '--output', str(html)], capture_output=True, text=True)
        assert result.returncode == 0, result.stderr
        assert html.read_bytes() == (EVIDENCE / candidate).read_bytes(), 'Published bytes differ from tested candidate'
        data = json.loads(receipt.read_text())
        assert data['sha256'] == sha(html)
        assert data['modelSha256'] == sha(model)
        for relative, digest in data['inputs'].items():
            assert sha(ROOT / 'tools/flow-atlas/workbench' / relative) == digest
        published.append({'model': str(model.relative_to(ROOT)), 'html': str(html.relative_to(ROOT)), 'receipt': str(receipt.relative_to(ROOT)), 'sha256': sha(html), 'bytes': html.stat().st_size})

    probe = socket.socket()
    probe.bind(('0.0.0.0', 18959))
    probe.close()
    artifact = ROOT / published[0]['html']
    argv = [sys.executable, str(ROOT / 'tools/flow-atlas/inspector/serve.py'), str(artifact), '--host', '0.0.0.0', '--port', '18959']
    server_log = (EVIDENCE / 'server.log').open('ab')
    process = subprocess.Popen(argv, cwd=ROOT, stdin=subprocess.DEVNULL, stdout=server_log, stderr=subprocess.STDOUT, start_new_session=True)
    server_log.close()
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
    checks = []
    try:
        for attempt in range(50):
            if process.poll() is not None:
                raise RuntimeError('New server exited during readiness')
            try:
                with opener.open('http://127.0.0.1:18959/', timeout=1) as response:
                    assert response.read() == artifact.read_bytes()
                break
            except urllib.error.URLError:
                time.sleep(0.1)
        else:
            raise RuntimeError('New server did not become ready')
        for base in ['http://127.0.0.1:18959', 'http://192.168.2.178:18959']:
            with opener.open(base + '/', timeout=5) as response:
                body = response.read()
                assert body == artifact.read_bytes()
                checks.append({'url': base + '/', 'sha256': hashlib.sha256(body).hexdigest(), 'bytes': len(body)})
            for path in ['/domain-workbench.json', '/../../package.json', '/examples/']:
                try:
                    opener.open(base + path, timeout=5)
                    raise AssertionError('Non-artifact route was exposed')
                except urllib.error.HTTPError as error:
                    assert error.code == 404
        server = identity(process.pid)
        record = {**server, 'argv': argv, 'observed_argv': server['argv'], 'url': 'http://192.168.2.178:18959/', 'loopback_url': 'http://127.0.0.1:18959/', 'port': 18959, 'artifact': published[0]['html'], 'artifact_sha256': sha(artifact), 'artifact_bytes': artifact.stat().st_size, 'owner': 'independent domain acceptance; intentional read-only single-artifact demo', 'status': 'intentionally running', 'scope': 'offline exported design; no native execution or repository/runtime API', 'evidence': str(EVIDENCE)}
        target = ROOT / '.controller/domain-daemon/flow-atlas-workbench-server.json'
        assert not target.exists()
        target.write_text(json.dumps(record, indent=2) + '\n')
        for name, protected in baseline['files'].items():
            assert sha(ROOT / name) == protected['sha256']
        for old in baseline['servers']:
            assert identity(old['pid']) == old
        report = {'result': 'passed', 'published': published, 'http': checks, 'server': record, 'preservedFiles': len(baseline['files']), 'existingServersPreserved': baseline['servers']}
        (EVIDENCE / 'publication.json').write_text(json.dumps(report, indent=2) + '\n')
        print(json.dumps(report, indent=2))
    except BaseException:
        process.terminate()
        process.wait(timeout=5)
        raise


if __name__ == '__main__':
    main()
