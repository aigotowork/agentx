#!/usr/bin/env python3
"""Independent object/port semantic counterexamples; no live product execution."""
import copy
import importlib.util
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('port_contract_cases', ROOT / 'tools/flow-atlas/model.py')
contract = importlib.util.module_from_spec(spec)
spec.loader.exec_module(contract)
example = json.loads((ROOT / 'tools/flow-atlas/examples/request.json').read_text())
runtime = json.loads((ROOT / '.docs/architecture/runtime-atlas/model.json').read_text())
contract.validate(example)
contract.validate(runtime)
normalized = contract.validate(example)
assert all('stateFacts' in node for node in normalized['nodes'])
for invalid_binding in [['missing-fact'], ['record', 'record']]:
    candidate = copy.deepcopy(example)
    candidate['nodes'][0]['stateFacts'] = invalid_binding
    try:
        contract.validate(candidate)
    except ValueError as error:
        assert 'stateFacts' in str(error)
    else:
        raise AssertionError('Invalid object state projection accepted')
legacy = copy.deepcopy(example)
legacy.pop('ports')
for scenario in legacy['scenarios']:
    for step in scenario['steps']:
        step.pop('interactions', None)
assert contract.validate(legacy)['ports'] == []

# Mutation helpers derive exact IDs from a real request/response consumer.
def success(model):
    return next(step for scenario in model['scenarios'] for step in scenario['steps'] if any('responseBeat' in i and i['result']['outcomeId'] == 'accepted' for i in step.get('interactions', [])))
def call(model):
    return success(model)['interactions'][0]
def port(model):
    return next(p for p in model['ports'] if p['id'] == call(model)['portId'])
def unknown(model):
    return next(s['steps'][0]['interactions'][0] for s in model['scenarios'] if s['id'] == 'unknown-selection')

mutations = {
    'duplicate port': lambda m: m['ports'].append(copy.deepcopy(m['ports'][0])),
    'unknown owner': lambda m: port(m).update(nodeId='missing-owner'),
    'wrong owner direction': lambda m: port(m).update(nodeId=call(m)['from']),
    'unknown method': lambda m: call(m).update(portId='missing-port'),
    'missing parameter': lambda m: call(m)['input'].pop('recordId'),
    'extra parameter': lambda m: call(m)['input'].update(undeclared=True),
    'unknown outcome': lambda m: call(m)['result'].update(outcomeId='missing-outcome'),
    'response before request': lambda m: call(m).update(responseBeat=call(m)['requestBeat']),
    'wrong response direction': lambda m: success(m)['beats'][call(m)['responseBeat']].update(direction='forward'),
    'disconnect response': lambda m: success(m)['beats'][call(m)['responseBeat']].update(kind='disconnect'),
    'unknown with response': lambda m: unknown(m).update(responseBeat=2),
    'unknown successful effect': lambda m: unknown(m)['effects'][0].update(kind='change'),
    'effect outside patch': lambda m: call(m)['effects'].append({'target':'fact','id':'record','kind':'change'}),
    'competing target': lambda m: call(m).update(to='archive'),
    'request uses disconnect': lambda m: success(m)['beats'][call(m)['requestBeat']].update(kind='disconnect'),
}
for label, mutate in mutations.items():
    candidate = copy.deepcopy(example)
    mutate(candidate)
    try:
        contract.validate(candidate)
    except ValueError as error:
        assert '$.' in str(error), f'{label}: reject context required'
    else:
        raise AssertionError('Incorrect port trace accepted: ' + label)

# Self calls are local work at the owner; labels/types never evaluate code.
self_call = copy.deepcopy(example)
target = success(self_call)
i = call(self_call)
i['from'] = port(self_call)['nodeId']
i['requestBeat'] = 1
i.pop('responseBeat')
target['beats'][1] = {'kind':'local','nodeId':i['from'],'label':'本地调用'}
i['input']['recordId'] = {'inert':'${not executable}', 'value':[1,True,None]}
contract.validate(self_call)

mail = next(s for s in runtime['scenarios'] if s['id'] == 'appserver-mail')
assert [i['result']['outcomeId'] for s in mail['steps'] for i in s.get('interactions',[])] == ['accepted','observed','prepared','unknown','observed','advanced']
assert all(e['kind'] == 'observation' for e in mail['steps'][3]['interactions'][0]['effects'])
assert 'responseBeat' not in mail['steps'][3]['interactions'][0]
assert not mail['steps'][4]['interactions'][0]['effects'], 'Native observation cannot itself advance delivery cursor'
assert mail['steps'][5]['patch']['facts']['inbox'] == '本批次交付已确认'
print(f'PASS: optional legacy, {len(mutations)} rejected port traces, local self call/inert payload, AgentX admission/record/confirmation boundaries')
