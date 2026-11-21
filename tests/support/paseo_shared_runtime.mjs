// Exercise the installed, unmodified Paseo client/provider API. No provider
// monkeypatches, fake RPC clients or live Paseo host are involved. The Python
// acceptance runner supplies the isolated environment and fixed local model.
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { createRequire } from "node:module";
import { pathToFileURL } from "node:url";
import { spawnSync } from "node:child_process";

const [providerPath, root, scenario] = process.argv.slice(2);
assert.equal(process.env.AGENTX_ACCEPTANCE_NAMESPACE, "isolated-v1");
const require = createRequire(pathToFileURL(providerPath));
const pino = require("pino");
const { CodexAppServerAgentClient } = await import(pathToFileURL(providerPath));
const packagePath = path.resolve(path.dirname(providerPath), "../../../../../package.json");
const packageInfo = JSON.parse(fs.readFileSync(packagePath, "utf8"));
const logger = pino({ level: "trace" }, pino.destination(path.join(root, "paseo-provider.log")));
const agentx = path.join(root, "bin/agentx");
const events = [];
const eventPath = path.join(root, "paseo-events.jsonl");

function eventMatches(event, text) {
  return event.type === "timeline" && JSON.stringify(event.item).includes(text);
}

async function until(check, label, timeoutMs = 25000) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const result = check();
    if (result) return result;
    await new Promise((resolve) => setTimeout(resolve, 20));
  }
  throw new Error(`Timed out waiting for ${label}; inspect paseo-events.jsonl and provider log`);
}

function runAgentx(args) {
  const result = spawnSync(agentx, args, { env: process.env, cwd: path.join(root, "project"), encoding: "utf8", timeout: 10000 });
  assert.equal(result.status, 0, `${JSON.stringify(args)}: ${result.stderr} ${result.stdout}`);
  return result.stdout;
}

function nativeRequests() {
  const trace = path.join(root, "trace.jsonl");
  return fs.readFileSync(trace, "utf8").trim().split("\n").filter(Boolean).map((line) => JSON.parse(line)).filter((event) => event.source === "model");
}

const client = new CodexAppServerAgentClient(logger, {
  command: { mode: "replace", argv: [agentx, "launch", "--tool", "codex", "--stdio", "--go", "--"] },
  env: { ...process.env },
});
let session;
let unsubscribe;
const result = { status: "failed", scenario, providerPackage: { name: packageInfo.name, version: packageInfo.version, path: packagePath }, providerPath, providerModified: false, guiRendering: "not exercised" };
try {
  session = await client.createSession({ provider: "codex", cwd: path.join(root, "project"), model: "fixture-model", modeId: scenario === "parent-child" ? "full-access" : "auto", providerOptions: { sandbox_mode: scenario === "parent-child" ? "danger-full-access" : "workspace-write" } }, { env: { ...process.env }, agentId: "shared-runtime-acceptance" });
  unsubscribe = session.subscribe((event) => {
    const retained = { seq: events.length, timeMs: Date.now(), ...event };
    events.push(retained);
    fs.appendFileSync(eventPath, JSON.stringify(retained) + "\n");
  });
  if (scenario === "parent-child") {
    result.initialRun = await session.run("SHARED_PARENT_INITIAL");
    assert(events.some((event) => event.type === "turn_completed"), "Paseo never observed initial completion");
    const mark = events.length;
    fs.closeSync(fs.openSync(path.join(root, "parent-idle"), "w"));
    // From this point until the autonomous cycle finishes, no API submits
    // another prompt/turn or polls the AgentX inbox on behalf of the parent.
    const inbound = await until(() => events.slice(mark).find((event) => eventMatches(event, "SHARED_CHILD_REPLY") && event.item.type === "user_message"), "Paseo autonomous user-message timeline");
    const continued = await until(() => events.slice(inbound.seq).find((event) => eventMatches(event, "PARENT_COMPLETED_ROUND_TRIP") && event.item.type === "assistant_message"), "Paseo autonomous assistant output");
    const complete = await until(() => events.slice(continued.seq).find((event) => event.type === "turn_completed"), "Paseo autonomous completion");
    await until(() => fs.existsSync(path.join(root, "child-done.json")), "native child business reply");
    result.autonomous = { input: inbound, assistant: continued, completion: complete, apiSubmittedPrompts: 1 };
    result.child = JSON.parse(fs.readFileSync(path.join(root, "child-done.json"), "utf8"));
  } else {
    const mark = events.length;
    await session.startTurn("PASEO_APPROVAL_CASE");
    const pending = await until(() => events.slice(mark).find((event) => event.type === "permission_requested"), "Paseo native approval request");
    const helper = path.resolve(path.dirname(new URL(import.meta.url).pathname), "codex_shared_child.py");
    const lookup = spawnSync("/usr/bin/python3", [helper, "lookup", root, session.id], { env: process.env, encoding: "utf8", timeout: 10000 });
    assert.equal(lookup.status, 0, lookup.stderr);
    const participant = lookup.stdout.trim();
    assert(JSON.parse(runAgentx(["list", "--json"])).some((entry) => entry.name === participant));
    runAgentx(["start", "--as", "lobo", "--iam", "lobo"]);
    runAgentx(["send", "--from", "fixture-child", "--iam", "lobo", "--intent", "inform", `@${participant}`, "--", "PASEO_HELD_MAIL"]);
    await new Promise((resolve) => setTimeout(resolve, 300));
    assert(!nativeRequests().some((event) => JSON.stringify(event.value.body).includes("PASEO_HELD_MAIL")), "pending mail bypassed native approval");
    assert(!fs.existsSync(path.join(root, "approval-effect.txt")), "approval tool ran before the client's answer");
    await session.respondToPermission(pending.request.id, { behavior: "allow" });
    await until(() => fs.existsSync(path.join(root, "approval-effect.txt")), "approved tool effect");
    const inbound = await until(() => events.slice(mark).find((event) => eventMatches(event, "PASEO_HELD_MAIL") && event.item.type === "user_message"), "mail after approval");
    await until(() => events.slice(inbound.seq).find((event) => event.type === "turn_completed"), "mail turn completion");
    const cancelMark = events.length;
    await session.startTurn("HOLD_CASE");
    await until(() => fs.existsSync(path.join(root, "model-held")), "held real native turn");
    await session.interrupt();
    const canceled = await until(() => events.slice(cancelMark).find((event) => event.type === "turn_canceled"), "Paseo cancel event");
    result.controls = { approval: pending, mailAfterApproval: inbound, canceled };
  }
  result.status = "passed";
  result.nativeThread = session.id;
  result.eventCount = events.length;
} catch (error) {
  result.failure = error?.stack ?? String(error);
  process.exitCode = 1;
} finally {
  unsubscribe?.();
  try { await session?.close(); } catch (error) { result.cleanupFailure = error?.stack ?? String(error); process.exitCode = 1; }
  fs.writeFileSync(path.join(root, "paseo-result.json"), JSON.stringify(result, null, 2) + "\n");
  logger.flush();
}
