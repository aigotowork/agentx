/** Development-only schema probe; final build/browser execution is independently owned. */
Object.assign(globalThis,{location:{search:''}});
const {provider,agentx}=await import('../host/provider');
const {getStats}=await import('../plugin/shared/stats');
const {getTokens}=await import('../plugin/shared/tokens');
const {getDigest}=await import('../plugin/shared/digest');
const {listRecovery,continueRecovery}=await import('../plugin/shared/recovery');
const stats=await provider.invoke(getStats,{range:'30d',utcOffsetMinutes:0});
const tokens=await provider.invoke(getTokens,{range:'30d',utcOffsetMinutes:0});
const digest=await provider.invoke(getDigest,{agentId:'payment-root'});
const recovery=await provider.invoke(listRecovery,{});
const item=recovery.incidents.find(item=>!item.blockedReason)!;
const command={operationId:'mock-probe-operation',targets:[{incidentId:item.id,revision:item.revision}]};
const first=await provider.invoke(continueRecovery,command);
const replay=await provider.invoke(continueRecovery,command);
if(first.attempts[0].id!==replay.attempts[0].id)throw new Error('Recovery operation duplicated');
if(first.attempts[0].phase!=='accepted')throw new Error('Mock receipt did not remain accepted');
console.log(JSON.stringify({stats:stats.mode,tokens:tokens.mode,digest:digest.mode,recovery:first.attempts[0].phase,agents:(await agentx.agents.list()).entries.length}));
process.exit(0);
