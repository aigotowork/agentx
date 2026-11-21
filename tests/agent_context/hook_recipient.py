#!/usr/bin/python3
"""Finite provider substitute executing real candidate hook commands, not a model."""
import importlib.util
import json
import os
from pathlib import Path
import sqlite3
import subprocess
import sys

tool = Path(sys.argv[0]).name
if tool == 'codex' and 'app-server' in sys.argv:
    spec = importlib.util.spec_from_file_location('preflight', os.environ['CONTEXT_PREFLIGHT_STUB'])
    stub = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(stub)
    stub.hooks_preflight()
    raise SystemExit(0)
if '--version' in sys.argv or '-V' in sys.argv:
    print('codex-cli 0.167.0' if tool == 'codex' else '2.1.296 (Claude Code)')
    raise SystemExit(0)

binary = os.environ['CONTEXT_CANDIDATE']
scope = Path(os.environ['AGENTX_DIR'])
capture = Path(os.environ['CONTEXT_HOOK_CAPTURE'])
session = 'context-hook-' + tool
guide = Path.cwd() / 'guide.txt'
commands, results = [], []

def call(command, payload):
    result = subprocess.run([binary, command], input=json.dumps(payload), text=True, capture_output=True, timeout=15)
    commands.append({'command':command, 'payload':payload, 'exit_code':result.returncode, 'stdout':result.stdout, 'stderr':result.stderr})
    assert result.returncode == 0, commands[-1]
    return result.stdout

def snapshot():
    uri = (scope / 'agentx.db').as_uri() + '?mode=ro'
    with sqlite3.connect(uri, uri=True) as db:
        db.row_factory = sqlite3.Row
        rows = [dict(row) for row in db.execute('select * from instances')]
    name = os.environ.get('AGENTX_INSTANCE_NAME')
    matches = [row for row in rows if row['name'] == name]
    assert len(matches) == 1, {'name':name, 'rows':rows}
    keys = ['name','session_id','status','status_context','status_detail','directory','transcript_path','name_announced']
    return {key:matches[0][key] for key in keys if key in matches[0]}

try:
    prefix = 'codex-' if tool == 'codex' else ''
    payload = {'session_id':session, 'cwd':str(Path.cwd()), 'source':'startup', 'transcript_path':str(Path.cwd()/'transcript.jsonl')}
    (Path.cwd()/'transcript.jsonl').write_text('')
    call(prefix+'sessionstart', payload)
    call(prefix+'userpromptsubmit', {**payload, 'prompt':'fixture active user turn'})
    before = snapshot()
    assert before['status'] == 'active', before
    for value in ['FRESH_HOOK_ONE', 'FRESH_HOOK_TWO']:
        guide.write_text(value)
        output = call(prefix+'sessionstart', {**payload, 'source':'compact'})
        parsed = json.loads(output)
        assert parsed['hookSpecificOutput']['additionalContext'] == value, parsed
        after = snapshot()
        for key in ['name','session_id','status','status_context','status_detail']:
            if key in before: assert after[key] == before[key], (before, after)
        results.append({'case':'H01-'+tool+'-'+value, 'passed':True, 'before':before, 'after':after})
        before = after
    if tool == 'codex':
        for label, bad in [('missing',None),('empty',''),('wrong','another-session'),('stale','retired-session')]:
            bad_payload = {**payload, 'source':'compact', 'cwd':str(Path.cwd()/'foreign-cwd'), 'transcript_path':str(Path.cwd()/'foreign-transcript')}
            if bad is None: bad_payload.pop('session_id')
            else: bad_payload['session_id'] = bad
            output = call(prefix+'sessionstart', bad_payload)
            assert not output.strip(), output
            after = snapshot()
            assert after == before, (before,after)
            results.append({'case':'H02-'+label, 'passed':True, 'before':before, 'after':after})
    (capture/'result.json').write_text(json.dumps({'tool':tool,'results':results,'commands':commands},indent=2)+'\n')
except Exception as error:
    (capture/'result.json').write_text(json.dumps({'tool':tool,'results':results,'commands':commands,'error':repr(error)},indent=2)+'\n')
    raise
