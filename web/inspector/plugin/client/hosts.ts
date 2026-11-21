import { useHosts } from "../../host/adapter";
import { useMemo } from "react";

/** The app's names for its hosts, by server id. A host this app has not added has no name here. */
export function useHostLabels(): ReadonlyMap<string, string> {
  const hosts = useHosts();
  return useMemo(() => new Map(hosts.map(entry => [entry.serverId, entry.label])), [hosts]);
}
