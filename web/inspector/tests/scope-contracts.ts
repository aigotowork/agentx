/** Affected semantic checks: reject removed operations, keep Token exports, and never queue/replay uncertain sends. */
import assert from 'node:assert/strict';
import {z} from 'zod';
Object.assign(globalThis,{location:{search:''}});
const {provider}=await import('../host/provider');
const {createFixtureLedger}=await import('../plugin/mock/messages-fixture.js');
const {analysisReportExport}=await import('../plugin/client/analysis-evidence-model');
const {fixtureInsight}=await import('../plugin/mock/fixtures.js');
const {analysisEvidenceFixture}=await import('../plugin/mock/analysis-evidence-fixture.js');
for(const name of ['observatory.quota.get','observatory.schedules.list','observatory.schedules.heartbeats','observatory.schedules.run','observatory.resume.schedule','observatory.resume.list','observatory.resume.cancel','observatory.deferred.enqueue','observatory.deferred.send','observatory.execution.snapshot','observatory.bots.automation'])await assert.rejects(()=>provider.invoke({name,input:z.object({}),output:z.unknown()},{}));
const exported=analysisReportExport(analysisEvidenceFixture(fixtureInsight('payment-root'),'zero'));
assert.deepEqual(exported.usage,{inputTokens:0,outputTokens:0});assert(!JSON.stringify(exported).includes('costUsd'));
let delivered=0;
const ledger=createFixtureLedger({agent:()=>({status:'idle',provider:'codex',pendingPermissions:[]}),deliver:async()=>{delivered++;throw new Error('agent_request_outcome_unknown')},capabilities:{steerable:[],slashSteerable:[]}});
assert.deepEqual(Object.keys(ledger).sort(),['list','send']);
const input={agentId:'mock',messageId:'same-message-id',key:'message-key',source:'test',text:'continue',consent:null};
const first=await ledger.send(input);assert.equal(first.view.state,'maybe-sent');await ledger.send(input);assert.equal(delivered,1);assert.equal(ledger.list({agentId:'mock'}).length,1);
console.log(JSON.stringify({status:'passed',removedOperations:11,export:'Token-only',unknownSend:'one delivery, no queue or replay'}));process.exit(0);
