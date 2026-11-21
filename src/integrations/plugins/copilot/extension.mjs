// agentx delivery for an idle Copilot CLI session (`copilot_delivery = native`, the default).
//
// Shipped in agentx's per-run plugin only for extension-delivery launches. Hooks
// still own everything mid-turn (PostToolUse context, Stop block); this covers
// what hooks cannot: a message arriving while Copilot sits idle. agentx answers
// `copilot-ext-poll` with messages only while the instance is listening, and
// `session.send` starts a turn straight from the SDK, so there is no prompt,
// focus or keystroke race to manage.
//
// Acks are not the extension's job: agentx records each batch it hands out and
// commits it when Copilot's UserPromptSubmit hook sees that exact prompt, i.e.
// once Copilot actually ran it. The extension only reports the send outcome
// (`copilot-ext-sent`), always echoing its exact offer receipt. Accepted means
// the prompt may sit queued behind a user turn; it never acknowledges mail.
// A missing SDK result or queue item is uncertain. Managed issued output stays
// owned until actual prompt evidence; unmanaged offers retain their lease policy.
//
// Wake protocol: agentx connects to the registered port and closes (same as the
// pi/omp plugins); the connection itself is the signal. `session.idle` is
// Copilot's own idle signal and is passed on as `--idle`.
//
// Diagnostics: plugin.copilot_* events in agentx.log, next to agentx's copilot.ext_*.

import { joinSession } from "@github/copilot-sdk/extension";
import { spawn } from "node:child_process";
import { appendFileSync, mkdirSync } from "node:fs";
import { createServer } from "node:net";
import { homedir } from "node:os";
import { join } from "node:path";

// Replaced at publish time with agentx's invocation prefix, e.g. ["agentx"].
const AGENTX = __AGENTX_PREFIX__;
const SAFETY_POLL_MS = 30_000;
// A hung agentx call would otherwise hold the serialized delivery loop forever.
const AGENTX_TIMEOUT_MS = 15_000;
const LOG_DIR = join(process.env.AGENTX_DIR || join(homedir(), ".agentx"), ".tmp", "logs");

let instance = null;

function log(level, event, extra = {}) {
	const entry = {
		ts: new Date().toISOString().replace(/\.\d{3}Z$/, "Z"),
		level,
		subsystem: "plugin",
		event: `plugin.copilot_${event}`,
		...(instance ? { instance } : {}),
		...extra,
	};
	try {
		mkdirSync(LOG_DIR, { recursive: true });
		appendFileSync(join(LOG_DIR, "agentx.log"), `${JSON.stringify(entry)}\n`);
	} catch {}
}

function agentx(args) {
	return new Promise((resolve) => {
		const child = spawn(AGENTX[0], [...AGENTX.slice(1), ...args], {
			stdio: ["ignore", "pipe", "pipe"],
			windowsHide: true,
		});
		let stdout = "";
		let stderr = "";
		const timer = setTimeout(() => {
			child.kill("SIGKILL");
			resolve({ error: `timed out after ${AGENTX_TIMEOUT_MS}ms: ${args[0]}` });
		}, AGENTX_TIMEOUT_MS);
		child.stdout.setEncoding("utf8").on("data", (chunk) => (stdout += chunk));
		child.stderr.setEncoding("utf8").on("data", (chunk) => (stderr += chunk));
		child.on("error", (error) => {
			clearTimeout(timer);
			resolve({ error: `spawn ${AGENTX[0]}: ${error}` });
		});
		child.on("close", (code) => {
			clearTimeout(timer);
			try {
				const json = JSON.parse(stdout || "{}");
				resolve(code === 0 ? json : { ...json, error: json.error || `exit ${code}` });
			} catch {
				resolve({ error: `exit ${code}: ${(stderr || stdout).slice(0, 300)}` });
			}
		});
	});
}

const session = await joinSession({});

