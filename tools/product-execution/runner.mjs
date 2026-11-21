#!/usr/bin/env node
// The trusted peer owns these two private child pipes, the journal and mTLS.
import { spawn } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { closeSync, constants, openSync, readFileSync, readSync, realpathSync, statSync, writeFileSync } from 'node:fs';
import { request } from 'node:https';
import { dirname, isAbsolute, relative, resolve } from 'node:path';
import { createInterface } from 'node:readline';
import { DatabaseSync } from 'node:sqlite';
import { setTimeout as pause } from 'node:timers/promises';

const digest = bytes => 'sha256:' + createHash('sha256').update(bytes).digest('hex');
const need = (condition, message) => { if (!condition) throw Error(message); };
const canonicalize = value => Array.isArray(value) ? value.map(canonicalize) : value && typeof value === 'object'
  ? Object.fromEntries(Object.entries(value).sort(([a], [b]) => a.localeCompare(b)).map(([key, item]) => [key, canonicalize(item)])) : value;
const canonical = value => JSON.stringify(canonicalize(value));
const same = (a, b) => canonical(a) === canonical(b);
const id = value => typeof value === 'string' && value.length > 0;
const settings = JSON.parse(readFileSync(process.argv[2] ?? '', 'utf8'));
process.umask(0o077);
const absolute = value => { need(typeof value === 'string' && isAbsolute(value), 'absolute configuration path required'); return realpathSync(value); };
const inside = (root, path) => {
  const rest = relative(root, path);
  return rest !== '' && rest !== '..' && !rest.startsWith('../') && !isAbsolute(rest);
};
const scope = absolute(settings.scope), workspace = absolute(settings.workspace), home = absolute(settings.nativeHome);
const journalDir = absolute(dirname(settings.journal));
need(inside(journalDir, resolve(settings.journal)) && (statSync(journalDir).mode & 0o077) === 0, 'private journal directory required');
const evidence = new DatabaseSync(settings.journal);
evidence.exec("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; CREATE TABLE IF NOT EXISTS journal (key TEXT PRIMARY KEY, payload TEXT NOT NULL)");
const load = evidence.prepare('SELECT payload FROM journal WHERE key = ?');
const insert = evidence.prepare('INSERT INTO journal VALUES (?, ?)');
function remember(key, payload) {
  const value = canonical(payload);
  const previous = load.get(key);
  if (previous) need(previous.payload === value, 'different information at an original journal key');
  else insert.run(key, value);
}
// Native processes are single-owner. A restarted runner cannot assume that a
// vanished pipe was drained or adopt it as a new execution.
need(!load.get('runner'), 'journal already owns an execution; preserve it for reconciliation');
remember('runner', { version: 1, at: new Date().toISOString() });

const endpoint = new URL(settings.privateOrigin);
need(endpoint.protocol === 'https:' && endpoint.origin === settings.privateOrigin && !endpoint.username && !endpoint.password, 'dedicated HTTPS origin required');
const peer = { ca: readFileSync(absolute(settings.ca)), cert: readFileSync(absolute(settings.cert)), key: readFileSync(absolute(settings.key)) };
const catalog = JSON.parse(readFileSync(absolute(settings.toolWire), 'utf8')).installation;
need(catalog.digest === settings.installationDigest && catalog.digest === digest(canonical(catalog.declarations.map(d => [d.name, d.inputSchema, d.outputSchema]))), 'tool installation digest mismatch');
need(same(catalog.names, catalog.declarations.map(d => d.name)) && new Set(catalog.names).size === catalog.names.length, 'invalid tool installation names');
const declarations = catalog.declarations.map(d => ({ type: 'function', name: d.name, inputSchema: d.inputSchema, description: 'Use the registered Hypit collaboration capability.' }));

