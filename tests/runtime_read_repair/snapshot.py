"""Private scoped rollout receipt. Reads processes/config; optional online DB backup."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import sqlite3
import urllib.request


def ticks(pid):
    if not pid:
        return None
    try:
        return Path(f'/proc/{pid}/stat').read_text().rpartition(')')[2].split()[19]
    except (OSError, IndexError):
        return None


def digest(path):
    path = Path(path)
    if not path.is_file():
        return None
    hasher = hashlib.sha256()
    with path.open('rb') as source:
        for block in iter(lambda: source.read(1024 * 1024), b''):
            hasher.update(block)
    return hasher.hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--scope', required=True)
    parser.add_argument('--url', required=True)
    parser.add_argument('--output', required=True)
    parser.add_argument('--backup')
    parser.add_argument('--compare')
    args = parser.parse_args()
    scope = Path(args.scope).resolve(strict=True)
    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    os.chmod(output.parent, 0o700)
    with urllib.request.urlopen(args.url.rstrip('/') + '/api/v1/management', timeout=30) as response:
        descriptor = json.load(response)
    assert descriptor['scope'] == str(scope), 'selected scope mismatch'
    connection = sqlite3.connect(f'file:{scope / "agentx.db"}?mode=ro', uri=True)
    try:
        rows = [list(row) for row in connection.execute('SELECT name,pid,created_at FROM instances ORDER BY name')]
        if args.backup:
            backup = Path(args.backup)
            fd = os.open(backup, os.O_CREAT | os.O_EXCL | os.O_WRONLY, 0o600)
            os.close(fd)
            with sqlite3.connect(backup) as destination:
                connection.backup(destination)
    finally:
        connection.close()
    result = {'descriptor': descriptor, 'instances': rows,
              'process_start_ticks': {name: ticks(pid) for name, pid, _ in rows},
              'config_sha256': digest(scope / 'config.toml'),
              'installed_sha256': digest(Path.home() / '.local/bin/agentx')}
    if args.compare:
        before = json.loads(Path(args.compare).read_text())
        after_rows = {row[0]: row for row in rows}
        result['preserved_instances'] = [row[0] for row in before['instances'] if after_rows.get(row[0]) == row]
        result['changed_instances'] = [row[0] for row in before['instances'] if after_rows.get(row[0]) != row]
        result['changed_processes'] = [name for name, value in before['process_start_ticks'].items() if result['process_start_ticks'].get(name) != value]
        result['generation_unchanged'] = descriptor['generation'] == before['descriptor']['generation']
        result['config_unchanged'] = result['config_sha256'] == before['config_sha256']
        result['installed_unchanged'] = result['installed_sha256'] == before['installed_sha256']
    output.write_text(json.dumps(result, ensure_ascii=False, indent=2) + '\n')
    os.chmod(output, 0o600)
    print(json.dumps({key: value for key, value in result.items() if key.endswith('_unchanged') or key.startswith('changed_')}))


if __name__ == '__main__':
    main()
