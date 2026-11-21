import { t } from "../shared/text";
/** @aiPurpose Reconcile failed agents, verify recovery effects and serialize safe manual/automatic follow-ups. */
import { randomUUID } from "../../host/identity";
import { CONTINUE_PROMPT, blocksAgentRecovery, classifyError, type Attempt, type Incident, type Target } from "../shared/domain/recovery";
import type { RecoveryState, RecoveryDelivery, RecoveryGuardReason } from "../shared/domain/recovery";
import type { RecoveryStore, AgentPort, DeliveryObservation } from "../shared/domain/recovery-ports";
import type { AgentObservation as Snapshot, WorkspaceObservation as Workspace, TurnStartedFact, TurnEndedFact } from "../shared/domain/observation";
export type { AgentPort } from "../shared/domain/recovery-ports";
export type { AgentObservation as Snapshot, WorkspaceObservation as Workspace } from "../shared/domain/observation";
import type { Policy } from "../shared/domain/recovery";
import { audit, currentCycle, deferCycle, delayMs, enroll, inScope, iso, transition } from "./backoff";

const pendingPhases = new Set<Attempt["phase"]>(["submitting", "accepted", "unknown"]);
const closedWithoutSend = (attempt: Attempt) => attempt.phase === "not_sent" && attempt.delivery?.status === "not_sent" && attempt.delivery.messageId === attempt.id;

function ineligible(agent: Snapshot | null): string | null {
  if (!agent) return t("personalRecovery.m105");
  if (agent.archivedAt || agent.status === "closed") return t("personalRecovery.m106");
  if (agent.internal) return t("personalRecovery.m107");
  if (agent.pendingPermissionCount) return t("personalRecovery.m108");
  if (agent.hasActiveTurn || agent.status === "running" || agent.status === "initializing") return t("personalRecovery.m109");
  if (agent.providerUnavailable) return t("personalRecovery.m110");
  if (agent.status !== "error" && agent.attentionReason !== "error") return t("personalRecovery.m111");
  return null;
}
function latest(state: RecoveryState, agentId: string) {
  return state.incidents.findLast((item) => item.agentId === agentId);
}
function setState(item: Incident, state: Incident["state"]) {
  if (item.state !== state) { item.state = state; item.revision++; }
}
function newIncident(agent: { id: string; title: string | null; provider: string; workspaceId?: string | null }, state: RecoveryState,
  message: string, source: Incident["source"], turnId: string | null, time: number, code?: string): Incident {
  return {
    id: randomUUID(), revision: 1, agentId: agent.id, title: agent.title || agent.id, provider: agent.provider,
    workspaceId: agent.workspaceId ?? null, projectId: null, projectName: t("personalRecovery.m034"), workspaceName: t("personalRecovery.m035"),
    generation: state.generations[agent.id] ?? 0, source, turnId, message: message.slice(0, 16000), code,
    category: classifyError(message, code), observedAt: iso(time), lastUserMessageAt: null, errorFingerprint: null, baselineCaptured: false,
    state: "detected", blockedReason: null,
  };
}

type Submission = { trigger: Attempt["trigger"]; policyRevision?: number };
/** @aiPurpose Own recovery state and timed retry guards. @aiUsage Share one instance across hooks, RPCs and the daemon scheduler. */
export class RecoveryService {
  private scanning: Promise<{ scannedAt: string; scannedAgents: number }> | null = null;
  private lastScan: { scannedAt: string; scannedAgents: number } | null = null;
  private scanEpoch = 0;
  private epoch = 0;
  private locks = new Set<string>();
  private batch: Promise<void> = Promise.resolve();
  private stopped = false;
  private ticking: Promise<void> | null = null;
  constructor(private store: RecoveryStore, private sendTimeoutMs = 20000, private clock = Date.now) {}
  private now() { return iso(this.clock()); }

