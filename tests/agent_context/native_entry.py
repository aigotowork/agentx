#!/usr/bin/python3
"""Keep ordinary AgentX launch binding while exposing native app-server I/O.

Only the fixture's final `exec` token becomes `app-server --stdio`; real
Codex receives the exact AgentX-generated hook and developer overrides.
"""
import os
import sys
args = sys.argv[1:]
if 'exec' in args:
    index = args.index('exec')
    args[index:index+1] = ['app-server', '--stdio']
os.execv(os.environ['CONTEXT_REAL_CODEX'], [os.environ['CONTEXT_REAL_CODEX'], *args])