if (!process.env.AGENTX_PROCESS_ID) {
	log("WARN", "no_process_id", { session_id: session.sessionId });
} else {
	let port = 0;
	let running = false;
	let again = false;
	let againIdle = false;
	// Our prompt still waiting in Copilot's queue (a user turn got in first):
	// { eventId, messageId, receipt }. Reported on every pass while queued.
	let queued = null;
	// messageIds Copilot has started running, newest last.
	const ran = [];
	session.on("user.message", (event) => {
		const id = event.data?.messageId;
		if (!id) return;
		ran.push(id);
		if (ran.length > 20) ran.shift();
		if (queued?.messageId === id) queued = null;
	});

	// Hooks leave a batch alone only while its prompt is vouched for, so keep
	// reporting a queued one. A missing queue item alone cannot prove it never
	// ran: its user.message hook/event may still be in flight.
	const checkQueued = async () => {
		const { eventId, messageId, receipt } = queued;
		const sent = ["copilot-ext-sent", "--event-id", String(eventId), "--session-id", session.sessionId, "--receipt", JSON.stringify(receipt)];
		let pending;
		try {
			pending = await session.rpc.queue.pendingItems();
		} catch (error) {
			log("WARN", "queue_check_failed", { error: String(error) });
			return;
		}
		if (queued?.messageId !== messageId) return;
		if (pending.items.some((item) => item.messageId === messageId)) {
			await agentx(sent);
			return;
		}
		// Dequeued to run: its user.message may still be on the way.
		await new Promise((resolve) => setTimeout(resolve, 1000));
		if (queued?.messageId !== messageId) return;
		queued = null;
		log("WARN", "queued_prompt_dropped", { event_id: eventId });
		await agentx([...sent, "--unknown"]);
	};
	// Serialized: wakes during a pass collapse into one more pass, so a message
	// that lands mid-delivery is never left waiting for the next unrelated wake.
	const deliver = async (idle = false) => {
		if (running) {
			again = true;
			againIdle ||= idle;
			return;
		}
		running = true;
		try {
			do {
				if (queued) await checkQueued();
				const args = ["copilot-ext-poll", "--notify-port", String(port), "--session-id", session.sessionId];
				if (idle || againIdle) args.push("--idle");
				again = againIdle = idle = false;
				const poll = await agentx(args);
				if (poll.error) {
					log("WARN", "poll_failed", { error: poll.error });
					break;
				}
				// Another session's extension (Copilot runs one per session): not ours.
				if (poll.skipped) break;
				if (poll.name && poll.name !== instance) {
					instance = poll.name;
					log("INFO", "bound", { session_id: session.sessionId, port });
				}
				if (!poll.text) continue;
				if (!poll.receipt?.id || poll.receipt.session_id !== session.sessionId) {
					log("WARN", "missing_offer_receipt", { event_id: poll.event_id });
					break;
				}
				const sent = ["copilot-ext-sent", "--event-id", String(poll.event_id), "--session-id", session.sessionId, "--receipt", JSON.stringify(poll.receipt)];
				let messageId;
				try {
					messageId = await session.send({ prompt: poll.text, mode: "enqueue" });
				} catch (error) {
					log("ERROR", "send_failed", { error: String(error) });
					await agentx([...sent, "--unknown"]);
					await session
						.log(`agentx: message delivery failed: ${error}`, { level: "warning", ephemeral: true })
						.catch(() => {});
					break;
				}
				// Copilot owns the prompt now, even if a user turn got in first and
				// it waits in the queue; hooks leave the batch to it.
				if (!ran.includes(messageId)) queued = { eventId: poll.event_id, messageId, receipt: poll.receipt };
				const report = await agentx(sent);
				if (report.error) log("WARN", "sent_report_failed", { error: report.error });
				log("INFO", "enqueued", { count: poll.count });
			} while (again);
		} catch (error) {
			// Callers fire and forget; an escaped rejection would kill the extension.
			log("ERROR", "deliver_failed", { error: String(error) });
		} finally {
			running = false;
		}
	};

	const server = createServer((socket) => {
		socket.destroy();
		void deliver();
	});
	server.on("error", (error) => log("ERROR", "listen_failed", { error: String(error) }));
	server.listen(0, "127.0.0.1", () => {
		port = server.address().port;
		log("INFO", "joined", { session_id: session.sessionId, port });
		void deliver();
	});

	session.on("session.idle", () => void deliver(true));
	setInterval(() => void deliver(), SAFETY_POLL_MS).unref();
}