  async list(port: AgentPort, force = false) {
    if (force || !this.lastScan || this.clock() - Date.parse(this.lastScan.scannedAt) >= 2000 || this.scanEpoch !== this.epoch) {
      this.scanning ??= this.scan(port).finally(() => { this.scanning = null; });
      this.lastScan = await this.scanning;
    }
    const state = await this.store.read();
    // Latest incident per agent, with a bounded result history. Full dedupe journal stays on disk.
    const incidents = [...new Map(state.incidents.map((item) => [item.agentId, item])).values()];
    const ids = new Set(incidents.map((item) => item.agentId));
    return { ...this.lastScan, incidents: incidents.reverse(), attempts: state.attempts.filter((a) => ids.has(a.agentId)), automation: state.automation };
  }

  private async scan(port: AgentPort) {
    const epoch = this.epoch;
    const { agents, workspaces } = await port.scan();
    await this.store.change((state) => {
      // A hook arriving during directory I/O makes that scan stale. Refuse to overwrite its facts.
      if (epoch !== this.epoch) throw new Error(t("personalRecovery.m112"));
      const map = new Map(workspaces.map((w) => [w.id, w]));
      const seen = new Set(agents.map((a) => a.id));
      for (const item of state.incidents) {
        if (!seen.has(item.agentId)) { item.blockedReason = t("personalRecovery.m113"); setState(item, "superseded"); }
      }
      for (const agent of agents) {
        let item = latest(state, agent.id);
        const attempts = state.attempts.filter((a) => a.agentId === agent.id);
        const pending = attempts.findLast((a) => pendingPhases.has(a.phase));
        const newPrompt = item?.baselineCaptured && item.lastUserMessageAt !== agent.lastUserMessageAt;
        if (item?.state === "detected" && (newPrompt || agent.hasActiveTurn || agent.status === "running")) setState(item, "superseded");
        // Snapshot fallback requires a current error state. A remembered lastError alone is insufficient.
        if (agent.status === "error" && !pending && (!item || (item.state !== "detected" && newPrompt) ||
          (item.state === "detected" && item.errorFingerprint !== null && item.errorFingerprint !== (agent.lastError ?? "")))) {
          if (item) setState(item, "superseded");
          item = newIncident(agent, state, agent.lastError || t("personalRecovery.m114"), "snapshot", null, this.clock());
          state.incidents.push(item);
        }
        if (!item) continue;
        const workspace = agent.workspaceId ? map.get(agent.workspaceId) : undefined;
        item.title = agent.title || agent.id;
        item.projectId = workspace?.projectId ?? null;
        item.projectName = workspace?.projectDisplayName ?? t("personalRecovery.m034");
        item.workspaceName = workspace?.title || workspace?.name || t("personalRecovery.m035");
        item.blockedReason = workspace?.archivingAt ? t("personalRecovery.m115") : ineligible(agent);
        if (!item.baselineCaptured) { item.lastUserMessageAt = agent.lastUserMessageAt; item.baselineCaptured = true; }
        if (item.errorFingerprint === null) item.errorFingerprint = agent.lastError ?? "";
        if (item.source === "snapshot" && agent.status !== "error") setState(item, "superseded");
        const previous = attempts.findLast((a) => a.incidentId === item!.id && a.phase !== "skipped");
        if (item.state === "submitted" && previous && ["observed_completed", "canceled"].includes(previous.phase) &&
          agent.status === "idle" && agent.attentionReason !== "error" && !agent.hasActiveTurn) setState(item, "superseded");
        if (pending?.phase === "accepted" && this.clock() - Date.parse(pending.updatedAt) >= 60000) {
          pending.phase = "unknown"; pending.reason = t("personalRecovery.m116");
        }
      }
      this.reconcileCycles(state, agents);
      if (state.automation.monitor.error) audit(state, this.clock(), "scan_restored", t("personalRecovery.m117"));
      state.automation.monitor = { scannedAt: this.now(), scannedAgents: agents.length, error: null };
      state.automation.catalog = {
        projects: [...new Map(workspaces.map((w) => [w.projectId, { id: w.projectId, name: w.projectDisplayName }])).values()],
        workspaces: workspaces.map((w) => ({ id: w.id, name: `${w.projectDisplayName} / ${w.title || w.name}` })),
      };
    });
    this.scanEpoch = epoch;
    return { scannedAt: this.now(), scannedAgents: agents.length };
  }

