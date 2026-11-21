/** @aiPurpose Independently challenge released domain contracts with technical-domain fixtures and mutations.
 * @aiUsage Execute only after coherent core release; no source-author fixture decides these expected outcomes.
 */
import assert from 'node:assert/strict';
import {writeFile} from 'node:fs/promises';
import {compile, inspect, checkpoint, snapshot, changes} from '../tools/flow-atlas/domain/index.ts';
import type {DomainModel, Json, SchemaHandle} from '../tools/flow-atlas/domain/index.ts';

const counters = {parse:0};
function schema(id:string, jsonSchema:Record<string,Json>):SchemaHandle {
  return {
    id, label:id,
    native:{parse(value:unknown){counters.parse++; return value;}},
    adapter:{convert(_native, context){return {ok:true, value:{id:context.id,label:context.label??context.id,jsonSchema},diagnostics:[]};}},
  };
}
const text = schema('text', {type:'string'});
const count = schema('count', {type:'integer', minimum:1});
const empty = schema('empty', {type:'object', properties:{}, additionalProperties:false});
const state = schema('job-state', {type:'object', properties:{jobKey:{type:'string'}, status:{type:'string',enum:['idle','prepared','done']}, units:{type:'integer'}},required:['jobKey','status','units'],additionalProperties:false});
const units = schema('units-input', {type:'object',properties:{units:{type:'integer',minimum:1}},required:['units'],additionalProperties:false});
const token = schema('token-input', {type:'object',properties:{token:{type:'string'}},required:['token'],additionalProperties:false});
const evidence = {kind:'authored' as const, explanation:'Independent acceptance witness; not native execution.'};

