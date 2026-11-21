import { useAgentX } from "../../host/adapter";
import { useEffect, useMemo, useSyncExternalStore } from "react";
import { AppState } from "react-native";
import { createAgentDirectory } from "../shared/directory-store";
import { isForeground } from "../../uiux/systems/inspector/inspector.native";

type Directory = ReturnType<typeof createAgentDirectory>;
interface Shared { directory: Directory; users: number; release: ReturnType<typeof setTimeout> | null; appState: { remove(): void } | null; stopped: boolean }
type AgentX = ReturnType<typeof useAgentX>;

/**
 * One directory per host connection and archive scope, shared by the page, the workspace panel and anything else open at
 * the same time (opus-F7): one observation and one paged read instead of one per view. A page switch remounts within
 * moments, so the last user keeps it a little longer instead of re-reading every Agent.
 */
const shared = new WeakMap<AgentX, Map<boolean, Shared>>();
const LINGER_MS = 15_000;

function sharedDirectory(agentx: AgentX, includeArchived: boolean): Shared {
  let scopes = shared.get(agentx);
  if (!scopes) { scopes = new Map(); shared.set(agentx, scopes); }
  let entry = scopes.get(includeArchived);
  if (!entry || entry.stopped) {
    entry = { users: 0, release: null, appState: null, stopped: false, directory: createAgentDirectory({
      list: options => agentx.agents.list(options),
      // AgentX 0.9 routes directory events only to observations this surface's API owns.
      observe: options => agentx.agents.list({ ...options, subscribe: {} }).then(result => result.subscription),
    }, { includeArchived }) };
    scopes.set(includeArchived, entry);
  }
  return entry;
}

function retain(agentx: AgentX, includeArchived: boolean, entry: Shared): () => void {
  if (entry.stopped) { entry.stopped = false; shared.get(agentx)?.set(includeArchived, entry); }
  entry.users += 1;
  if (entry.release) { clearTimeout(entry.release); entry.release = null; }
  if (!entry.appState) {
    entry.directory.start(isForeground(AppState.currentState));
    entry.appState = AppState.addEventListener("change", next => entry.directory.setActive(isForeground(next)));
  }
  return () => {
    entry.users -= 1;
    if (entry.users > 0) return;
    entry.release = setTimeout(() => {
      entry.release = null;
      if (entry.users > 0) return;
      entry.appState?.remove(); entry.appState = null;
      entry.stopped = true; entry.directory.stop();
      const scopes = shared.get(agentx);
      if (scopes?.get(includeArchived) === entry) scopes.delete(includeArchived);
    }, LINGER_MS);
  };
}

/** Keeps the shared directory of a connection and archive scope running; `release` lets it go after a short while. */
export function retainDirectory(agentx: AgentX, includeArchived: boolean): { directory: Directory; release: () => void } {
  const entry = sharedDirectory(agentx, includeArchived);
  return { directory: entry.directory, release: retain(agentx, includeArchived, entry) };
}

/** Host changes get their own directory; disposed host requests cannot leak data. */
export function useObservatory({ includeArchived }: { includeArchived: boolean }) {
  const agentx = useAgentX();
  const entry = useMemo(() => sharedDirectory(agentx, includeArchived), [agentx, includeArchived]);
  useEffect(() => retain(agentx, includeArchived, entry), [agentx, includeArchived, entry]);
  const state = useSyncExternalStore(entry.directory.subscribe, entry.directory.getSnapshot, entry.directory.getSnapshot);
  return useMemo(() => ({ ...state, refresh: entry.directory.refresh }), [state, entry]);
}
