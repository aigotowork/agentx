#!/usr/bin/env python3
"""Preserve the exact owned outcome, public role calls and immutable Git inputs."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess

from http_client import WorkbenchHttp


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('evidence', type=Path)
    args = parser.parse_args()
    evidence = args.evidence.resolve()
    resources = json.loads((evidence / 'runtime-resources.json').read_text())
    root = Path(resources['owned_root']).resolve()
    if root.parent != Path('/tmp') or not root.name.startswith('agentx-workbench-fixture-'):
        raise ValueError('not the recorded disposable fixture')
    target = evidence / 'retained-outcome'
    target.mkdir()
    http = WorkbenchHttp(json.loads((evidence / 'runtime-plan.json').read_text())['browser_origin'], target / 'native')
    descriptor = http.post('connection', 'connection', {})
    reference = {key: descriptor[key] for key in ['scope', 'project', 'generation']}
    reference['id'] = resources['work']
    detail = http.post('accepted-work', 'read', {'generation': descriptor['generation'], 'read': {'Detail': reference}})['Read']['Detail']
    assert detail['eligibility']['closed'], 'retain only after the actual explicit decision'
    (target / 'accepted-detail.json').write_text(json.dumps(detail, ensure_ascii=False, indent=2) + '\n')
    after = None
    for page in range(20):
        result = http.post(f'thread-{page:02}', 'read', {'generation': descriptor['generation'], 'read': {'Thread': {'work': reference, 'page': {'limit': 64, 'after': after}}}})['Read']['Thread']
        if result['next'] is None:
            break
        after = result['next']
    else:
        raise RuntimeError('unexpected unbounded fixture thread')
    project = root / 'project'
    subprocess.run(['git', 'bundle', 'create', str(target / 'outcome.bundle'), '--all'], cwd=project, check=True, capture_output=True)
    git_facts = {}
    for label, command in [('status', ['status', '--porcelain=v1']), ('worktrees', ['worktree', 'list', '--porcelain']), ('history', ['log', '--format=%H %s', '--all'])]:
        git_facts[label] = subprocess.run(['git', *command], cwd=project, check=True, text=True, capture_output=True).stdout
    (target / 'git-facts.json').write_text(json.dumps(git_facts, indent=2) + '\n')
    for result in detail['work']['results']:
        commit = result['input']['source']['revision']
        with (target / f'{result["id"]}-source.tar').open('xb') as archive:
            subprocess.run(['git', 'archive', '--format=tar', commit], cwd=project, stdout=archive, check=True)
    # Only actual fixture evidence/intent files; never copy scope or credentials.
    for source in sorted(root.iterdir()):
        if source.is_dir() and source.name.startswith('zeno-evidence-'):
            shutil.copytree(source, target / source.name)
        elif source.is_file() and (source.name.startswith('mila-command-') or source.name == 'mila_report.py'):
            shutil.copy2(source, target / source.name)
    verification = []
    for report_path in sorted(target.glob('zeno-evidence-*/report.json')):
        report = json.loads(report_path.read_text())
        source = subprocess.run(['git', 'show', f'{report["source_commit"]}:export_orders.py'], cwd=project, capture_output=True, check=True).stdout
        assert hashlib.sha256(source).hexdigest() == report['source_sha256']
        report_hash = hashlib.sha256(report_path.read_bytes()).hexdigest()
        matched = [check for check in detail['work']['checks'] if check['input']['observed_fingerprint'] == report['source_commit']]
        assert len(matched) == 1 and any(report_hash in text for text in matched[0]['input']['evidence'])
        assert matched[0]['input']['verdict'].lower() == report['verdict']
        verification.append({'commit': report['source_commit'], 'source_sha256': report['source_sha256'], 'report_sha256': report_hash, 'check_id': matched[0]['id'], 'verdict': report['verdict']})
    assert {item['verdict'] for item in verification} == {'failed', 'passed'}
    (target / 'independent-report-verification.json').write_text(json.dumps(verification, indent=2) + '\n')
    participants = json.loads((evidence / 'provider-session-attribution.stdout').read_text())
    for participant in participants:
        if participant['name'] not in ('mila', 'zeno'):
            continue
        records = []
        for line in Path(participant['transcript_path']).open():
            item = json.loads(line)
            if item.get('type') != 'response_item':
                continue
            payload = item.get('payload', {})
            if payload.get('type') not in ('custom_tool_call', 'custom_tool_call_output'):
                continue
            records.append({'timestamp': item.get('timestamp'), 'payload': payload})
        (target / f'{participant["name"]}-public-tool-records.json').write_text(json.dumps({'session_id': participant['session_id'], 'source': participant['transcript_path'], 'records': records, 'limits': 'Visible tool calls/output only. Private reasoning, user/developer messages and authentication files excluded. Earlier failed mixed-image calls remain historical.'}, ensure_ascii=False, indent=2) + '\n')
    hashes = {str(path.relative_to(target)): hashlib.sha256(path.read_bytes()).hexdigest() for path in sorted(target.rglob('*')) if path.is_file()}
    (evidence / 'retained-outcome-manifest.json').write_text(json.dumps({'root': str(target), 'sha256': hashes}, indent=2) + '\n')
    print(json.dumps({'retained': str(target), 'files': len(hashes), 'closed': True}))


if __name__ == '__main__':
    main()
