"""Exact embedded extension checks on a private native Qoder fixture; no paid tools."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import sys
import uuid
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'agent_management'))
from fixture import Fixture, wait_for

parser = argparse.ArgumentParser()
parser.add_argument('candidate')
parser.add_argument('evidence')
args = parser.parse_args()
fixture = Fixture(args.candidate, args.evidence)
try:
    http = fixture.prepare('interactions')
    launch = {'generation': http.generation, 'request_id': str(uuid.uuid4()), 'launch': {'tool': 'qoder', 'title': 'Mailbox sender'}}
    http.post('/api/v1/agents', launch)
    operation = http.operation(launch['request_id'])
    name = operation['result']['Launched']['agents'][0]
    actor = wait_for('native sender ready', lambda: next((row for row in http.agents() if row['agent']['name'] == name and row['session_id'] and row['can_send']), None))
    # Two real accepted requests make the fixture reply from its own exact process binding.
    for index in range(2):
        send = {'generation': http.generation, 'request_id': str(uuid.uuid4()), 'agent': actor['agent'], 'message': {'text': f'**Mailbox {index}** `safe` <script>window.mailboxInjected=1</script>', 'intent': 'request'}}
        http.post(f'/api/v1/agents/{name}/messages', send)
        http.operation(send['request_id'])
    wait_for('both human replies', lambda: len(http.get('/api/v1/human-mailbox', pending='true', limit='100')['messages']) == 2)
    config = {'url': http.url, 'generation': http.generation, 'actor': actor, 'out': str(fixture.evidence / 'browser')}
    path = fixture.evidence / 'browser-config.json'
    path.write_text(json.dumps(config, indent=2) + '\n')
    result = subprocess.run(['node', str(Path(__file__).with_name('interactions.mjs')), str(path)], capture_output=True, text=True, timeout=180, env=os.environ)
    (fixture.evidence / 'browser-output.json').write_text(json.dumps({'code': result.returncode, 'stdout': result.stdout, 'stderr': result.stderr}, indent=2) + '\n')
    assert result.returncode == 0, result.stderr
    # Fork capability is exposed by the exact provider and covered by the native unit case;
    # this test deliberately does not click/submit supported fork, avoiding an extra actor.
    refused = {'status': 'not-submitted', 'reason': 'fork menu capability observed; no extra fixture actor created'}
    stop = {'generation': http.generation, 'request_id': str(uuid.uuid4()), 'agent': actor['agent']}
    http.post(f'/api/v1/agents/{name}/stop-participation', stop)
    stopped = http.operation(stop['request_id'])
    assert stopped['result']['Stopped']['name'] == name
    assert stopped['result']['Stopped']['outcome'] in ['participation_retained', 'participation_stopped']
    wait_for('retired exact actor', lambda: not any(row['agent'] == actor['agent'] for row in http.agents()))
    status, stale = http.call('GET', f'/api/v1/agents/{name}', query={'generation': http.generation, 'incarnation': actor['agent']['incarnation']})
    assert status in [404, 409], (status, stale)
    letters = http.get('/api/v1/human-mailbox', pending='false', limit='100')['messages']
    (fixture.evidence / 'native-actions.json').write_text(json.dumps({'fork_not_submitted': refused, 'participation': stopped, 'stale_sender': stale, 'retained_mail_count': len(letters)}, indent=2) + '\n')
finally:
    fixture.close()