  private reconcileCycles(state: RecoveryState, agents: Snapshot[]) {
    const snapshots = new Map(agents.map((agent) => [agent.id, agent]));
    const policy = state.automation.policy, time = this.clock();
    for (const item of [...new Map(state.incidents.map((i) => [i.agentId, i])).values()]) {
      let cycle = currentCycle(state, item.agentId);
      const snapshot = snapshots.get(item.agentId);
      if (policy.enabled && inScope(item, policy) && item.state === "detected" &&
        !state.attempts.some((a) => a.incidentId === item.id && a.phase !== "skipped")) {
        cycle = enroll(state, item, time, !!snapshot && item.blockedReason === null);
      }
      if (!cycle || ["recovered", "stopped"].includes(cycle.status)) continue;
      if (cycle.status === "waiting" && item.state === "detected") cycle.incidentId = item.id;
      if (!inScope(item, policy) && cycle.status === "waiting") {
        cycle.pauseSource = "scope"; transition(state, cycle, "paused", time, "paused", t("personalRecovery.m118")); continue;
      }
      if (inScope(item, policy) && cycle.status === "paused" && cycle.pauseSource === "scope") {
        cycle.pauseSource = null; transition(state, cycle, "waiting", time, "resumed", t("personalRecovery.m119"));
      }
      const attempt = state.attempts.find((a) => a.id === cycle.currentAttemptId);
      if (cycle.status === "checking") {
        if (!snapshot || snapshot.archivedAt || snapshot.status === "closed") transition(state, cycle, "stopped", time, "stopped", t("personalRecovery.m120"));
        else if (snapshot.pendingPermissionCount) transition(state, cycle, "blocked", time, "blocked", t("personalRecovery.m121"));
        else if (attempt?.phase === "unknown") transition(state, cycle, "blocked", time, "blocked", attempt.reason);
        else if (attempt?.phase === "observed_completed" && snapshot?.status === "idle" && !snapshot.hasActiveTurn &&
          !snapshot.pendingPermissionCount && snapshot.attentionReason !== "error") {
          transition(state, cycle, "recovered", time, "recovered", t("personalRecovery.m122"));
        } else if (attempt?.phase === "observed_completed" && time - Date.parse(attempt.updatedAt) >= 60000) {
          transition(state, cycle, "blocked", time, "blocked", t("personalRecovery.m123"));
        }
      }
      if (cycle.status !== "waiting") continue;
      if (!snapshot || snapshot.archivedAt || snapshot.status === "closed" || item.state === "superseded") {
        transition(state, cycle, "stopped", time, "stopped", t("personalRecovery.m124"));
      } else if (snapshot.pendingPermissionCount) {
        transition(state, cycle, "blocked", time, "blocked", t("personalRecovery.m121"));
      } else if (snapshot.hasActiveTurn || snapshot.status === "running" || snapshot.status === "initializing") {
        transition(state, cycle, "stopped", time, "stopped", t("personalRecovery.m125"));
      } else if (snapshot.status !== "error" && snapshot.attentionReason !== "error") {
        transition(state, cycle, "stopped", time, "stopped", t("personalRecovery.m126"));
      }
    }
  }

  async configure(policy: Policy) {
    return this.store.change((state) => {
      if (policy.revision !== state.automation.policy.revision) throw new Error(t("personalRecovery.m127"));
      state.automation.policy = { ...policy, revision: policy.revision + 1 };
      audit(state, this.clock(), "policy", t("personalRecovery.m128", { p0: policy.enabled ? t("personalRecovery.m129") : t("personalRecovery.m024"), p1: policy.scanIntervalSeconds, p2: policy.projectIds.length || t("personalRecovery.m080"), p3: policy.workspaceIds.length || t("personalRecovery.m080") }));
      this.epoch++;
      return { policy: state.automation.policy };
    });
  }

