/** Independent plugin event harness; mocked RPC is not native provider evidence. */
import assert from 'node:assert/strict';
import { EventEmitter } from 'node:events';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import childProcess from 'node:child_process';
import net from 'node:net';
import { stripTypeScriptTypes, syncBuiltinESMExports } from 'node:module';
import { fileURLToPath } from 'node:url';

if (!process.argv.includes('--source-authorized')) throw new Error('Source-only execution requires explicit authorization; pass --source-authorized.');
const root = fs.mkdtempSync(path.join(os.tmpdir(), 'agentx-context-plugin-'));
process.env.AGENTX_DIR = root;
process.env.AGENTX_LAUNCHED = '1';
delete process.env.AGENTX_SESSION_ID;
const calls = [];
let fresh = 'FRESH_ONE';
let failure = false;
childProcess.spawn = (exe, args) => {
  assert.equal(exe, 'agentx');
  calls.push([...args]);
  const child = new EventEmitter();
  child.stdout = new EventEmitter(); child.stderr = new EventEmitter();
  child.stdout.setEncoding = child.stderr.setEncoding = () => {};
  child.kill = () => {};
  queueMicrotask(() => {
    let reply = {ok:true, messages:[], has_more:false};
    if (args[0].endsWith('-start')) reply = {name:'test-participant', session_id:args[args.indexOf('--session-id')+1], bootstrap:'OLD_BOOTSTRAP'};
    if (args[0].endsWith('-context')) reply = failure ? {error:'fixture failure'} : {context:fresh};
    child.stdout.emit('data', JSON.stringify(reply));
    child.emit('close', 0, null);
  });
  return child;
};
net.createServer = () => {
  const server = new EventEmitter();
  server.listen = (...args) => { queueMicrotask(args.at(-1)); return server; };
  server.address = () => ({port:19381});
  server.close = () => {};
  return server;
};
syncBuiltinESMExports();

async function moduleFor(tool) {
  const sourcePath = fileURLToPath(new URL(`../../src/integrations/plugins/${tool}/agentx.ts`, import.meta.url));
  const source = stripTypeScriptTypes(fs.readFileSync(sourcePath, 'utf8'));
  return import(`data:text/javascript;base64,${Buffer.from(source).toString('base64')}`);
}

for (const tool of ['pi', 'omp']) {
  const {default: install} = await moduleFor(tool);
  const handlers = new Map(); const sent = [];
  install({on:(key, fn) => handlers.set(key, fn), sendUserMessage:(...args) => sent.push(args)});
  let sid = 'session-a';
  const ctx = {cwd:root, agent:{kind:'main'}, sessionManager:{getSessionId:()=>sid, getSessionFile:()=>null}, isIdle:()=>false, hasPendingMessages:()=>false};
  const event = key => { assert(handlers.has(key), `missing ${tool} ${key}`); return handlers.get(key); };
  await event('session_start')({}, ctx);
  const baseline = calls.length;
  for (const text of ['FRESH_ONE','FRESH_TWO','']) {
    fresh = text;
    await event('session_compact')({}, ctx);
    for (let i = 0; i < 2; i++) {
      const original = {role:'user', content:'existing'};
      const result = event('context')({messages:[original]}, ctx);
      if (text) {
        assert.equal(result.messages.length, 2);
        assert.equal(result.messages[0], original);
        assert.equal(result.messages[1].content, text);
        assert.equal(result.messages[1].display, false);
      } else assert.equal(result, undefined);
    }
  }
  fresh = 'BEFORE_FAILURE'; await event('session_compact')({}, ctx);
  failure = true;
  await assert.rejects(event('session_compact')({}, ctx), /context refresh failed/);
  assert.equal(event('context')({messages:[]}, ctx), undefined, 'stale recovery must be cleared on failure');
  failure = false;
  fresh = 'SESSION_A'; await event('session_compact')({}, ctx);
  sid = 'session-b';
  const beforeForeign = calls.length;
  await event('session_compact')({}, ctx);
  assert.equal(event('context')({messages:[]}, ctx), undefined);
  assert.equal(calls.length, beforeForeign, 'foreign session must not fetch');
  assert.deepEqual(sent, [], 'compact must not synthesize a user turn');
  assert(calls.slice(baseline).every(args => args[0] === `${tool}-context`), 'compact must not write status or acknowledge mail');
  if (tool === 'omp') await event('session_shutdown')({}, ctx);
  else await event('session_shutdown')({reason:'exit'}, ctx);
  console.log(JSON.stringify({tool, result:'passed', evidence:'mocked plugin events/RPC only'}));
}

const {AgentxPlugin} = await moduleFor('opencode');
const hooks = await AgentxPlugin({client:{session:{status:async()=>({data:{}}),create:async()=>({data:{id:'session-a'}})},tui:{publish:async()=>({})}}});
try {
  await hooks.event({event:{type:'session.created',properties:{info:{id:'session-a'}}}});
  const baseline = calls.length;
  for (const text of ['OC_ONE','OC_TWO','']) {
    fresh = text; const output = {context:['retained']};
    await hooks['experimental.session.compacting']({sessionID:'session-a'}, output);
    assert.deepEqual(output.context, text ? ['retained',text] : ['retained']);
    const messages = [{info:{id:'message-a',sessionID:'session-a',role:'user'},parts:[{id:'part-a',type:'text',text:'ordinary prompt'}]}];
    await hooks['experimental.chat.messages.transform']({}, {messages});
    assert.deepEqual(messages[0].parts.filter(p => p.synthetic).map(p => p.text), text ? [text] : [], 'next ordinary request must use fresh projection');
    assert.equal(messages.length, 1, 'refresh must not create a user turn');
  }
  const foreign = {context:[]};
  await hooks['experimental.session.compacting']({sessionID:'foreign'}, foreign);
  assert.deepEqual(foreign.context, []);
  assert(calls.slice(baseline).every(args => args[0] === 'opencode-context'), 'compact must not write status/ack or send messages');
  fresh = 'OC_BEFORE_FAILURE';
  await hooks['experimental.session.compacting']({sessionID:'session-a'}, {context:[]});
  failure = true;
  const failedOutput = {context:['retained-external-context']};
  await assert.rejects(hooks['experimental.session.compacting']({sessionID:'session-a'}, failedOutput), /context refresh failed/);
  assert.deepEqual(failedOutput.context, ['retained-external-context'], 'failed refresh must not inject stale context');
  const afterFailure = [{info:{id:'after-failure',sessionID:'session-a',role:'user'},parts:[{id:'ordinary',type:'text',text:'ordinary prompt'}]}];
  await hooks['experimental.chat.messages.transform']({}, {messages:afterFailure});
  assert.deepEqual(afterFailure[0].parts.filter(p => p.synthetic), [], 'next ordinary request must not replay cached context after refresh failure');
  failure = false;
  fresh = 'OC_RECOVERED';
  const recovered = {context:[]};
  await hooks['experimental.session.compacting']({sessionID:'session-a'}, recovered);
  assert.deepEqual(recovered.context, ['OC_RECOVERED']);
  console.log(JSON.stringify({tool:'opencode',result:'passed',evidence:'mocked v1 hook/RPC only'}));
} finally { await hooks.dispose(); }
console.log(JSON.stringify({fixture:root, calls}));
