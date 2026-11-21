/** @aiPurpose Present native identity, time and disclosure without coercing integers or inferring missing evidence. */
import type * as N from "./api/native.js";

/**
 * @aiPurpose Name a native enum variant without serializing its evidence payload.
 * @aiUsage Use for finite tags; render consequential payload meaning in the owning evidence region.
 */
export function variant(value: unknown): string {
  if (typeof value === "string") return value;
  if (value && typeof value === "object") return Object.keys(value)[0] ?? "未知";
  return "未知";
}

/** @aiPurpose Preserve integer precision and explicit missing values. @aiUsage Format scalar diagnostic facts. */
export function exact(value: string | bigint | boolean | null | undefined): string {
  return value === null || value === undefined ? "未观测" : String(value);
}

/** @aiPurpose Key retained snapshots by complete era identity. @aiUsage Use only for page-memory maps and operand controls. */
export function configKey(ref: N.ConfigSnapshotRef): string {
  return `${ref.epoch.daemon.scope.canonical_scope}\u0000${ref.epoch.daemon.daemon_instance}\u0000${ref.epoch.database_generation}\u0000${ref.sequence}`;
}

/** @aiPurpose Compare complete daemon identity. @aiUsage Guard cross-read joins and historical references. */
export function sameDaemon(a: N.DaemonRef, b: N.DaemonRef): boolean {
  return a.daemon_instance === b.daemon_instance && a.scope.canonical_scope === b.scope.canonical_scope;
}

/** @aiPurpose Compare daemon-scoped request observations. @aiUsage Preserve exact selection without PID or name heuristics. */
export function sameRequest(a: N.RequestObservationRef, b: N.RequestObservationRef): boolean {
  return a.id === b.id && sameDaemon(a.daemon, b.daemon);
}

/** @aiPurpose Retain clock identity and exact native timestamps. @aiUsage Display source, collection and receive times independently. */
export function time(value: N.ObservationTime | null | undefined): string {
  if (!value) return "未观测";
  return `${value.monotonic_ms} ms · ${value.clock}${value.unix_ms == null ? " · 无墙钟" : ` · Unix ${value.unix_ms} ms`}`;
}

/** @aiPurpose Subtract only compatible monotonic observations. @aiUsage Pass observed native milestones; unknown clocks remain incomparable. */
export function duration(start: N.ObservationTime | null | undefined, end: N.ObservationTime | null | undefined): string {
  if (!start || !end) return "未观测";
  if (start.clock !== end.clock) return "时钟不同 · 不可相减";
  if (end.monotonic_ms < start.monotonic_ms) return "时序不可比较";
  return `${end.monotonic_ms - start.monotonic_ms} ms`;
}

/** @aiPurpose Preserve present, absent, unknown and withheld semantics. @aiUsage Supply a formatter for non-scalar Present payloads. */
export function evidence<T>(value: N.EvidenceValue<T>, show: (item: T) => string = (item) => String(item)): string {
  if (value === "Absent") return "明确缺席";
  if (value === "Withheld") return "已隐去";
  if ("Unknown" in value) return `未知 · ${value.Unknown}`;
  return show(value.Present);
}

/** @aiPurpose Format only native-disclosed configuration values. @aiUsage Use for captured fields and comparison sides. */
export function configValue(value: N.EvidenceValue<N.ConfigValue>): string {
  return evidence(value, (item) => {
    if ("Text" in item) return item.Text;
    if ("Path" in item) return item.Path;
    if ("Seconds" in item) return `${item.Seconds} 秒`;
    return item.Boolean ? "是" : "否";
  });
}

/** @aiPurpose Name native configuration fields. @aiUsage Use field identity rather than table position. */
export function fieldName(field: N.ConfigFieldId): string {
  if ("Setting" in field) return field.Setting;
  if ("Context" in field) return field.Context;
  return field.Participant;
}

/** @aiPurpose Preserve unknown comparison relations including withheld. @aiUsage Render the native relation; never calculate concealed equality. */
export function relation(value: N.ValueRelation): string {
  if (value === "Equal") return "相同";
  if (value === "Different") return "不同";
  return `未知 · ${value.Unknown}`;
}
