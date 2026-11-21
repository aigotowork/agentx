/** Plugin lifecycle event/RPC checks; no native provider execution is claimed. */
import assert from 'node:assert/strict';
import { EventEmitter } from 'node:events';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import childProcess from 'node:child_process';
import net from 'node:net';
import { stripTypeScriptTypes, syncBuiltinESMExports } from 'node:module';
const root = fs.mkdtempSync(path.join(os.tmpdir(), 'agentx-lifecycle-plugins-'));
process.env.AGENTX_DIR = root;
process.env.AGENTX_LAUNCHED = '1';
delete process.env.AGENTX_SESSION_ID;
let start = {instructions:'FIXED', context:'INITIAL', bootstrap:'STALE_COMBINED'};
let pending = [];
const calls = [];
childProcess.spawn = (_exe, args) => {
  calls.push(args);
  const child = new EventEmitter();
  child.stdout = new EventEmitter(); child.stderr = new EventEmitter();
  child.stdout.setEncoding = child.stderr.setEncoding = () => {};
  child.kill = () => {};
  const answer = reply => { child.stdout.emit('data', JSON.stringify(reply)); child.emit('close', 0, null); };
  queueMicrotask(() => {
    if (args[0].endsWith('-context')) pending.push(answer);
    else answer(args[0].endsWith('-start') ? {name:'participant',session_id:args[args.indexOf('--session-id')+1],...start} : {ok:true,messages:[]});
  });
  return child;
};
net.createServer = () => {
  const server = new EventEmitter();
  server.listen = (...args) => queueMicrotask(args.at(-1));
  server.address = () => ({port:19381}); server.close = () => {};
  return server;
};
syncBuiltinESMExports();
async function moduleFor(tool) {
  const source = fs.readFileSync(new URL(`../../src/integrations/plugins/${tool}/agentx.ts`, import.meta.url),'utf8');
  return import(`data:text/javascript;base64,${Buffer.from(stripTypeScriptTypes(source)).toString('base64')}`);
}
async function waitPending(count) {
  for (let i=0;i<50 && pending.length<count;i++) await new Promise(resolve=>setImmediate(resolve));
  assert.equal(pending.length,count);
}
try {
  for (const tool of ['pi','omp','opencode']) {
    start = {instructions:'FIXED',context:'INITIAL',bootstrap:'STALE_COMBINED'};
    let sid = 'session-a';
    let refresh, project, system, shutdown, restart, foreign;
    const sent = [];
    if (tool !== 'opencode') {
      const handlers = new Map();
      (await moduleFor(tool)).default({on:(key,fn)=>handlers.set(key,fn),sendUserMessage:(...args)=>sent.push(args)});
      const ctx = {cwd:root,agent:{kind:'main'},sessionManager:{getSessionId:()=>sid,getSessionFile:()=>null},isIdle:()=>false,hasPendingMessages:()=>false};
      const emit = (key,event={}) => handlers.get(key)(event,ctx);
      restart = () => emit('session_start');
      await restart();
      refresh = () => emit('session_compact');
      project = async () => (emit('context',{messages:[{role:'user',content:'ordinary'}]})?.messages ?? []).filter(m=>m.role==='custom').map(m=>m.content).join('\n');
      system = async () => {
        const event = {systemPromptOptions:{appendSystemPrompt:'HOST_APPEND'},systemPrompt:['HOST_BASE']};
        await emit('before_agent_start',event);
        await emit('before_agent_start',event);
        return tool === 'pi' ? event.systemPromptOptions.appendSystemPrompt : event.systemPrompt.join('\n');
      };
      shutdown = () => emit('session_shutdown',{reason:'quit'});
      foreign = async () => { sid='foreign'; await refresh(); assert.equal(await project(),''); sid='session-a'; };
    } else {
      const hooks = await (await moduleFor(tool)).AgentxPlugin({client:{session:{status:async()=>({data:{}})},tui:{}}});
      restart = () => hooks.event({event:{type:'session.created',properties:{info:{id:sid}}}});
      await restart();
      refresh = async () => { const output={context:['EXTERNAL']}; await hooks['experimental.session.compacting']({sessionID:sid},output); return output; };
      const messages=[{info:{id:'m',sessionID:sid,role:'user'},parts:[{id:'p',type:'text',text:'ordinary'}]}];
      project = async () => { await hooks['experimental.chat.messages.transform']({}, {messages}); assert.equal(messages.length,1); return messages[0].parts.filter(p=>p.synthetic).map(p=>p.text).join('\n'); };
      system = async () => { const output={system:['HOST_BASE']}; await hooks['experimental.chat.system.transform']({sessionID:sid},output); await hooks['experimental.chat.system.transform']({sessionID:sid},output); return output.system.join('\n'); };
      shutdown = hooks.dispose;
      foreign = async () => { sid='foreign'; assert.deepEqual(await refresh(),{context:['EXTERNAL']}); sid='session-a'; };
    }
    try {
      assert((await project()).includes('INITIAL'));
      assert(!(await project()).includes('STALE_COMBINED'));
      const prompt = await system();
      assert(prompt.includes('HOST_'));
      if (tool !== 'omp') assert.equal(prompt.split('FIXED').length,2);
      else assert((await project()).includes('FIXED'));
      const baseline=calls.length;
      let older=refresh(); await waitPending(1);
      let newer=refresh(); await waitPending(2);
      pending.pop()({instructions:'MUTATED',context:'NEW'}); await newer;
      pending.pop()({instructions:'MUTATED',context:'OLD'}); await older;
      assert((await project()).includes('NEW')); assert(!(await project()).includes('OLD'));
      assert(!(await system()).includes('MUTATED'));
      const empty=refresh(); await waitPending(1); pending.pop()({instructions:'MUTATED',context:''}); await empty;
      assert(!(await project()).includes('NEW'));
      const failure=refresh(); const rejected=assert.rejects(failure,/context refresh failed/);
      await waitPending(1); pending.pop()({error:'fixture failure'}); await rejected;
      assert(!(await project()).includes('INITIAL'));
      assert(tool==='omp' ? (await project()).includes('FIXED') : (await system()).includes('FIXED'));
      await foreign();
      assert.deepEqual(sent,[]);
      assert(calls.slice(baseline).every(args=>args[0]===`${tool}-context`));
      // Session reset fences a late reply even when the same native ID is rebound.
      if (tool !== 'opencode') {
        const stale=refresh(); await waitPending(1);
        start={instructions:'',context:'',bootstrap:'STALE_COMBINED'}; await restart();
        pending.pop()({instructions:'FIXED',context:'LATE'}); await stale;
        assert.equal(await project(),''); assert(!(await system()).includes('STALE_COMBINED'));
        start={bootstrap:'LEGACY'}; await restart();
        const legacySystem=await system();
        assert(tool==='omp' ? (await project()).includes('LEGACY') : legacySystem.includes('LEGACY'));
      }
      console.log(`${tool}: lifecycle, empty/failure, reply order, session fences and user-turn invariants passed`);
    } finally { await shutdown(); }
  }
  // Exercise OpenCode 2's distinct mutable system-array channel and disposal fence.
  const hooks = new Map();
  const queue = [];
  let wake;
  const registration = {dispose() {}};
  const ctx = {
    location:{directory:root},
    session:{hook:async (key,fn)=>(hooks.set(key,fn),registration)},
    tool:{hook:async ()=>registration},
    event:{async *subscribe({signal}) {
      while (!signal.aborted) {
        if (queue.length) { yield queue.shift(); continue; }
        await new Promise(resolve=>{wake=resolve; signal.addEventListener('abort',resolve,{once:true});});
      }
    }},
  };
  start={instructions:'V2_FIXED',context:'V2_INITIAL',bootstrap:'STALE_COMBINED'};
  const cleanup = await (await moduleFor('opencode')).default.setup(ctx);
  try {
    queue.push({type:'session.created',data:{sessionID:'v2'}}); wake?.();
    // Prompt preparation waits for any outstanding binding and does not send a turn.
    for (let i=0;i<10;i++) await new Promise(resolve=>setImmediate(resolve));
    await hooks.get('prompt')({sessionID:'v2',messageID:'m',prompt:{text:'ordinary'}});
    const project = async () => {
      const draft={sessionID:'v2',system:[{type:'text',text:'BASE'}],messages:[]};
      await hooks.get('context')(draft); return draft;
    };
    assert.deepEqual((await project()).system.map(p=>p.text),['BASE','V2_FIXED','V2_INITIAL']);
    const older={sessionID:'v2',system:[{type:'text',text:'BASE'}]};
    const newer={sessionID:'v2',system:[{type:'text',text:'BASE'}]};
    const first=hooks.get('compaction')(older); await waitPending(1);
    const second=hooks.get('compaction')(newer); await waitPending(2);
    pending.pop()({instructions:'MUTATED',context:'V2_NEW'}); await second;
    pending.pop()({instructions:'MUTATED',context:'V2_OLD'}); await first;
    assert.deepEqual(older.system.map(p=>p.text),['BASE']);
    assert.deepEqual(newer.system.map(p=>p.text),['BASE','V2_FIXED\n\nV2_NEW']);
    assert.deepEqual((await project()).system.map(p=>p.text),['BASE','V2_FIXED','V2_NEW']);
    const empty=hooks.get('compaction')({sessionID:'v2',system:[]}); await waitPending(1);
    pending.pop()({instructions:'',context:''}); await empty;
    assert.deepEqual((await project()).system.map(p=>p.text),['BASE','V2_FIXED']);
    const foreign={sessionID:'foreign',system:[]}; await hooks.get('context')(foreign); await hooks.get('compaction')(foreign);
    assert.deepEqual(foreign.system,[]);
    const disposedDraft={sessionID:'v2',system:[]};
    const disposed=hooks.get('compaction')(disposedDraft); await waitPending(1);
    await cleanup(); pending.pop()({context:'LATE'}); await disposed;
    assert.deepEqual(disposedDraft.system,[]);
    assert.deepEqual((await project()).system.map(p=>p.text),['BASE']);
    console.log('opencode2: native system append, refresh order, empty snapshot and disposal fence passed');
  } finally { await cleanup(); }
} finally { fs.rmSync(root,{recursive:true,force:true}); }