function post(path, value, call) {
  const bytes = Buffer.from(JSON.stringify(value));
  const headers = { 'content-type': 'application/json', 'content-length': bytes.length };
  if (call) {
    headers.accept = 'application/json, text/event-stream';
    headers['mcp-protocol-version'] = '2025-03-26';
    headers['x-hypit-agentx-invocation'] = Buffer.from(JSON.stringify(call)).toString('base64url');
  }
  return new Promise((ok, reject) => {
    const outgoing = request(new URL(path, endpoint), { ...peer, method: 'POST', headers, agent: false }, response => {
      let size = 0;
      const parts = [];
      response.on('data', bytes => {
        size += bytes.length;
        if (size > 2 * 1024 * 1024) outgoing.destroy(Error('oversized private response'));
        else parts.push(bytes);
      });
      response.once('error', reject);
      response.once('end', () => ok({ code: response.statusCode, text: Buffer.concat(parts).toString('utf8'), type: response.headers['content-type'] }));
    });
    const deadline = setTimeout(() => outgoing.destroy(Error('private request deadline exceeded')), 55_000);
    outgoing.once('close', () => clearTimeout(deadline));
    outgoing.once('error', reject);
    outgoing.end(bytes);
  });
}

// JSON-RPC server request identities retain the original int64 or string even
// when JavaScript cannot represent the input integer as a Number.
function decode(line) {
  const value = JSON.parse(line);
  if (Object.hasOwn(value, 'id')) {
    if (typeof value.id === 'number') {
      const raw = JSON.parse(line, (key, item, context) => key === 'id' && typeof item === 'number' ? context?.source : item);
      const source = raw.id;
      need(/^-?[0-9]+$/.test(source), 'unsupported native request ID');
      const integer = BigInt(source);
      need(integer >= -(1n << 63n) && integer < (1n << 63n), 'out-of-range native request ID');
      return { value, token: source };
    }
    need(typeof value.id === 'string', 'unsupported native request ID');
    return { value, token: JSON.stringify(value.id) };
  }
  return { value };
}

function pipe(program, args, env) {
  const child = spawn(program, args, { cwd: workspace, env, stdio: ['pipe', 'pipe', 'inherit'] });
  let fail;
  const ended = new Promise((_, reject) => { fail = reject; });
  ended.catch(() => {});
  child.stdin.on('error', fail);
  child.once('error', fail);
  child.once('exit', (code, signal) => fail(Error('dedicated child exited: ' + (signal ?? code))));
  const lines = createInterface({ input: child.stdout, crlfDelay: Infinity })[Symbol.asyncIterator]();
  return {
    child, ended,
    async read() {
      const { done, value } = await Promise.race([lines.next(), ended]);
      need(!done && value.length <= 2 * 1024 * 1024, 'closed or oversized dedicated pipe');
      return value;
    },
    async send(message) {
      await Promise.race([new Promise((ok, no) => child.stdin.write(JSON.stringify(message) + '\n', error => error ? no(error) : ok())), ended]);
    },
  };
}
const localEnv = { PATH: '/usr/bin:/bin', LANG: 'C.UTF-8', HOME: journalDir };
const authority = pipe(absolute(settings.agentx), ['__product-execution', scope, workspace, home, settings.name, settings.role], localEnv);
let queue = Promise.resolve();
const unwrap = line => {
  const envelope = JSON.parse(line);
  need(envelope.ok === true, 'AgentX authority: ' + envelope.error);
  return envelope.value;
};
function claim(method, fields = {}) {
  const result = queue.then(async () => {
    await authority.send({ method, ...fields });
    return unwrap(await authority.read());
  });
  queue = result.catch(() => {});
  return result;
}

let runtime, active, registration, stopRefresh, stopRequested = false;
const halt = () => { stopRequested = true; runtime?.child.kill('SIGTERM'); };
process.on('SIGTERM', halt);
process.on('SIGINT', halt);

