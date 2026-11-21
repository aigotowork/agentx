/** @aiPurpose Independently verify native owner methods and their correspondence to inert workbench exports.
 * @aiUsage Run explicitly after coherent source release; all instances stay isolated in memory.
 */
import assert from 'node:assert/strict';
import {writeFile} from 'node:fs/promises';
import * as z from 'zod';
import {compile, inspect} from '../tools/flow-atlas/domain/index.ts';
import {schema} from '../tools/flow-atlas/authoring/handles.ts';
import {zodAdapter} from '../tools/flow-atlas/authoring/adapters/zod.ts';
import {Order, StockPosition, ReservationIntent, ReservationReceipt, nativeCallCount, runCheckout} from '../tools/flow-atlas/domain/examples/native.ts';
import type {CheckoutMode} from '../tools/flow-atlas/domain/examples/native.ts';
import {createOrderModel, scenarioFromNativeRun} from '../tools/flow-atlas/domain/examples/order-stock.ts';
import {createTechnicalModel} from '../tools/flow-atlas/domain/examples/technical.ts';
import {exportArtifact} from '../tools/flow-atlas/domain/examples/export.ts';

const initialCalls=nativeCallCount();
assert.equal(initialCalls,0,'Imports perform no native commands');
const authored=createOrderModel(), technical=createTechnicalModel();
assert.equal(authored.definition.operations.find(op=>op.id==='operation.reserve')!.input.native,ReservationIntent);
assert.equal(authored.definition.operations.find(op=>op.id==='operation.confirm')!.input.native,ReservationReceipt);

// Observe actual SchemaHandle.parse entrypoints without replacing their conversion identity.
let parseHooks=0, predicateHooks=0;
const handles=[...authored.definition.values.map(v=>v.schema),...authored.definition.objects.map(o=>o.state),...authored.definition.operations.flatMap(op=>[op.input,...(op.native?[op.native.output]:[]),...op.outcomes.flatMap(outcome=>outcome.payload?[outcome.payload]:[])])];
const nativeSchemas=[...new Set(handles.map(handle=>handle.native))];
const originals=nativeSchemas.map(native=>({native,descriptor:Object.getOwnPropertyDescriptor(native,'parse')}));
try {
  for(const {native,descriptor} of originals) Object.defineProperty(native,'parse',{configurable:true,writable:true,...descriptor,value:function(){parseHooks++;throw new Error('Inspection called a native parse hook');}});
  assert(compile(authored).ok,'Authored reference compiles');
  assert(compile(technical).ok,'Technical reference compiles');
  const exported=exportArtifact();
  assert.equal(exported.format,'flow-atlas/workbench-1');
  assert.equal(nativeCallCount(),initialCalls,'Compile/inspect/export invoke zero handlers');
  assert.equal(parseHooks,0,'Compile/inspect/export invoke zero parse hooks');
  const guarded=createOrderModel() as any;
  guarded.definition.values[0].schema=schema(z.string().refine(()=>{predicateHooks++;return true;}),zodAdapter,{id:'unsupported-refinement',label:'Predicate cannot run during inspection'});
  assert(inspect(guarded).diagnostics.some(d=>d.severity==='error'),'Unsupported executable refinement is explicit');
  assert.equal(predicateHooks,0,'Inspection does not execute refinement predicates');
} finally {
  for(const {native,descriptor} of originals) {
    if(descriptor) Object.defineProperty(native,'parse',descriptor);
    else delete (native as any).parse;
  }
}