  async manage(cycleId: string, action: "pause" | "resume" | "stop") {
    return this.store.change((state) => {
      const cycle = state.automation.cycles.find((c) => c.id === cycleId);
      if (!cycle) throw new Error(t("personalRecovery.m130"));
      if (action === "pause") {
        if (cycle.status !== "waiting") throw new Error(t("personalRecovery.m131"));
        cycle.pauseSource = "user"; transition(state, cycle, "paused", this.clock(), "paused", t("personalRecovery.m132"));
      } else if (action === "resume") {
        if (cycle.status !== "paused") throw new Error(t("personalRecovery.m133"));
        cycle.pauseSource = null; cycle.nextAt = iso(this.clock() + delayMs(cycle.retryIndex));
        transition(state, cycle, "waiting", this.clock(), "resumed", t("personalRecovery.m134"));
      } else transition(state, cycle, "stopped", this.clock(), "stopped", t("personalRecovery.m135"));
      return { cycle };
    });
  }

  async scanInterval() { return (await this.store.read()).automation.policy.scanIntervalSeconds; }
  /** Check recovery intent again under the observatory's shared Agent delivery lock. */
  async deliveryGuard(attemptId: string, agentId: string, agent: DeliveryObservation) {
    return this.store.change((state) => {
      const attempt = state.attempts.find(a => a.id === attemptId);
      const check = (): RecoveryGuardReason | null => {
        if (this.stopped || !attempt || attempt.agentId !== agentId || attempt.phase !== "submitting") return "recovery_inactive";
        const item = state.incidents.find(i => i.id === attempt.incidentId);
        if (!item || item.state !== "submitted" || item.revision !== attempt.revision + 1) return "incident_changed";
        if ((state.generations[attempt.agentId] ?? 0) !== attempt.baselineGeneration) return "generation_changed";
        if (agent.archivedAt || agent.status === "closed") return "agent_gone";
        if (agent.pendingPermissionCount) return "permission_pending";
        if (agent.hasActiveTurn || ["running", "initializing"].includes(agent.status)) return "agent_busy";
        if (agent.providerUnavailable) return "provider_unavailable";
        if (agent.lastUserMessageAt !== item.lastUserMessageAt) return "baseline_changed";
        if (agent.status !== "error" && agent.attentionReason !== "error") return "stop_evidence_missing";
        if (attempt.trigger === "automatic") {
          const cycle = currentCycle(state, attempt.agentId), policy = state.automation.policy;
          if (!policy.enabled) return "automation_disabled";
          if (policy.revision !== attempt.policyRevision) return "policy_changed";
          if (!inScope(item, policy)) return "outside_scope";
          if (cycle?.status !== "checking" || cycle.currentAttemptId !== attempt.id) return "cycle_changed";
        }
        return null;
      };
      const reason = check();
      // Persist the machine reason before the ledger is allowed to return its human-readable refusal.
      if (reason && attempt?.agentId === agentId) attempt.guardReason = reason;
      return reason ? t(`recoveryDelivery.guard_${reason}`) : null;
    });
  }
  async scanFailed() {
    await this.store.change((state) => {
      if (!state.automation.monitor.error) audit(state, this.clock(), "scan_error", t("personalRecovery.m141"));
      state.automation.monitor.error = t("personalRecovery.m142");
    });
  }
  tick(port: AgentPort): Promise<void> {
    if (this.stopped) return Promise.resolve();
    this.ticking ??= this.runTick(port).finally(() => { this.ticking = null; });
    return this.ticking;
  }
  private async runTick(port: AgentPort) {
    await this.list(port, true);
    const state = await this.store.read(), policy = state.automation.policy;
    if (!policy.enabled || this.stopped) return;
    const targets = state.automation.cycles.filter((c) => c.status === "waiting" && c.nextAt && Date.parse(c.nextAt) <= this.clock())
      .map((c) => state.incidents.find((i) => i.id === c.incidentId))
      .filter((i): i is Incident => !!i && i.state === "detected" && inScope(i, policy))
      .slice(0, 500).map((i) => ({ incidentId: i.id, revision: i.revision }));
    if (targets.length) await this.continue(port, `auto-${randomUUID()}`, targets, { trigger: "automatic", policyRevision: policy.revision });
  }

