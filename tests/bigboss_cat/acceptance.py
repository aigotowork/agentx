"""Disposable actual daemon/HTTP/browser mailbox checks. Never opens user-managed state."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import sys
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'agent_management'))
from fixture import Fixture

parser = argparse.ArgumentParser()
parser.add_argument('candidate')
parser.add_argument('evidence')
args = parser.parse_args()
fixture = Fixture(args.candidate, args.evidence)
try:
    http = fixture.prepare('mailbox')
    for index in range(23):
        fixture.cli('mailbox', f'seed-{index}', ['send', '--from', 'mailbox-fixture', '@bigboss', '--intent', 'inform', '--', f'Disposable mailbox letter {index}'])
    initial = http.get('/api/v1/human-mailbox', pending='true', limit='100')
    assert initial['unhandled_count'] == 23 and len(initial['messages']) == 23, initial
    browser_config = {'url': http.url, 'generation': http.generation, 'candidate': str(fixture.candidate),
                      'env': fixture.environment('mailbox'), 'project': str(fixture.root / 'mailbox/project'),
                      'out': str(fixture.evidence / 'browser')}
    path = fixture.evidence / 'browser-config.json'
    path.write_text(json.dumps(browser_config, indent=2) + '\n')
    result = subprocess.run(['node', str(Path(__file__).with_name('browser.mjs')), str(path)], capture_output=True, text=True, timeout=240, env=os.environ)
    (fixture.evidence / 'browser-output.json').write_text(json.dumps({'code': result.returncode, 'stdout': result.stdout, 'stderr': result.stderr}, indent=2) + '\n')
    assert result.returncode == 0, result.stderr
    # Restart only this owned daemon: handling must survive process lifetime, not only reload.
    fixture.stop_daemon('mailbox')
    http = fixture.start('mailbox')
    current = http.get('/api/v1/human-mailbox', pending='true', limit='100')
    history = http.get('/api/v1/human-mailbox', pending='false', limit='100')
    assert current['unhandled_count'] == 3 and len(current['messages']) == 3
    assert len(history['messages']) == 24 and sum(message['handled_at'] is not None for message in history['messages']) == 21
    (fixture.evidence / 'persistence.json').write_text(json.dumps({'pending': current, 'history': history}, indent=2) + '\n')
finally:
    fixture.close()