function model():DomainModel {
  return {
    definition:{
      id:'technical-fixture', label:'Technical job lifecycle', sources:[],
      contexts:[{id:'scheduler',label:'Scheduler',responsibility:'Manage local technical jobs.'}],
      modules:[{id:'jobs',label:'Jobs',contextId:'scheduler',responsibility:'Own job lifecycle and transfer tokens.',dependencies:[]}],
      values:[
        {id:'job-count',label:'Job units',schema:count,meaning:'Number of work units',unit:'work-unit'},
        {id:'transfer-token',label:'Transfer token',schema:text,meaning:'Capability produced by prepare'},
        {id:'unrelated-key',label:'Unrelated key',schema:text,meaning:'A different technical identity'},
      ],
      objects:[
        {id:'driver',label:'Driver',kind:'technical',moduleId:'jobs',responsibility:'Coordinate public job methods.',state:empty,fields:[]},
        {id:'job',label:'Job',kind:'entity',moduleId:'jobs',responsibility:'Own preparation and completion.',state, businessIdentity:[['jobKey']],fields:[{id:'job-key',label:'Job key',path:['jobKey']},{id:'job-status',label:'Status',path:['status']},{id:'job-units',label:'Units',path:['units'],valueId:'job-count'}],lifecycle:{path:['status'],transitions:[
          {id:'prepared-transition',label:'Prepare',from:'idle',to:'prepared',operationId:'prepare-op',outcomeId:'ready'},
          {id:'done-transition',label:'Complete',from:'prepared',to:'done',operationId:'consume-op',outcomeId:'accepted'},
        ]}},
      ],
      rules:[{id:'unit-rule',label:'Positive work',ownerId:'job',description:'Work units remain positive.',basis:'declared',sourceIds:[],check:{left:{kind:'state',path:['units']},operator:'gte',right:{kind:'literal',value:1}}}],
      operations:[
        {id:'prepare-op',label:'Prepare',ownerId:'job',kind:'command',input:units,inputFields:[{id:'prepare-units',label:'Units',path:['units'],valueId:'job-count'}],outcomes:[{id:'ready',label:'Ready',meaning:'returned',payload:token,fields:[{id:'ready-token',label:'Token',path:['token'],valueId:'transfer-token'}]},{id:'refused',label:'Refused',meaning:'rejected'},{id:'unknown',label:'Unknown',meaning:'unknown'}],effects:[{outcomeId:'ready',fieldPaths:[['status']]}],ruleIds:['unit-rule'],binding:{status:'declared',sourceIds:[]}},
        {id:'consume-op',label:'Consume',ownerId:'job',kind:'command',input:token,inputFields:[{id:'consume-token',label:'Token',path:['token'],valueId:'transfer-token'}],outcomes:[{id:'accepted',label:'Accepted',meaning:'returned'},{id:'refused',label:'Refused',meaning:'rejected'}],effects:[{outcomeId:'accepted',fieldPaths:[['status']]}],ruleIds:['unit-rule'],binding:{status:'declared',sourceIds:[]}},
      ],
      relationships:[{id:'driver-job',label:'Coordinates',kind:'uses',fromObjectId:'driver',toObjectId:'job'}],
      processes:[{id:'job-process',label:'Prepare and consume',input:units,inputFields:[{id:'process-units',label:'Units',path:['units'],valueId:'job-count'}],roles:[{id:'caller',label:'Caller',objectId:'driver'},{id:'target',label:'Target',objectId:'job'}],startId:'start',steps:[
        {id:'start',label:'Start',kind:'start'},
        {id:'prepare',label:'Prepare job',kind:'operation',roleId:'target',callerRoleId:'caller',operationId:'prepare-op',bindings:[{id:'units-binding',label:'Process units',targetPath:['units'],source:{kind:'input',path:['units']}}]},
        {id:'consume',label:'Consume token',kind:'operation',roleId:'target',callerRoleId:'caller',operationId:'consume-op',bindings:[{id:'token-binding',label:'Ready token',targetPath:['token'],source:{kind:'result',stepId:'prepare',outcomeId:'ready',path:['token']}}]},
        {id:'done',label:'Done',kind:'end',outcome:'completed'},
        {id:'prepare-refused',label:'Prepare refused',kind:'end',outcome:'rejected'},
        {id:'consume-refused',label:'Consume refused',kind:'end',outcome:'rejected'},
        {id:'lost',label:'Unknown',kind:'end',outcome:'unknown'},
      ],flows:[
        {id:'f-start',label:'Begin',from:'start',to:'prepare'},
        {id:'f-ready',label:'Ready',from:'prepare',to:'consume',outcomeId:'ready'},
        {id:'f-prepare-refused',label:'Refused',from:'prepare',to:'prepare-refused',outcomeId:'refused'},
        {id:'f-lost',label:'Unknown',from:'prepare',to:'lost',outcomeId:'unknown'},
        {id:'f-accepted',label:'Accepted',from:'consume',to:'done',outcomeId:'accepted'},
        {id:'f-consume-refused',label:'Refused',from:'consume',to:'consume-refused',outcomeId:'refused'},
      ]}],
    },
    scenarios:[{id:'independent-success',label:'Independent technical success',processId:'job-process',input:{units:2},instances:[
      {id:'driver-instance',label:'Driver instance',objectId:'driver',initial:{}},
      {id:'job-A',label:'Job A',objectId:'job',businessIdentity:{jobKey:'native-A'},initial:{jobKey:'native-A',status:'idle',units:2}},
      {id:'job-B',label:'Job B',objectId:'job',businessIdentity:{jobKey:'native-B'},initial:{jobKey:'native-B',status:'idle',units:9}},
    ],roles:{caller:'driver-instance',target:'job-A'},visits:[
      {id:'prepare-visit',label:'Preparation',stepId:'prepare',viaFlowId:'f-start',callId:'prepare-call',input:{units:2},outcomeId:'ready',completion:'reply',payload:{token:'token-A'},evidence,effect:{outcomeId:'ready',writes:[{instanceId:'job-A',path:['status'],value:'prepared',transitionId:'prepared-transition'}],evidence}},
      {id:'consume-visit',label:'Consumption',stepId:'consume',viaFlowId:'f-ready',callId:'consume-call',input:{token:'token-A'},outcomeId:'accepted',completion:'reply',evidence,effect:{outcomeId:'accepted',writes:[{instanceId:'job-A',path:['status'],value:'done',transitionId:'done-transition'}],evidence}},
    ],terminalStepId:'done',expectations:[{id:'job-done',label:'Job completed',kind:'state',instanceId:'job-A',path:['status'],equals:'done'},{id:'other-isolated',label:'Other job unchanged',kind:'state',instanceId:'job-B',path:['status'],equals:'idle'},{id:'successful-terminal',label:'Success',kind:'terminal',outcome:'completed'}]}],
  };
}