if(process.env.FLOW_ATLAS_STATIC_ONLY){
  const output={result:'passed',scope:'static-only',nativeCalls:nativeCallCount(),parseHooks,predicateHooks};
  if(process.argv[2])await writeFile(process.argv[2],JSON.stringify(output,null,2)+'\n');
  console.log(JSON.stringify(output,null,2));
}else{
const summaries=[];
for(const mode of ['success','refusal','lost-response','wrong-receipt','stale-receipt','quantity-mismatch'] satisfies CheckoutMode[]) {
  const run=runCheckout(mode), first=run.calls[0]!, reserve=run.calls.find(call=>call.stepId==='reserve')!;
  assert.equal(run.otherOrder.orderId,'order-43');
  assert.equal(run.otherOrder.status,'draft');
  assert.equal(run.otherOrder.receipt,null);
  assert.equal(run.stock.onHand,20,'Reservation never decrements onHand');
  assert.equal(run.available,run.stock.onHand-run.stock.reservations.reduce((sum,receipt)=>sum+receipt.quantity,0));
  assert.equal(first.stepId,'begin');
  assert.equal((first.before as any).status,'draft');
  assert.equal((first.after as any).status,'awaiting');
  if(mode==='success') {
    assert.equal(run.outcome,'completed'); assert.equal(run.order.status,'confirmed');
    assert.equal(run.available,18); assert.equal(run.stock.reservations.length,1);
    assert.equal(run.order.receipt!.orderId,run.order.orderId);
    assert.equal(run.order.receipt!.quantity,run.order.quantity);
    assert.deepEqual(run.calls.map(call=>call.stepId),['begin','reserve','confirm']);
  } else if(mode==='refusal') {
    assert.equal(run.outcome,'rejected'); assert.equal(run.order.status,'rejected');
    assert.equal(run.available,20); assert.equal(run.stock.reservations.length,0);
    assert.equal(reserve.outcomeId,'refused');
    assert.deepEqual(run.calls.map(call=>call.stepId),['begin','reserve','reject']);
  } else if(mode==='lost-response') {
    assert.equal(run.outcome,'unknown'); assert.equal(run.order.status,'awaiting'); assert.equal(run.order.receipt,null);
    assert.equal(run.available,18); assert.equal(run.stock.reservations.length,1);
    assert.equal((reserve.before as any).reservations.length,0);
    assert.equal((reserve.after as any).reservations.length,1,'Remote effect happened before delivery loss');
    assert.equal(reserve.completion,'local-outcome'); assert.equal(reserve.outcomeId,'unknown'); assert.equal(reserve.ownerOutcomeId,'reserved');
    assert.equal(reserve.payload,undefined,'Caller has no receipt');
    assert.deepEqual(run.calls.map(call=>call.stepId),['begin','reserve']);
  } else {
    assert.equal(run.outcome,'rejected'); assert.equal(run.order.status,'awaiting'); assert.equal(run.order.receipt,null);
    assert.equal(run.available,18); assert.equal(run.stock.reservations.length,1,'Confirmation refusal cannot undo inventory authority');
    assert.equal(run.calls.at(-1)!.outcomeId,'invalid-receipt');
    assert.deepEqual(run.calls.at(-1)!.before,run.calls.at(-1)!.after);
  }
  const projected=scenarioFromNativeRun(run), observedModel=createOrderModel();
  const checked=compile({...observedModel,scenarios:[projected]});
  assert(checked.ok,`${mode}: ${JSON.stringify(checked.diagnostics)}`);
  const checkpoints=checked.value.scenarios[0]!.checkpoints;
  if(mode==='lost-response') {
    const reservePoints=checkpoints.filter(point=>point.stepId==='reserve');
    assert.deepEqual(reservePoints.map(point=>point.phase),['call','effect','outcome']);
    const end=checkpoints.at(-1)!;
    const call=end.calls.find(item=>item.stepId==='reserve')!;
    assert.equal(call.phase,'unknown'); assert.equal(call.effectOutcomeId,'reserved');
    assert.equal(call.payload,undefined); assert.equal(call.completion,'local-outcome');
  }
  summaries.push({mode,outcome:run.outcome,order:run.order.status,available:run.available,calls:run.calls,checkpoints:checkpoints.map(point=>({phase:point.phase,stepId:point.stepId}))});
}

// Challenge the actual public owners directly, independently of Checkout's fault fixtures.
const order=new Order({orderId:'independent-order',sku:'book',quantity:2}), untouched=order.snapshot();
assert.equal(order.confirm({orderId:'independent-order',sku:'book',quantity:2,intentKey:'intent',reservationId:'receipt'}).kind,'rejected','Cannot confirm a draft order');
assert.deepEqual(order.snapshot(),untouched);
const begun=order.beginReservation({intentKey:'independent-intent'});assert.equal(begun.kind,'ready');
assert.equal(order.beginReservation({intentKey:'second-intent'}).kind,'rejected','Cannot begin twice');
const pending=order.snapshot();
const stock=new StockPosition('book',2);
assert.equal(stock.reserve({orderId:'independent-order',sku:'wrong-sku',quantity:2,intentKey:'independent-intent'}).kind,'rejected');
const reserved=stock.reserve({orderId:'independent-order',sku:'book',quantity:2,intentKey:'independent-intent'});assert.equal(reserved.kind,'reserved');
assert.equal(stock.available,0,'Exact stock boundary is valid');
assert.equal(stock.reserve({orderId:'overflow',sku:'book',quantity:1,intentKey:'overflow-intent'}).kind,'rejected');
assert.equal(stock.reserve({orderId:'independent-order',sku:'book',quantity:2,intentKey:'independent-intent'}).kind,'rejected','Used intent requires explicit recovery, not another commit');
assert.equal(stock.snapshot().reservations.length,1);
if(reserved.kind==='reserved') {
  for(const bad of [{orderId:'another-order'},{sku:'another-sku'},{quantity:1},{intentKey:'stale-intent'}]) {
    assert.equal(order.confirm({...reserved.receipt,...bad}).kind,'rejected');
    assert.deepEqual(order.snapshot(),pending,'Invalid receipt cannot change order');
  }
  assert.equal(order.confirm(reserved.receipt).kind,'confirmed');
}
const external=stock.snapshot();external.reservations[0]!.quantity=100;
assert.equal(stock.available,0,'Snapshots cannot mutate private stock');
const output={result:'passed',inspection:{nativeCalls:initialCalls,parseHooks,predicateHooks},nativeCalls:nativeCallCount(),modes:summaries};
if(process.argv[2]) await writeFile(process.argv[2],JSON.stringify(output,null,2)+'\n');
console.log(JSON.stringify(output,null,2));
}
