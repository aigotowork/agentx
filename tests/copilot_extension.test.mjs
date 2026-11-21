// Execute the embedded consumer with controlled SDK/host ports, without network or process I/O.
import assert from "node:assert/strict";
import { EventEmitter } from "node:events";
import { readFileSync } from "node:fs";
import { posix } from "node:path";
import { test } from "node:test";
import { createContext, SourceTextModule, SyntheticModule } from "node:vm";

const source = readFileSync(new URL("../src/integrations/plugins/copilot/extension.mjs", import.meta.url), "utf8");
const offer = {
	name: "fixture",
	event_id: 42,
	count: 1,
	text: "exact frozen prompt",
	receipt: { id: "opaque-offer", session_id: "main", participant: { name: "fixture", incarnation: "root-1" }, frontier: 42, nested: { retain: "all fields" } },
};
const plain = (value) => JSON.parse(JSON.stringify(value));
const settle = () => new Promise((resolve) => setImmediate(resolve));

async function extension({ poll = offer, sendError = false } = {}) {
	const calls = [];
	const prompts = [];
	const handlers = new Map();
	const diagnostics = [];
	let items = [{ messageId: "sdk-1" }];
	let nextPoll = poll;
	let wake;
	const session = {
		sessionId: "main",
		on: (kind, handler) => handlers.set(kind, handler),
		send: async (input) => {
			prompts.push(plain(input));
			if (sendError) throw new Error("SDK response lost");
			return "sdk-1";
		},
		log: async () => {},
		rpc: { queue: { pendingItems: async () => ({ items }) } },
	};
	const context = createContext({
		process: { env: { AGENTX_PROCESS_ID: "fixture-process", AGENTX_DIR: "/fixture/scope" } },
		setTimeout: (callback, milliseconds) => {
			// Release the queue's explicit grace observation; never run a simulated process timeout.
			if (milliseconds === 1000) queueMicrotask(callback);
			return {};
		},
		clearTimeout: () => {},
		setInterval: () => ({ unref() {} }),
	});
	const modules = {
		"@github/copilot-sdk/extension": { joinSession: async () => session },
		"node:child_process": {
			spawn: (command, args) => {
				assert.equal(command, "fixture-agentx");
				calls.push([...args]);
				const stream = () => Object.assign(new EventEmitter(), { setEncoding() { return this; } });
				const child = Object.assign(new EventEmitter(), { stdout: stream(), stderr: stream(), kill() { assert.fail("unexpected process timeout"); } });
				const response = args[0] === "copilot-ext-poll" ? nextPoll : {};
				if (args[0] === "copilot-ext-poll") nextPoll = {};
				queueMicrotask(() => { child.stdout.emit("data", JSON.stringify(response)); child.emit("close", 0); });
				return child;
			},
		},
		"node:fs": { mkdirSync() {}, appendFileSync(_path, line) { diagnostics.push(JSON.parse(line)); } },
		"node:net": {
			createServer: (callback) => {
				wake = () => callback({ destroy() {} });
				const server = { on() { return server; }, address: () => ({ port: 41761 }), listen(_port, _host, ready) { queueMicrotask(ready); } };
				return server;
			},
		},
		"node:os": { homedir: () => "/fixture/home" },
		"node:path": { join: posix.join },
	};
	const module = new SourceTextModule(source.replace("__AGENTX_PREFIX__", '["fixture-agentx"]'), { context });
	await module.link((specifier) => {
		assert.ok(Object.hasOwn(modules, specifier), `uncontrolled import ${specifier}`);
		const exports = modules[specifier];
		return new SyntheticModule(Object.keys(exports), function () {
			for (const [key, value] of Object.entries(exports)) this.setExport(key, value);
		}, { context });
	});
	await module.evaluate();
	await settle();
	return {
		calls, prompts, diagnostics,
		async wake() { wake(); await settle(); },
		async running() { handlers.get("user.message")({ data: { messageId: "sdk-1" } }); await settle(); },
		dropQueued() { items = []; },
	};
}

function reports(runtime) {
	return runtime.calls.filter((args) => args[0] === "copilot-ext-sent");
}

function assertExactReport(args, unknown = false) {
	assert.equal(args[args.indexOf("--session-id") + 1], "main");
	assert.equal(args[args.indexOf("--event-id") + 1], "42");
	assert.deepEqual(JSON.parse(args[args.indexOf("--receipt") + 1]), offer.receipt);
	assert.equal(args.includes("--unknown"), unknown);
	assert.equal(args.includes("--ack"), false);
}

test("accepted SDK input and queued refresh preserve the full exact receipt", async () => {
	const runtime = await extension();
	assert.deepEqual(runtime.prompts, [{ prompt: offer.text, mode: "enqueue" }]);
	assert.equal(reports(runtime).length, 1);
	assertExactReport(reports(runtime)[0]);
	await runtime.wake();
	assert.equal(runtime.prompts.length, 1, "queue refresh must not send the prompt again");
	assert.equal(reports(runtime).length, 2);
	assertExactReport(reports(runtime)[1]);
});

test("missing or foreign-session receipts cannot reach the SDK or Sent", async () => {
	for (const receipt of [undefined, { id: "other", session_id: "historical" }]) {
		const runtime = await extension({ poll: { ...offer, receipt } });
		assert.deepEqual(runtime.prompts, []);
		assert.deepEqual(reports(runtime), []);
		assert.ok(runtime.diagnostics.some((entry) => entry.event === "plugin.copilot_missing_offer_receipt"));
	}
});

test("SDK uncertainty reports unknown with the original opaque receipt", async () => {
	const runtime = await extension({ sendError: true });
	assert.equal(runtime.prompts.length, 1);
	assert.equal(reports(runtime).length, 1);
	assertExactReport(reports(runtime)[0], true);
});

test("queue disappearance reports uncertainty while actual execution ends queue reports", async () => {
	const dropped = await extension();
	dropped.dropQueued();
	await dropped.wake();
	assert.equal(reports(dropped).length, 2);
	assertExactReport(reports(dropped)[1], true);
	const started = await extension();
	await started.running();
	started.dropQueued();
	await started.wake();
	assert.equal(reports(started).length, 1, "an executing prompt cannot become a dropped-queue report");
});
