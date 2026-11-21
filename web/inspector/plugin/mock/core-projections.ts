/** Correlated simulated facts run through the original portable query/fold algorithms. */
import { StatisticsFacts } from '../core/stats-facts';
import { buildStatsSnapshot } from '../core/stats-snapshot';
import type { StatsDirectoryAgent, StatsIdentity } from '../core/stats-types';
import { admitContribution, validLedgerCall } from '../core/tokens/admission';
import { projectTokenCalls } from '../core/tokens/data';
import { buildTokenSnapshot } from '../core/tokens/snapshot';
import type { Candidate } from '../core/tokens/types';
import { compactRow, createFold, foldRows, withTimeBounds } from '../core/digest/fold';
import { buildDigest } from '../core/digest/build';
import { projectTimelineRows, type TimelinePage } from '../shared/timeline';
import type { StatsRange } from '../shared/stats';
import type { TokenSnapshot } from '../shared/tokens';
import type { InspectorAgent } from '../../host/contracts';
import { rawAgents, fixtureTimeline } from './fixtures.js';
import { fixtureTokens } from "./tokens-fixture.js";
import { fixtureDirectoryHealth } from './directory-health-fixture.js';
const agents = rawAgents as InspectorAgent[];
const now = Date.now(), DAY = 86_400_000, from = now - 72 * DAY;
const facts = new StatisticsFacts(from);
const candidates = new Map<string,Candidate>();
const iso=(at:number)=>new Date(at).toISOString();
const directory=():StatsDirectoryAgent[] => agents.map(agent=>({
 id:agent.id,createdAt:agent.createdAt,provider:agent.provider,model:agent.model??null,parentAgentId:agent.parentAgentId??null,
 projectKey:agent.projectId??`cwd:${agent.cwd}`,projectName:agent.projectName??'Mock workspace',status:agent.status,archivedAt:agent.archivedAt??null,title:agent.title??null,
 requiresAttention:agent.requiresAttention??false,attentionReason:agent.attentionReason??null,pendingPermissions:agent.pendingPermissions.length,activeTurn:!!agent.activeTurn,
 lastUserMessageAt:agent.lastUserMessageAt??null,updatedAt:agent.updatedAt,contextUsed:agent.lastUsage?.contextWindowUsedTokens??null,contextMax:agent.lastUsage?.contextWindowMaxTokens??null,
 sessionId:agent.id,cwd:agent.cwd,workspaceId:agent.workspaceId??null,
}));
// This is a finite synthetic fact set, not a fabricated native history import.
for(let day=0;day<72;day++) for(let index=0;index<agents.length;index++) {
 const agent=agents[index], at=from+day*DAY+(index%12)*3_600_000;
 if(at>now-60_000 || (day+index)%9===0)continue;
 const identity:StatsIdentity={provider:agent.provider,model:agent.model??null,projectKey:agent.projectId??null,projectName:agent.projectName??null,workspaceId:agent.workspaceId??null,metadataAt:iso(at)};
 const failed=(day+index)%17===0, duration=(3+index%7)*60_000;
 facts.point({at,agentId:agent.id,identity,metrics:{sessions:day===0?1:0,childSessions:day===0&&agent.parentAgentId?1:0,turns:1,completed:failed?0:1,failed:failed?1:0,runMs:duration,runSamples:1,contentSamples:1,contentComplete:1,toolCalls:8,toolFailures:failed?1:0,userMessages:1,assistantMessages:2,edits:2,linesAdded:20+index,linesRemoved:4,filesChanged:2,shellCalls:2,errors:failed?1:0},tools:[{name:'exec_command',kind:'shell',calls:2,failures:failed?1:0},{name:'apply_patch',kind:'edit',calls:2,failures:0},{name:'Read',kind:'read',calls:4,failures:0}]});
 facts.interval({agentId:agent.id,identity,kind:'active',start:at-duration,end:at});
 facts.longest(agent.id,agent.title??null,identity,duration,at);
 const source=agent.provider==='claude'?'claude':'codex';
 const next:Candidate={forked:false,call:{key:`mock:${day}:${agent.id}`,identity:'response',source,sessionId:agent.id,cwd:agent.cwd,model:agent.model??null,at,start:at-20_000,input:1200+index*10,cacheWrite:200,cacheRead:4200,output:850+index*5,reasoning:source==='codex'?120:0,subagent:!!agent.parentAgentId}};
 if(validLedgerCall(next.call)) {const decision=admitContribution(candidates.get(next.call.key),next);candidates.set(next.call.key,decision.value);}
}
const factsSnapshot=facts.snapshot([]);
factsSnapshot.coverage=[{start:iso(from),end:iso(now)}];
export function coreStats(range:StatsRange,utcOffsetMinutes:number,timeZone?:string|null) {
 const health=fixtureDirectoryHealth() as ReturnType<typeof buildStatsSnapshot>['directoryHealth'];
 return buildStatsSnapshot({now:Date.now(),range,utcOffsetMinutes,timeZone,facts:factsSnapshot,coverage:factsSnapshot.coverage,admission:'available',directory:directory(),directoryHealth:health,directoryObservedAt:now,ended:new Map(),running:new Map(),awaiting:new Set(),background:new Set()});
}
export function coreTokens(range:StatsRange,utcOffsetMinutes:number,timeZone?:string|null) {
 const health=fixtureDirectoryHealth() as TokenSnapshot['directoryHealth'];
 const data=projectTokenCalls(candidates.values(),new Map(),Date.now());
 const coverage=fixtureTokens(range,utcOffsetMinutes).coverage as TokenSnapshot['coverage'];
 coverage.earliestAt=iso(from);coverage.latestAt=iso(now);coverage.roots=[];
 coverage.backfill={done:true,files:candidates.size,filesDone:candidates.size,bytes:0,bytesDone:0};
 return buildTokenSnapshot(data,{range,utcOffsetMinutes,timeZone},Date.now(),{agents:directory(),loaded:true,truncated:false,health},coverage);
}
export function coreDigest(agentId:string) {
 const agent=agents.find(item=>item.id===agentId);if(!agent)throw new Error('模拟 Agent 不存在');
 const sourcePage=fixtureTimeline(agentId);
 const page={...sourcePage,truncated:sourcePage.hasOlder} as TimelinePage;
 const state=createFold(agentId,agent.cwd,page.epoch);
 foldRows(state,withTimeBounds(projectTimelineRows(page.rows).map(row=>compactRow(row,agent.cwd))));
 state.truncated=!!page.truncated||page.hasOlder;
 const history={source:(agent.provider==='claude'?'claude':'codex') as 'claude'|'codex',calls:[...candidates.values()].filter(item=>item.call.sessionId===agentId).map(item=>item.call),version:'mock-v1'};
 return buildDigest(state,{now:Date.now(),status:agent.status,activeTurn:!!agent.activeTurn},history);
}
