#!/usr/bin/env python3
"""Release only recorded fixture resources, then promote verified repository bytes."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import socket
import subprocess

from session import ActualSession, exact_process, process_record


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('evidence', type=Path)
    parser.add_argument('artifact', type=Path)
    args = parser.parse_args()
    evidence = args.evidence.resolve()
    artifact = args.artifact.resolve(strict=True)
    state = json.loads((evidence / 'runtime-resources.json').read_text())
    session = ActualSession(state['owned_root'], evidence)
    retained = json.loads((evidence / 'retained-outcome-manifest.json').read_text())
    assert all(digest(Path(retained['root']) / name) == expected for name, expected in retained['sha256'].items())
    preserved = json.loads((evidence / 'preservation.json').read_text())
    protected = []
    for record in preserved['protected_processes']:
        current = process_record(record['pid'], 'protected-demo')
        assert all(current[key] == record[key] for key in ('pid', 'start_ticks', 'executable'))
        protected.append(current)
    old = next(item for item in preserved['preservation'] if item['source'].endswith('/.bin/agentx'))
    destination = Path(old['source'])
    assert not destination.is_symlink() and digest(destination) == old['sha256']
    assert digest(old['copy']) == old['sha256']
    provider_records = json.loads((evidence / 'provider-process-ownership-resumed.json').read_text())
    for record in provider_records:
        assert not exact_process(record), ('Provider launch root still present', record)
    for label in ('final-stop-mila', 'final-stop-zeno'):
        result = json.loads((evidence / f'{label}.stdout').read_text())
        assert result['ok']
        for outcome in result['results']:
            if outcome.get('pid'):
                assert not Path(f'/proc/{outcome["pid"]}').exists(), ('Provider process still present', outcome)
    browsers = []
    for receipt in sorted(evidence.glob('browser-*/owned-browser-processes.json')):
        for record in json.loads(receipt.read_text()):
            try:
                current = process_record(record['pid'], 'owned-browser')
            except FileNotFoundError:
                continue
            assert any(current[key] != record[key] for key in ('start_ticks', 'executable')), ('Browser still present', record)
        browsers.append(str(receipt.relative_to(evidence)))
    stopped = [session.terminate(record) for record in sorted(state['processes'], key=lambda record: record['role'] == 'fixture-daemon')]
    ports = sorted(set(state['ports']) | {18947, 18948, 18949})
    for port in ports:
        with socket.socket() as client:
            assert client.connect_ex(('127.0.0.1', port)) != 0, ('owned port still listening', port)
    root = session.root
    assert root.parent == Path('/tmp') and root.name.startswith('agentx-workbench-fixture-')
    shutil.rmtree(root)
    assert not root.exists()
    stage = destination.with_name('agentx.workbench-promotion')
    assert not stage.exists()
    shutil.copyfile(artifact, stage)
    stage.chmod(0o755)
    expected = digest(artifact)
    assert digest(stage) == expected
    os.replace(stage, destination)
    assert digest(destination) == expected
    version = subprocess.run([str(destination), '--version'], check=True, capture_output=True, text=True).stdout.strip()
    record = {'artifact': str(artifact), 'repository_artifact': str(destination), 'sha256': expected, 'bytes': destination.stat().st_size, 'version': version, 'prior_artifact': old, 'stopped': stopped, 'owned_ports_closed': ports, 'provider_launches_absent': provider_records, 'browser_receipts_absent': browsers, 'removed_root': str(root), 'protected_demo_unchanged': protected, 'outcome_hashes_verified': len(retained['sha256']), 'limits': 'Repository-local same-byte promotion only. No installed/PATH/default program or persistent LAN service changed. Evidence, frozen candidates and task-local tooling retained.'}
    (evidence / 'final-artifact-cleanup-handoff.json').write_text(json.dumps(record, indent=2) + '\n')
    state['historical_processes'].extend(state['processes'])
    state.update(processes=[], ports=[], status='cleaned-and-repository-artifact-promoted', owned_root_removed=True)
    (evidence / 'runtime-resources.json').write_text(json.dumps(state, indent=2) + '\n')
    print(json.dumps({'sha256': expected, 'artifact': str(destination), 'version': version, 'cleanup': str(root), 'ports_closed': ports}))


if __name__ == '__main__':
    main()
