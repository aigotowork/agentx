/** @aiPurpose Check decisive false-success witnesses and emit a diagnostic-only browser fixture.
 * @aiUsage Generated drafts are for diagnostic navigation acceptance, never the accepted publication artifact.
 */
import assert from 'node:assert/strict';
import {writeFile} from 'node:fs/promises';
import {compile,inspect} from '../tools/flow-atlas/domain/index.ts';
import {createOrderModel} from '../tools/flow-atlas/domain/examples/order-stock.ts';

const cases=[];
for(const [field,value] of [['quantity',3],['orderId','order-43']] as const) {
  const model=createOrderModel() as any;
  model.scenarios[0].visits.find((visit:any)=>visit.stepId==='confirm').input[field]=value;
  const result=compile(model);
  assert(!result.ok);
  assert(result.diagnostics.some(d=>d.code==='RULE_VIOLATION'));
  assert(result.diagnostics.some(d=>d.code==='PARAMETER_MISMATCH'));
  cases.push({name:`false-success-${field}`,diagnostics:result.diagnostics});
}
const unknown=createOrderModel() as any;
unknown.scenarios.find((s:any)=>s.id==='scenario.lost-response').terminalStepId='completed';
const unknownCheck=compile(unknown);
assert(!unknownCheck.ok);
assert(unknownCheck.diagnostics.some(d=>d.code==='UNKNOWN_AS_SUCCESS'));
cases.push({name:'unknown-is-not-success',diagnostics:unknownCheck.diagnostics});

const branch=createOrderModel() as any;
branch.definition.processes[0].steps.find((step:any)=>step.id==='confirm').bindings[0].source.outcomeId='refused';
const draft=inspect(branch);
assert(draft.diagnostics.some(d=>d.code==='WRONG_BRANCH'||d.code==='UNAVAILABLE_RESULT'));
cases.push({name:'wrong-result-branch',diagnostics:draft.diagnostics});
if(process.argv[2])await writeFile(process.argv[2],JSON.stringify(draft,null,2)+'\n');
console.log(JSON.stringify({result:'passed',cases},null,2));
