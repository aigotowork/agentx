/** @aiPurpose Independently check actual typed method correspondence, handle authenticity and inert export.
 * @aiUsage Run against the coherent binding derivative; native business/checkpoint behavior is covered by retained prior evidence.
 */
import assert from 'node:assert/strict';
import {writeFile} from 'node:fs/promises';
import {bindNativeMethod,compile,inspect} from '../tools/flow-atlas/domain/index.ts';
import {createOrderModel} from '../tools/flow-atlas/domain/examples/order-stock.ts';
import {nativeCallCount} from '../tools/flow-atlas/domain/examples/native.ts';

const originalCalls=nativeCallCount(),results=[];
const source=createOrderModel(),baseline=compile(source);assert(baseline.ok,JSON.stringify(baseline.diagnostics));
assert(baseline.value.definition.operations.every(op=>op.binding.status==='source-bound'&&op.nativeMethodId&&op.nativeOutputSchemaId));
function inert(value:unknown):void {
  assert.notEqual(typeof value,'function');
  if(value&&typeof value==='object')for(const descriptor of Object.values(Object.getOwnPropertyDescriptors(value))){assert('value'in descriptor,'Artifact cannot contain an accessor');inert(descriptor.value);}
}
inert(baseline.value);assert(baseline.value.definition.operations.every(op=>!Object.hasOwn(op,'native')));
let invocations=0;
const registered=createOrderModel() as any,operation=registered.definition.operations[0];
operation.native=bindNativeMethod({id:'independent.method',label:'Independent method',ownerId:operation.ownerId,input:operation.input,output:operation.native.output,method:()=>{invocations++;throw new Error('Inspection must never invoke this method');}});
assert(compile(registered).ok);assert.equal(invocations,0);
results.push({name:'real-bound-method-inert-export',nativeCalls:nativeCallCount(),invocations});

for(const [name,code,mutate] of [
  ['copied handle','FOREIGN_NATIVE_METHOD',(m:any)=>m.definition.operations[0].native={...m.definition.operations[0].native}],
  ['forged handle','FOREIGN_NATIVE_METHOD',(m:any)=>m.definition.operations[0].native={id:'forged',label:'Forged',ownerId:m.definition.operations[0].ownerId,input:m.definition.operations[0].input,output:m.definition.operations[0].native.output}],
  ['wrong exact input handle','NATIVE_INPUT_MISMATCH',(m:any)=>m.definition.operations[0].input={...m.definition.operations[0].input}],
  ['wrong native owner','NATIVE_METHOD_OWNERSHIP',(m:any)=>m.definition.operations[0].ownerId='object.checkout'],
  ['duplicate actual method ownership','DUPLICATE_METHOD_OWNERSHIP',(m:any)=>m.definition.operations.push({...m.definition.operations[0],id:'duplicate-operation'})],
] as const){
  const draft=createOrderModel() as any;mutate(draft);const report=compile(draft);
  assert(!report.ok,`${name}: invalid native correspondence cannot compile`);
  assert(report.diagnostics.some(d=>d.code===code&&d.severity==='error'),`${name}: ${JSON.stringify(report.diagnostics)}`);
  results.push({name,diagnostics:report.diagnostics});
}
const absent=createOrderModel() as any;delete absent.definition.operations[0].native;
const declared=inspect(absent),downgraded=declared.definition.operations[0]!;
assert.equal(downgraded.binding.status,'declared');assert.equal(downgraded.nativeMethodId,undefined);
assert(declared.diagnostics.some(d=>d.code==='UNBOUND_NATIVE_METHOD'&&d.severity==='warning'));
assert(declared.coverage.some(item=>item.subjectId===downgraded.id&&item.status==='declared'));
results.push({name:'source-links-alone-downgrade-to-declaration'});
assert.throws(()=>bindNativeMethod({...operation.native,method:undefined} as any),TypeError);
assert.equal(nativeCallCount(),originalCalls);assert.equal(invocations,0);
const output={result:'passed',results,nativeCalls:nativeCallCount(),invocations};
if(process.argv[2])await writeFile(process.argv[2],JSON.stringify(output,null,2)+'\n');
if(process.argv[3])await writeFile(process.argv[3],JSON.stringify(baseline.value,null,2)+'\n');
console.log(JSON.stringify(output,null,2));