  async turnBefore(event: TurnStartedFact) {
    this.epoch++;
    await this.store.change((state) => {
      const id = event.agent.id;
      state.generations[id] = (state.generations[id] ?? 0) + 1;
      const item = latest(state, id);
      if (item?.state === "detected") setState(item, "superseded");
      const attempt = state.attempts.findLast((a) => a.agentId === id && pendingPhases.has(a.phase));
      if (attempt) {
        attempt.phase = "observed_running"; attempt.observedGeneration = state.generations[id];
        attempt.observedTurnId = event.turnId; attempt.updatedAt = this.now();
        attempt.reason = t("personalRecovery.m143");
      }
      const cycle = currentCycle(state, id);
      if (cycle?.status === "waiting" || cycle?.status === "paused") transition(state, cycle, "stopped", this.clock(), "stopped", t("personalRecovery.m144"));
    });
  }

  async turnAfter(event: TurnEndedFact) {
    this.epoch++;
    await this.store.change((state) => {
      const id = event.agent.id, generation = state.generations[id] ?? 0;
      const attempt = state.attempts.findLast((a) => a.agentId === id && a.phase === "observed_running" &&
        a.observedGeneration === generation && a.observedTurnId === event.turnId);
      if (attempt) {
        attempt.phase = event.outcome.kind === "failed" ? "failed_again" : event.outcome.kind === "canceled" ? "canceled" : "observed_completed";
        attempt.reason = t("personalRecovery.m145"); attempt.updatedAt = this.now();
      }
      if (event.outcome.kind !== "failed") {
        const item = latest(state, id);
        if (item && (item.state === "detected" || item.state === "submitted")) setState(item, "superseded");
        const cycle = currentCycle(state, id);
        if (cycle && (event.outcome.kind === "canceled" || (cycle.status === "waiting" && !attempt))) {
          transition(state, cycle, "stopped", this.clock(), "stopped", t("personalRecovery.m146"));
        }
        return;
      }
      const old = latest(state, id);
      if (old && old.generation === generation && (old.turnId === event.turnId || old.source === "snapshot") && old.message === event.outcome.error.message.slice(0, 16000)) {
        old.source = "hook"; old.turnId = event.turnId; return;
      }
      if (old) setState(old, "superseded");
      const item = newIncident(event.agent, state, event.outcome.error.message, "hook", event.turnId, this.clock(), event.outcome.error.code);
      if (old) { item.projectId = old.projectId; item.projectName = old.projectName; item.workspaceName = old.workspaceName; }
      state.incidents.push(item);
      const cycle = currentCycle(state, id);
      if (cycle?.status === "checking" && attempt?.id === cycle.currentAttemptId) {
        cycle.incidentId = item.id;
        deferCycle(state, cycle, this.clock(), t("personalRecovery.m147"));
      } else if (cycle && attempt?.id === cycle.currentAttemptId && cycle.status === "stopped") {
        cycle.incidentId = item.id;
      }
    });
  }

  // A batch owns at most two in-flight sends, across all connected clients.
  continue(port: AgentPort, operationId: string, targets: Target[], submission: Submission = { trigger: "manual" }) {
    const unique = [...new Map(targets.map((t) => [t.incidentId, t])).values()];
    const task = this.batch.then(async () => {
      const result: Attempt[] = [];
      let index = 0;
      const worker = async () => {
        while (index < unique.length) {
          const target = unique[index++];
          result.push(await this.continueOne(port, operationId, target, submission));
        }
      };
      const workers = await Promise.allSettled([worker(), worker()]);
      const failure = workers.find((r) => r.status === "rejected");
      if (failure?.status === "rejected") throw failure.reason;
      return { attempts: result };
    });
    this.batch = task.then(() => {}, () => {});
    return task;
  }