async function serve() {
  const { instructions } = unwrap(await authority.read());
  // This explicit executable is the operator's isolated native launch command.
  // It receives neither the peer TLS files nor the daemon/journal path.
  runtime = pipe(absolute(settings.nativeLauncher), Object.freeze(settings.nativeArgs ?? []), { PATH: '/usr/bin:/bin', LANG: 'C.UTF-8' });
  const pending = new Map();
  let serial = 0;
  const input = [];
  let waiter;
  const reading = (async () => {
    while (true) {
      const frame = decode(await runtime.read());
      const key = frame.value.id;
      if (!frame.value.method && pending.has(key)) {
        const accept = pending.get(key);
        pending.delete(key);
        accept(frame.value);
      } else {
        need(input.length < 256, 'native frame queue exceeded');
        input.push(frame);
        waiter?.();
        waiter = undefined;
      }
    }
  })();
  reading.catch(() => {});
  async function rpc(method, params, timeout = 30_000) {
    const requestId = 'runner:' + ++serial;
    const result = new Promise(ok => pending.set(requestId, ok));
    await runtime.send({ id: requestId, method, params });
    const deadline = AbortSignal.timeout(timeout);
    const expiry = new Promise((_, no) => deadline.addEventListener('abort', () => no(Object.assign(Error('native RPC deadline: ' + method), { code: 'NATIVE_RPC_TIMEOUT' })), { once: true }));
    try {
      const reply = await Promise.race([result, reading, expiry]);
      need(!reply.error && reply.result, 'native RPC refused: ' + method);
      return reply.result;
    } finally { pending.delete(requestId); }
  }
  const initialize = await rpc('initialize', { clientInfo: { name: 'agentx_product_execution', version: '1' }, capabilities: { experimentalApi: true } });
  need(initialize.codexHome === home, 'unexpected native home');
  await runtime.send({ method: 'initialized', params: {} });
  const root = (await rpc('thread/start', {
    cwd: workspace, developerInstructions: instructions, dynamicTools: declarations, ephemeral: false,
    approvalPolicy: 'never', sandbox: 'read-only', allowProviderModelFallback: false,
  })).thread;
  need(root?.cliVersion === settings.nativeVersion && root.cwd === workspace && root.status?.type === 'idle' && id(root.id), 'native root differs from the dedicated registration');
  const thread = root.id;
  await rpc('thread/name/set', { threadId: thread, name: 'AgentX product execution' });
  const sessions = absolute(resolve(home, 'sessions'));
  const rollout = absolute(root.path);
  need(inside(sessions, rollout), 'native installation path escaped the dedicated home');
  function installed() {
    need(absolute(root.path) === rollout && inside(sessions, rollout), 'native installation path changed');
    const descriptor = openSync(rollout, constants.O_RDONLY | constants.O_NOFOLLOW);
    let prefix;
    try {
      const bytes = Buffer.alloc(1_048_576);
      const read = readSync(descriptor, bytes);
      const newline = bytes.subarray(0, read).indexOf(10);
      need(newline > 0, 'native installation record exceeds bound');
      prefix = bytes.toString('utf8', 0, newline);
    } finally { closeSync(descriptor); }
    const meta = JSON.parse(prefix);
    need(meta.type === 'session_meta' && meta.payload?.id === thread && same(meta.payload.dynamic_tools, declarations), 'native installation changed');
  }
  installed();
  registration = await claim('bind', { thread });
  registration.execution.installationDigest = catalog.digest;
  const execution = registration.execution;
  const installation = { kind: 'agentx-native-tool-installation@1', nativeThreadRef: thread,
    nativeInstallationRef: digest(canonical([execution.executionId, thread, catalog.digest])), toolNames: catalog.names };
  remember('registration', registration);
  remember('installation', installation);
  writeFileSync(settings.registration, JSON.stringify(registration) + '\n', { flag: 'wx', mode: 0o600 });
  let refreshError;
  async function refresh() {
    await claim('renew');
    need((await rpc('thread/read', { threadId: thread, includeTurns: false })).thread?.id === thread, 'native thread lost');
    installed();
    const result = await post('/agentx-private/installation', { execution, observedAt: new Date().toISOString(), source: installation });
    need(result.code === 204 || result.code === 403, 'unexpected installation response');
    return result.code === 204;
  }
  const refreshLoop = async () => {
    while (!stopRequested) {
      try { ready = await refresh(); }
      catch (error) {
        ready = false;
        // Read/transport timeouts suspend readiness. They do not replace the
        // native thread or permit a forced installation update.
        if (!['ECONNREFUSED', 'ECONNRESET', 'NATIVE_RPC_TIMEOUT'].includes(error.code)) { refreshError = error; break; }
      }
      await pause(8_000, undefined, { ref: false, signal: stopRefresh.signal }).catch(() => {});
    }
  };
  let ready = false;
  stopRefresh = new AbortController();
  void refreshLoop();

  async function event(kind, fields) {
    const payload = { conversationId: active.location.conversationId, event: {
      kind, eventId: randomUUID(), sequence: ++active.sequence, turnId: active.location.turnId, nativeTurnRef: active.turn, execution, ...fields,
    } };
    const key = active.batch.batchId + ':event:' + payload.event.sequence;
    remember(key, payload);
    for (let attempt = 0; attempt < 3; attempt++) {
      const result = await post('/agentx-private/events', payload).catch(() => null);
      if (result?.code === 204) {
        remember(key + ':accepted', true);
        return;
      }
      if (result && result.code < 500) break;
      await pause(350);
    }
    throw Error('event acknowledgement unknown: ' + kind);
  }

  async function locate(messageId) {
    for (let attempt = 0; attempt < 60; attempt++) {
      const result = await post('/agentx-private/turns/locate', { execution, messageId }).catch(() => null);
      if (result?.code === 200) {
        const found = JSON.parse(result.text);
        need(id(found.turnId) && id(found.conversationId), 'invalid product locator');
        return found;
      }
      if (result && result.code !== 404 && result.code < 500) break;
      await pause(400);
    }
    throw Error('original Sent has no accepted Hypit locator');
  }

  async function invoke(frame) {
    const { value: { params }, token } = frame;
    need(active.proof && !active.outstanding && !active.terminal && id(params.callId) && declarations.some(d => d.name === params.tool)
      && (params.namespace === null || params.namespace === undefined), 'native tool is outside the installed active turn');
    const invocationId = digest(canonical([execution.executionId, thread, active.turn, token, params.callId]));
    const call = { ...active.location, execution, nativeTurnRef: active.turn, invocationId, sequence: active.frontier + 1, tool: params.tool,
      argumentsDigest: digest(canonical(params.arguments)) };
    const key = active.batch.batchId + ':call:' + call.sequence;
    const body = { jsonrpc: '2.0', id: invocationId, method: 'tools/call', params: { name: params.tool, arguments: params.arguments } };
    remember(key, { call, body, nativeRequestId: token, nativeCallId: params.callId });
    let response;
    for (let attempt = 0; attempt < 3; attempt++) {
      response = await post('/agentx-private/tools/' + call.tool, body, call).catch(() => null);
      if (response && response.code < 500) break;
      await pause(400);
    }
    need(response?.code === 200, 'MCP effect unconfirmed');
    const messages = response.type?.includes('text/event-stream')
      ? response.text.split(/\r?\n\r?\n/).map(block => block.split(/\r?\n/).filter(line => line.startsWith('data:')).map(line => line.slice(5)).join('\n')).filter(Boolean).map(line => JSON.parse(line))
      : [JSON.parse(response.text)];
    const message = messages.find(message => message.id === invocationId);
    need(message?.jsonrpc === '2.0' && !message.error && message.result?.isError === false && message.result?.structuredContent, 'MCP result unconfirmed');
    const result = { contentItems: [{ type: 'inputText', text: JSON.stringify(message.result.structuredContent) }], success: true };
    remember(key + ':response', result);
    active.outstanding = { nativeCallId: params.callId, name: params.tool, result, key, sequence: call.sequence };
    await runtime.send({ id: JSON.rawJSON(token), result });
  }

  async function consume(frame) {
    const { method, params } = frame.value;
    if (frame.token !== undefined && method !== 'item/tool/call') throw Error('unsupported native server request');
    if (!method || params?.threadId !== thread || !active) return;
    const nativeTurn = params.turnId ?? params.turn?.id;
    if (nativeTurn !== active.turn) {
      need(method !== 'item/tool/call', 'native tool did not belong to the active turn');
      return;
    }
    if (method === 'item/tool/call') {
      await invoke(frame);
    } else if (method === 'item/completed') {
      const item = params.item;
      if (item.type === 'userMessage') {
        need(!active.proof, 'multiple native user items in the dedicated turn');
        const proof = await claim('record', { thread, turn: active.turn, item });
        remember(active.batch.batchId + ':native-input', proof);
        await event('input-recorded', { messageId: proof.message_id, source: { kind: 'agentx-native-inbox@1', batchId: active.batch.batchId,
          requestId: proof.request_id, memberMessageIds: [proof.message_id], nativeThreadRef: proof.thread_id,
          nativeInputItemRef: proof.item_id, payloadDigest: proof.payload_digest } });
        active.proof = true;
      } else if (item.type === 'dynamicToolCall') {
        const call = active.outstanding;
        need(call && call.nativeCallId === item.id && call.name === item.tool && item.status === 'completed'
          && item.success === true && same(call.result.contentItems, item.contentItems), 'native tool response fence missing');
        remember(call.key + ':native-completion', { item: item.id, outputDigest: digest(canonical(item.contentItems)) });
        active.frontier = call.sequence;
        active.outstanding = null;
      } else if (item.type === 'agentMessage' && id(item.id) && id(item.text) && item.phase !== 'commentary') {
        active.replies.set(item.id, item.text);
      }
    } else if (method === 'turn/completed') {
      const turn = params.turn;
      need(active.proof && !active.terminal && !active.outstanding && ['completed', 'failed', 'interrupted'].includes(turn.status), 'native terminal cannot settle the active turn');
      active.terminal = true;
      const finalItems = (turn.items ?? []).filter(item => item.type === 'agentMessage' && item.phase !== 'commentary' && id(item.text));
      const latest = finalItems.at(-1);
      if (latest) {
        need(active.replies.get(latest.id) === latest.text, 'native final reply did not have a matching completed item');
        await event('reply-observed', { replyId: latest.id, text: latest.text });
      }
      await event('terminal', { outcome: turn.status, toolFrontier: active.frontier });
      await event('drained', { toolFrontier: active.frontier });
      await claim('finish');
      console.log(JSON.stringify({ outcome: turn.status, drained: true, executionId: execution.executionId, msg: active.batch.messageId, toolFrontier: active.frontier }));
      active = null;
    }
  }

  while (!stopRequested) {
    if (refreshError) throw refreshError;
    await Promise.race([reading, authority.ended, pause(250)]);
    while (input.length) await consume(input.shift());
    if (!ready || active) continue;
    const batch = await claim('prepare');
    if (!batch) continue;
    const location = await locate(batch.messageId);
    active = { batch, location, sequence: 0, frontier: 0, replies: new Map() };
    remember(batch.batchId + ':source', { requestId: batch.requestId, messageId: batch.messageId, payloadDigest: digest(batch.content), location });
    await claim('issue');
    remember(batch.batchId + ':issuing', true);
    const started = await rpc('turn/start', { threadId: thread, input: [{ type: 'text', text: batch.content }], clientUserMessageId: batch.clientId });
    need(id(started.turn?.id), 'native turn admission unknown');
    active.turn = started.turn.id;
    remember(batch.batchId + ':turn', { thread, turn: active.turn });
  }
}

try {
  await serve();
} catch (error) {
  if (!stopRequested || active) {
    remember('stopped-unknown', { activeBatch: active?.batch.batchId ?? null, reason: error.message });
    console.error(error);
    process.exitCode = 1;
  }
} finally {
  stopRequested = true;
  stopRefresh?.abort();
  runtime?.child.kill('SIGTERM');
  await claim('close').catch(() => {});
  authority.child.stdin.end();
  evidence.close();
}
