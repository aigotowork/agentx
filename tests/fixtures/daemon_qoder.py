#!/usr/bin/python3
"""Controlled Qoder composer for one actual AgentX launcher/PTY acceptance case.

Executes only the generated per-run hook commands. All context and binding
values come from the production launcher. This is not a native Provider.
"""
import codecs
import hashlib
import json
import os
from pathlib import Path
import select
import subprocess
import sys
import termios
import time
import tty
import tomllib


def main():
    if any(value in sys.argv for value in ['--version', '-V']):
        print('codex-cli 0.134.0 (controlled PTY fixture)')
        return
    tool = 'codex' if Path(sys.argv[0]).name == 'codex' else os.environ.get('AGENTX_TOOL', 'qoder')
    if tool == 'codex':
        hooks = {}
        for index, value in enumerate(sys.argv[1:], 1):
            if value in ['-c', '--config'] and index+1 < len(sys.argv):
                raw = sys.argv[index+1]
                if raw.startswith('hooks.') and not raw.startswith('hooks.state='):
                    hooks.update(tomllib.loads(raw).get('hooks', {}))
        if 'app-server' in sys.argv:
            entries = [{'key': f'fixture:{event}:{group_index}:{index}',
                        'command': handler['command'], 'source': 'sessionFlags',
                        'currentHash': hashlib.sha256(handler['command'].encode()).hexdigest()}
                       for event, groups in hooks.items()
                       for group_index, group in enumerate(groups)
                       for index, handler in enumerate(group['hooks'])]
            for line in sys.stdin:
                request = json.loads(line)
                if request['method'] == 'initialized':
                    continue
                assert request['method'] in ['initialize', 'hooks/list']
                result = {'data': [{'hooks': entries}]} if request['method'] == 'hooks/list' else {}
                print(json.dumps({'id': request['id'], 'result': result}), flush=True)
            return
    root = Path(os.environ['AGENTX_TEST_ROOT']).resolve()
    scope = Path(os.environ['AGENTX_DIR']).resolve()
    assert root in scope.parents
    observations = root / 'qoder-observations.jsonl'

    def record(kind, **values):
        with observations.open('a') as stream:
            stream.write(json.dumps({'kind': kind, 'time': time.time(), **values}) + '\n')
            stream.flush()

    settings_values = []
    arguments = iter(sys.argv[1:])
    for value in arguments:
        if value == '--settings':
            settings_values.append(next(arguments))
        elif value.startswith('--settings='):
            settings_values.append(value.split('=', 1)[1])
    if tool == 'codex':
        assert 'SessionStart' in hooks and 'Stop' in hooks
        settings_path = Path(os.environ['CODEX_HOME'])
        settings = {'hooks': hooks}
    else:
        assert len(settings_values) == 1, settings_values
        settings_path = Path(settings_values[0]).resolve()
        assert scope in settings_path.parents
        settings = json.loads(settings_path.read_text())
    session = f'controlled-{tool}-session'
    base = {'session_id': session, 'cwd': os.getcwd()}
    recovery = tool == 'codex' and os.environ.get('AGENTX_TEST_RECOVERY') == '1'
    missed_stop = tool == 'codex' and os.environ.get('AGENTX_TEST_MISSED_STOP') == '1'
    if missed_stop:
        from datetime import datetime, timezone
        rollout = root / 'controlled-rollout.jsonl'
        rollout.write_text(json.dumps({'type': 'session_meta', 'payload': {'id': session}})+'\n')
        base['transcript_path'] = str(rollout)

        def rollout_event(kind):
            with rollout.open('a') as out:
                out.write(json.dumps({'timestamp': datetime.now(timezone.utc).isoformat(),
                                      'type': 'event_msg', 'payload': {'type': kind, 'turn_id': 'missed'}})+'\n')

    def hook(event, **values):
        outputs = []
        commands = [entry['command'] for group in settings['hooks'][event]
                    for entry in group['hooks'] if entry.get('type') == 'command']
        assert commands, event
        for command in commands:
            result = subprocess.run(['/bin/sh', '-c', command],
                                    input=json.dumps({**base, **values}),
                                    capture_output=True, text=True, timeout=10)
            record('hook', event=event, command=command, code=result.returncode,
                   stdout=result.stdout, stderr=result.stderr)
            assert result.returncode == 0, result.stderr
            if result.stdout.strip():
                outputs.append(json.loads(result.stdout))
        if event == 'Stop' and any(item.get('decision') == 'block' for item in outputs):
            # Qoder continues the current turn from Stop output, including
            # the first Stop after resume. No second prompt is submitted.
            record('continued', outputs=outputs)
            record('delivered', outputs=outputs)
        return outputs

    env_keys = ['AGENTX_DIR', 'AGENTX_LAUNCHED', 'AGENTX_TOOL', 'AGENTX_PROCESS_ID',
                'AGENTX_LAUNCH_PARTICIPANT', 'AGENTX_LAUNCH_GENERATION',
                'AGENTX_LAUNCH_START_EVENT_ID', 'AGENTX_PTY_MODE',
                'QODER_CONFIG_DIR', 'HOME']
    record('started', pid=os.getpid(), ppid=os.getppid(), cwd=os.getcwd(),
           settings=str(settings_path), args=sys.argv[1:],
           environment={key: os.environ.get(key) for key in env_keys})
    started = hook('SessionStart', source='startup')
    record('session-started', outputs=started)
    assert any('additionalContext' in item.get('hookSpecificOutput', {}) for item in started)
    result = subprocess.run(['agentx', 'send', '@'+os.environ['AGENTX_TEST_PARENT'],
                             '--', 'controlled child to parent ready'],
                            capture_output=True, text=True, timeout=10)
    record('child_send', code=result.returncode, stdout=result.stdout, stderr=result.stderr)
    assert result.returncode == 0, result.stderr
    hook('Stop', stop_hook_active=False)

    original = termios.tcgetattr(0)
    tty.setraw(0)
    composer = ''

    def render():
        columns, rows = os.get_terminal_size(1)
        rule = '\u2500' * columns
        content = ['? for shortcuts', rule,
                   ' Shift+Tab to Accept Edits', rule,
                   ' > '+(composer or '  Type your message or @path/to/file'),
                   rule, ' Qwen3.8-Flash Model · ctx ░░░░░░░░░░ 0% · /work/dir']
        if tool == 'codex':
            content = ['Controlled Codex fixture', '› '+composer, '', '  ? for shortcuts']
        screen = '\x1b[2J\x1b[H'+f'\x1b[{max(1, rows-len(content)+1)};1H'
        os.write(1, (screen+'\r\n'.join(content)).encode())

    decoder = codecs.getincrementaldecoder('utf-8')()
    pending = ''
    deadline = time.monotonic()+90
    control = root / 'qoder-control.json'
    last_control = None
    try:
        render()
        record('ready')
        while time.monotonic() < deadline:
            if control.exists():
                command = json.loads(control.read_text())
                if command != last_control:
                    event = command['event']
                    if missed_stop and event == 'StartWithoutStop':
                        rollout_event('task_started')
                        hook('PreModelRequest', turn_id='missed', model='fixture')
                        composer = 'a user draft'
                        render()
                    elif missed_stop and event == 'CompleteWithoutStop':
                        rollout_event('task_complete')
                    elif missed_stop and event == 'ClearDraft':
                        composer = ''
                        render()
                    elif recovery and event == 'Fail':
                        hook('PreModelRequest', turn_id='original', model='fixture')
                        hook('PostModelRequest', turn_id='original', model='fixture', status='failed', error='upstream 502')
                    elif recovery and event == 'StopWithDraft':
                        composer = 'a user draft'
                        render()
                        hook('Stop', turn_id='original', stop_hook_active=False)
                    elif recovery and event == 'ClearDraft':
                        composer = ''
                        render()
                    elif recovery and event == 'Success':
                        hook('PostModelRequest', turn_id='continued', model='fixture', status='completed')
                        hook('Stop', turn_id='continued', stop_hook_active=False)
                    else:
                        assert event in ['UserPromptSubmit', 'Stop']
                        hook(event, prompt='controlled active turn', stop_hook_active=False)
                    record('controlled-state', command=command)
                    last_control = command
            if not select.select([0], [], [], 1)[0]:
                continue
            data = os.read(0, 4096)
            if not data:
                break
            record('input', hex=data.hex())
            pending += decoder.decode(data)
            # PTY injection can bracket paste; consume split escape wrappers
            # before interpreting composer text or Enter.
            while pending:
                wrapper = next((token for token in ['\x1b[200~', '\x1b[201~'] if pending.startswith(token)), None)
                if wrapper:
                    pending = pending[len(wrapper):]
                    continue
                if any(token.startswith(pending) for token in ['\x1b[200~', '\x1b[201~']):
                    break
                character, pending = pending[0], pending[1:]
                if character in '\r\n':
                    if composer:
                        prompt, composer = composer, ''
                        record('submitted', prompt=prompt)
                        if recovery and prompt == 'Continue the current task.':
                            hook('UserPromptSubmit', prompt=prompt)
                            hook('PreModelRequest', turn_id='continued', model='fixture')
                            record('recovery-started', session=session)
                        else:
                            assert prompt == '<agentx>', prompt
                            output = hook('UserPromptSubmit', prompt=prompt)
                            record('delivered', outputs=output)
                            hook('Stop', stop_hook_active=False)
                    render()
                elif character in ['\x7f', '\b']:
                    composer = composer[:-1]
                    render()
                elif character >= ' ':
                    composer += character
                    render()
                elif character in ['\x03', '\x04']:
                    return
    finally:
        termios.tcsetattr(0, termios.TCSANOW, original)
        record('finished')


if __name__ == '__main__':
    main()