  private async continueOne(port: AgentPort, operationId: string, target: Target, submission: Submission): Promise<Attempt> {
    const state = await this.store.read();
    const existing = state.attempts.find((a) => a.operationId === operationId && a.incidentId === target.incidentId);
    if (existing) return existing;
    const item = state.incidents.find((i) => i.id === target.incidentId);
    const skipped = (reason: string, behavior: "retry" | "blocked" | "stopped" | "none" = "none"): Promise<Attempt> => this.store.change((draft) => {
      const attempt: Attempt = { id: randomUUID(), operationId, incidentId: target.incidentId, revision: target.revision,
        agentId: item?.agentId ?? "", phase: "skipped", reason, createdAt: this.now(), updatedAt: this.now(), trigger: submission.trigger,
        baselineGeneration: item?.generation ?? 0, observedGeneration: null, observedTurnId: null, delivery: null };
      draft.attempts.push(attempt);
      const cycle = item ? currentCycle(draft, item.agentId) : undefined;
      if (submission.trigger === "automatic" && cycle?.status === "waiting" &&
        draft.automation.policy.enabled && draft.automation.policy.revision === submission.policyRevision) {
        if (behavior === "retry") deferCycle(draft, cycle, this.clock(), reason);
        else if (behavior !== "none") transition(draft, cycle, behavior, this.clock(), behavior, reason);
      }
      return attempt;
    });
    if (this.stopped) return skipped(t("personalRecovery.m148"));
    if (!item || item.revision !== target.revision || item.state !== "detected") return skipped(t("personalRecovery.m149"));
    if (this.locks.has(item.agentId)) return skipped(t("personalRecovery.m150"));
    if (state.attempts.some((a) => (a.incidentId === item.id && a.phase !== "skipped" && !closedWithoutSend(a)) || (a.agentId === item.agentId && blocksAgentRecovery(a)))) return skipped(t("personalRecovery.m151"));
    this.locks.add(item.agentId);
    let reserved: Attempt | null = null;
    try {
      let snapshot: Snapshot | null;
      try { snapshot = await port.refresh(item.agentId); }
      catch { return await skipped(t("personalRecovery.m152"), "retry"); }
      const blocked = ineligible(snapshot) || item.blockedReason;
      if (blocked) return await skipped(blocked, snapshot?.providerUnavailable ? "retry" : snapshot?.pendingPermissionCount ? "blocked" : "stopped");
      if (!snapshot || snapshot.lastUserMessageAt !== item.lastUserMessageAt ||
        (item.source === "snapshot" && (snapshot.status !== "error" || (snapshot.lastError ?? "") !== item.errorFingerprint))) {
        return await skipped(t("personalRecovery.m153"), "stopped");
      }
      let freshWorkspace: Workspace | null | undefined;
      if (snapshot.workspaceId) {
        try { freshWorkspace = await port.refreshWorkspace(snapshot.workspaceId); }
        catch { return await skipped(t("personalRecovery.m154"), "retry"); }
        if (!freshWorkspace || freshWorkspace.archivingAt) return await skipped(t("personalRecovery.m155"), "stopped");
      }
      reserved = await this.store.change((draft) => {
        const current = draft.incidents.find((i) => i.id === item.id);
        if (this.stopped || !current || current.state !== "detected" || current.revision !== target.revision ||
          (draft.generations[item.agentId] ?? 0) !== item.generation) return null;
        if (draft.attempts.some(a => a.agentId === item.agentId && blocksAgentRecovery(a))) return null;
        const cycle = currentCycle(draft, item.agentId), policy = draft.automation.policy;
        const scopedItem = freshWorkspace ? { ...current, projectId: freshWorkspace.projectId, workspaceId: freshWorkspace.id } : current;
        if (submission.trigger === "automatic" && (!policy.enabled || policy.revision !== submission.policyRevision || !inScope(scopedItem, policy) ||
          cycle?.status !== "waiting" || cycle.incidentId !== item.id || !cycle.nextAt || Date.parse(cycle.nextAt) > this.clock())) return null;
        const previous = draft.attempts.findLast(a => a.incidentId === item.id && closedWithoutSend(a));
        const attempt: Attempt = { id: randomUUID(), operationId, incidentId: item.id, revision: target.revision,
          agentId: item.agentId, phase: "submitting", reason: t("personalRecovery.m156"), createdAt: this.now(), updatedAt: this.now(), trigger: submission.trigger,
          ...(submission.policyRevision === undefined ? {} : { policyRevision: submission.policyRevision }),
          ...(previous ? { previousAttemptId: previous.id } : {}),
          baselineGeneration: item.generation, observedGeneration: null, observedTurnId: null, delivery: null };
        draft.attempts.push(attempt); setState(current, "submitted");
        const prior = draft.attempts.find(a => a.id === cycle?.currentAttemptId);
        if (cycle?.status === "waiting" || (submission.trigger === "manual" && cycle?.status === "blocked" && cycle.incidentId === item.id && prior && closedWithoutSend(prior))) {
          cycle.attemptCount++; cycle.currentAttemptId = attempt.id;
          transition(draft, cycle, "checking", this.clock(), "submitted", t("personalRecovery.m157", { p0: cycle.attemptCount, p1: submission.trigger === "automatic" ? t("personalRecovery.m036") : t("personalRecovery.m037") }));
        }
        return attempt;
      });
      if (!reserved) return await skipped(t("personalRecovery.m158"));
      let timer: ReturnType<typeof setTimeout> | undefined;
      let delivery: RecoveryDelivery;
      let timedOut = false;
      try {
        delivery = await Promise.race([
          port.send(item.agentId, CONTINUE_PROMPT, reserved),
          new Promise<never>((_, reject) => { timer = setTimeout(() => { timedOut = true; reject(new Error(t("personalRecovery.m160"))); }, this.sendTimeoutMs); }),
        ]);
        if (delivery.messageId !== reserved.id) delivery = { status: "unknown", messageId: reserved.id, reasonCode: "ledger_conflict", relatedMessageId: delivery.messageId };
      } catch {
        delivery = { status: "unknown", messageId: reserved.id, reasonCode: timedOut ? "send_timeout" : "send_error" };
      } finally { clearTimeout(timer); }
      const id = reserved.id;
      this.epoch++;
      return await this.store.change((draft) => {
        const attempt = draft.attempts.find((a) => a.id === id)!;
        attempt.delivery = delivery;
        const phase = delivery.status === "not_sent" ? "not_sent" : delivery.status === "unknown" ? "unknown" : "accepted";
        const detail = delivery.reasonCode === "guard_rejected" && attempt.guardReason ? t(`recoveryDelivery.guard_${attempt.guardReason}`)
          : t(`recoveryDelivery.${delivery.reasonCode}`);
        const reason = phase === "not_sent" ? t("recoveryDelivery.reviewRequired", { reason: detail }) : detail;
        // Hooks may run and finish before send resolves. Never overwrite observed activity.
        if (attempt.phase === "submitting") {
          attempt.phase = phase; attempt.reason = reason; attempt.updatedAt = this.now();
          const current = draft.incidents.find(i => i.id === attempt.incidentId);
          if (phase === "not_sent" && current?.state === "submitted" && current.revision === attempt.revision + 1 &&
            (draft.generations[attempt.agentId] ?? 0) === attempt.baselineGeneration) setState(current, "detected");
        }
        const cycle = currentCycle(draft, attempt.agentId);
        if (["unknown", "not_sent"].includes(attempt.phase) && cycle?.currentAttemptId === id && cycle.status === "checking") {
          transition(draft, cycle, "blocked", this.clock(), "blocked", reason);
        }
        return attempt;
      });
    } finally { this.locks.delete(item.agentId); }
  }

  async close() { this.stopped = true; await this.ticking?.catch(() => {}); await this.batch; await this.scanning?.catch(() => {}); await this.store.close(); }
}
