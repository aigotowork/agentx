/** Independent outcome checks: schema reuse, process/code correspondence and explicit coverage. */
import assert from 'node:assert/strict';
import * as z from 'zod';
import { compile, inspect, checkTrace, observeProcess } from '../tools/flow-atlas/authoring/index.ts';
import { zodAdapter } from '../tools/flow-atlas/authoring/adapters/zod.ts';
import { designCases, traceCases, schemaCases } from '../tools/flow-atlas/authoring/examples/cases.ts';
import { createOrderExample, observeOrder } from '../tools/flow-atlas/authoring/examples/order-inventory.ts';
import { nativeCallCount, OrderRequest, ReserveRequest } from '../tools/flow-atlas/authoring/examples/native-contracts.ts';

const baseline = nativeCallCount();
const cases = designCases();
for (const item of cases) {
  const result = compile(item.model), artifact = inspect(item.model);
  if (item.expectedCode) assert(artifact.diagnostics.some(d => d.code === item.expectedCode && d.severity === item.severity), `${item.name}: ${JSON.stringify(artifact.diagnostics)}`);
  if (item.severity === 'error') assert.equal(result.ok,false,item.name);
  else assert.equal(result.ok,true,`${item.name}: ${JSON.stringify(result.diagnostics)}`);
}
const example = createOrderExample();
assert.equal(example.prepare.input.native,OrderRequest);
assert.equal(example.reserve.input.native,ReserveRequest);
const artifact=inspect(example.model);
assert.equal(nativeCallCount(),baseline,'Inspection and lint must not invoke methods/classifiers');
assert.equal(artifact.operations[1]!.metadata!.documents![0]!.id,'source-binding');
assert(!JSON.stringify(artifact).includes('nativeOperations'));
for (const item of traceCases()) {
  const result=checkTrace(example.process,item.trace,{complete:item.complete});
  if (item.expectedCode) assert(result.diagnostics.some(d=>d.code===item.expectedCode),`${item.name}: ${JSON.stringify(result)}`);
  else assert.equal(result.diagnostics.length,0,`${item.name}: ${JSON.stringify(result)}`);
}
const conversion=schemaCases();
for(const item of conversion.cases)for(const io of ['input','output'] as const){
  const result=zodAdapter.convert(item.native,{id:item.name,label:item.name,io});
  assert.equal(result.ok,!item.expectedCode,`${item.name}/${io}: ${JSON.stringify(result)}`);
  if(item.expectedCode)assert(result.diagnostics.some(d=>d.code===item.expectedCode));
}
assert.equal(conversion.callbackCount(),0,'Conversion must not execute author callbacks');
const namedStandard=zodAdapter.convert(z.object({'~standard':z.string()}),{id:'business-standard',label:'Business field',io:'input'});
assert(namedStandard.ok);
assert(!Object.hasOwn(namedStandard.value.jsonSchema,'~standard'));
assert.equal((namedStandard.value.jsonSchema.properties as any)['~standard'].type,'string','Only root integration metadata may be omitted');

const outcomes=[];
for(const [sku,terminal,options] of [['book','reserved',{}],['missing','rejected',{}],['lost','unknown',{}],['throw','failed',{}],['book','unknown',{noResponse:true}]] as const){
  const result=await observeOrder({orderId:'accept-'+sku,sku,quantity:1},options);
  assert.equal(result.report.complete,true,JSON.stringify(result));assert.equal(result.report.terminalStepId,terminal);
  outcomes.push({sku,...result});
}
const pending=observeProcess(example.process,example.model.bindings);
const preparing=pending.invoke(example.prepareBinding,{orderId:'pending',sku:'book',quantity:1});
assert(pending.check().diagnostics.some(d=>d.code==='TRACE_PENDING'));
await assert.rejects(pending.invoke(example.reserveBinding,{orderId:'pending',sku:'book',quantity:1}),/CONCURRENT_OBSERVATION/);
await preparing;
assert.equal(pending.trace().length,1);
const trace=pending.trace();assert.throws(()=>{(trace as any).push({});},TypeError);
assert.equal(pending.trace().length,1);
console.log(JSON.stringify({result:'passed',designCases:cases.length,traceCases:traceCases().length,schemaConversions:conversion.cases.length*2,inspectionHandlerCalls:0,outcomes},null,2));
