import { defineRpc } from "../../host/contracts";
import { z } from "zod";

export const getProviderBases = defineRpc({
  name: "observatory.providers.bases",
  input: z.object({}),
  output: z.object({ bases: z.record(z.string().max(64), z.string().max(64)) }),
});
export type ProviderBases = Readonly<Record<string, string>>;

const ID = /^[\w.-]{1,64}$/;
/** At most this many custom providers are read (a config is far smaller). */
export const MAX_PROVIDER_BASES = 200;

/** Custom provider id → the provider it extends, from a daemon config (`agents.providers.<id>.extends`); ACP ones are left out. */
export function providerBasesFromConfig(config: unknown): Record<string, string> {
  const providers = (config as { agents?: { providers?: unknown } } | null)?.agents?.providers;
  if (!providers || typeof providers !== "object" || Array.isArray(providers)) return {};
  const bases: Record<string, string> = {};
  for (const [id, entry] of Object.entries(providers as Record<string, unknown>).slice(0, MAX_PROVIDER_BASES)) {
    const base = entry && typeof entry === "object" && !Array.isArray(entry) ? (entry as { extends?: unknown }).extends : undefined;
    if (ID.test(id) && typeof base === "string" && ID.test(base) && base !== "acp" && base !== id) bases[id] = base;
  }
  return bases;
}

/** The base a provider id runs on (its usage source): what it extends, else the id itself. */
export function providerBaseOf(provider: string, bases: ProviderBases = {}): string {
  return bases[provider] ?? provider;
}