// Source handles intentionally retain native identity; only JSON-authored parts are mutated.
function mutable():any {return model();}
const results:unknown[] = [];
function negative(name:string, mutate:(draft:any)=>void) {
  const draft=mutable(); mutate(draft);
  const before=JSON.stringify(draft), report=compile(draft), artifact=inspect(draft);
  assert.equal(report.ok,false, `${name}: invalid design must not compile`);
  const errors=artifact.diagnostics.filter(item=>item.severity==='error');
  assert(errors.length>0, `${name}: a concrete diagnostic is required`);
  assert(errors.every(item=>item.path.length>0 && item.message.length>0),`${name}: diagnostics must be located`);
  assert.equal(JSON.stringify(draft),before, `${name}: inspecting cannot rewrite authored input`);
  results.push({name,diagnostics:errors});
}

const source=model(), original=JSON.stringify(source), baseline=compile(source);
assert(baseline.ok, JSON.stringify(baseline.diagnostics));
const artifact=baseline.value;
assert.equal(artifact.format,'flow-atlas/workbench-1');
assert.equal(JSON.stringify(source),original);
assert.equal(JSON.stringify(inspect(source)),JSON.stringify(artifact),'Repeated inspection is deterministic');
assert.deepEqual(counters,{parse:0},'Static schema validation does not parse native input');
const scenario=artifact.scenarios[0]!, finalIndex=scenario.checkpoints.length-1;
assert.equal((snapshot(artifact,scenario.id,0)['job-A'] as any).status,'idle');
assert.equal((snapshot(artifact,scenario.id,finalIndex)['job-A'] as any).status,'done');
assert.equal((snapshot(artifact,scenario.id,finalIndex)['job-B'] as any).status,'idle');
const copy=snapshot(artifact,scenario.id,0) as any; copy['job-A'].status='external-mutation';
assert.equal((snapshot(artifact,scenario.id,0)['job-A'] as any).status,'idle','Snapshots are detached');
const detached=checkpoint(artifact,scenario.id,finalIndex) as any; detached.calls[0].input.units=500;
assert.equal((checkpoint(artifact,scenario.id,finalIndex).calls[0]!.input as any).units,2,'Nested calls are detached');
assert.throws(()=>checkpoint(artifact,'absent',0),RangeError);
assert.throws(()=>checkpoint(artifact,scenario.id,9999),RangeError);
assert(scenario.checkpoints.every(point=>point.phase==='effect'||point.differences.length===0),'Differences exist only when effects apply');
const provenance=scenario.checkpoints.flatMap(point=>point.parameters).find(parameter=>parameter.id==='token-binding');
assert(provenance && provenance.status==='resolved' && provenance.sourceVisitId==='prepare-visit' && provenance.sourceCallId==='prepare-call');
assert.deepEqual(provenance.value,'token-A');
const effectPoint=scenario.checkpoints.find(point=>point.phase==='effect')!;
const changeCopy=changes(artifact,scenario.id,effectPoint.index) as any;
changeCopy[0].after='external-mutation';
assert.notEqual(changes(artifact,scenario.id,effectPoint.index)[0]!.after,'external-mutation');
results.push({name:'technical-positive-isolation-and-checkpoints',checkpoints:scenario.checkpoints.length});

