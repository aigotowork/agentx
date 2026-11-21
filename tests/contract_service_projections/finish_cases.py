#!/usr/bin/env python3
"""Supplement only uncovered public CLI/preview/finite-gap branches in owned scope."""
import copy
import json
from pathlib import Path
from system_acceptance import SystemCases, encode
from native_http import Fixture
from package_acceptance import WireFixture

base = Path(__file__).resolve().parent / 'evidence/full-system'
f = Fixture.reopen_owned('/tmp/agentx-contract-candidate-r1-99xo9tn2/agentx-system-schema-debug', base/'runtime-08', base/'runtime-07')
result = {'status': 'failed'}
try:
    cases = SystemCases(f, Path('/tmp/agentx-contract-package-zh7txu0q/consumer'))
    f.start('primary')
    scope, project = f.root/'primary/scope', f.root/'primary/project'
    roles = project/'.agentx/roles'
    roles.mkdir(parents=True, exist_ok=True)
    (roles/'fixture.toml').write_text('default = true\nprofile = "management-fixture"\nprompt = "Finite isolated fixture role; no paid Provider."\n')
    before = cases.management('listAgents', {'generation': f.clients['primary'].generation})
    preview = cases.management('directory.inspect', {'generation': f.clients['primary'].generation, 'directory': str(project)})
    assert cases.management('listAgents', {'generation': f.clients['primary'].generation}) == before
    cases.record('CS-07-actual-successful-route-preview', preview=preview)
    url = cases.listener(['workbench','--scope',str(scope),'--project',str(project),'--collaboration','--host','127.0.0.1','--port','0'], 'workbench')
    command = cases.execute('work-cli-list', [cases.node,cases.bin,'work','list','--workbench-url',url,'--limit','8'])
    assert command.returncode == 0 and 'Read' in json.loads(command.stdout), command
    for flag in ['--help','--version']:
        checked = cases.execute('offline-final', [cases.node,cases.bin,flag])
        assert checked.returncode == 0 and checked.stdout and not checked.stderr, checked
    cases.record('CS-08-10-actual-work-cli-and-offline-entry', cli=command.stdout)
    prior = json.loads((base/'runtime-07/package-cases.json').read_text())[-1]
    finite = copy.deepcopy(prior['actual_gap_probe'])
    head = prior['events']['Inspection']['response']['Events']['Ok']['Head']
    finite['Inspection']['response']['Events']['Ok']['Read'] = {'Gap': {'first_missing':1,'last_missing':2,'head':head}}
    schema = json.loads((Path(__file__).resolve().parents[2]/'.docs/api-panorama/openapi/observation.json').read_text())
    operand = {'window': {'after': {'stream':head['stream'],'sequence':0}, 'through':head['newest'],
                         'filter': {key:None for key in schema['components']['schemas']['ObservationFilter']['properties']}}, 'limit':8}
    with WireFixture(encode(finite)) as wire:
        cases.urls['observation'] = wire.url
        response = cases.domain('observation','events.read',('ObservationRead',operand))
    assert response == finite
    cases.record('CS-09-finite-gap-preserved', boundary='installed public SDK over finite HTTP; no native retention-gap transition claimed', response=response)
    result['status'] = 'passed'
finally:
    f.close()
    (f.evidence/'system-result.json').write_text(json.dumps(result,indent=2)+'\n')