negative('missing entity identity',m=>delete m.definition.objects[1].businessIdentity);
negative('business identity differs from initial state',m=>m.scenarios[0].instances[1].businessIdentity={jobKey:'wrong-key'});
negative('unknown rule owner',m=>m.definition.rules[0].ownerId='absent-owner');
negative('wrong operation owner',m=>m.definition.operations[0].ownerId='driver');
negative('foreign state write',m=>m.scenarios[0].visits[0].effect.writes[0].instanceId='job-B');
negative('query declares state effect',m=>m.definition.operations[0].kind='query');
negative('illegal lifecycle transition',m=>m.scenarios[0].visits[0].effect.writes[0].value='done');
negative('missing required parameter source',m=>m.definition.processes[0].steps[1].bindings=[]);
negative('source field absent',m=>m.definition.processes[0].steps[1].bindings[0].source.path=['absent']);
negative('incompatible primitive source',m=>m.definition.processes[0].steps[1].bindings[0].source={kind:'literal',value:'two',valueId:'job-count'});
negative('incompatible semantic value',m=>m.definition.operations[1].inputFields[0].valueId='unrelated-key');
negative('wrong result branch',m=>m.definition.processes[0].steps[2].bindings[0].source.outcomeId='refused');
negative('future result unavailable',m=>m.definition.processes[0].steps[1].bindings[0].source={kind:'result',stepId:'consume',outcomeId:'accepted',path:['token']});
negative('unsupported cycle',m=>m.definition.processes[0].flows.find((f:any)=>f.id==='f-accepted').to='prepare');
negative('unsupported fork',m=>m.definition.processes[0].steps[0].kind='fork');
negative('unsupported join',m=>m.definition.processes[0].steps[0].kind='join');
negative('scenario expected differs from actual',m=>m.scenarios[0].expectations[0].equals='idle');
negative('observed input differs from bound value',m=>m.scenarios[0].visits[1].input.token='wrong-token');
negative('local unknown fabricates payload',m=>{m.scenarios[0].visits[0].completion='local-outcome';m.scenarios[0].visits[0].outcomeId='unknown';});

const lost=mutable();
lost.scenarios[0].id='independent-committed-unknown';
lost.scenarios[0].visits=lost.scenarios[0].visits.slice(0,1);
Object.assign(lost.scenarios[0].visits[0],{completion:'local-outcome',outcomeId:'unknown'});
delete lost.scenarios[0].visits[0].payload;
lost.scenarios[0].terminalStepId='lost';
lost.scenarios[0].expectations=[{id:'still-prepared',label:'Effect committed',kind:'state',instanceId:'job-A',path:['status'],equals:'prepared'},{id:'caller-unknown',label:'Caller unknown',kind:'call',visitId:'prepare-visit',outcomeId:'unknown'},{id:'unknown-terminal',label:'Unknown end',kind:'terminal',outcome:'unknown'}];
const lostResult=compile(lost);
assert(lostResult.ok,JSON.stringify(lostResult.diagnostics));
const lostScenario=lostResult.value.scenarios[0]!;
assert.deepEqual(lostScenario.checkpoints.map(point=>point.phase),['initial','call','effect','outcome','terminal']);
const lostFinal=lostScenario.checkpoints.at(-1)!;
assert.equal((lostFinal.states['job-A'] as any).status,'prepared');
assert.equal(lostFinal.calls[0]!.phase,'unknown');
assert.equal(lostFinal.calls[0]!.effectOutcomeId,'ready');
assert.equal(lostFinal.calls[0]!.completion,'local-outcome');
assert.equal(lostFinal.calls[0]!.payload,undefined);
results.push({name:'committed-effect-then-local-uncertainty',phases:lostScenario.checkpoints.map(point=>point.phase)});

const envelope=lost;
envelope.scenarios[0].id='independent-unknown-envelope';
envelope.definition.operations[0].outcomes[2].payload=schema('uncertainty-envelope',{type:'object',properties:{reason:{type:'string'}},required:['reason'],additionalProperties:false});
Object.assign(envelope.scenarios[0].visits[0],{completion:'reply',payload:{reason:'Result is still being reconciled'}});
const envelopeResult=compile(envelope);
assert(envelopeResult.ok,JSON.stringify(envelopeResult.diagnostics));
const envelopeScenario=envelopeResult.value.scenarios[0]!;
assert.deepEqual(envelopeScenario.checkpoints.map(point=>point.phase),['initial','call','effect','reply','terminal']);
assert.equal(envelopeScenario.checkpoints.at(-1)!.calls[0]!.phase,'unknown');
assert.equal(envelopeScenario.checkpoints.at(-1)!.calls[0]!.completion,'reply');
assert.equal(JSON.stringify(envelopeScenario.checkpoints.at(-1)!.calls[0]!.payload),JSON.stringify({reason:'Result is still being reconciled'}));
results.push({name:'unknown-envelope-is-a-real-reply',phases:envelopeScenario.checkpoints.map(point=>point.phase)});

const output={result:'passed',cases:results.length,counters,results};
if(process.argv[2]) await writeFile(process.argv[2],JSON.stringify(output,null,2)+'\n');
console.log(JSON.stringify(output,null,2));
