import { useEffect, useState } from "react";
import { useForeground } from "../../uiux/systems/inspector/inspector.native";

/** A foreground-only clock; hidden views never re-render on a timer. */
export function useClock(intervalMs: number): number {
  const foreground = useForeground();
  const [now, setNow] = useState(Date.now());
  useEffect(() => { if (!foreground) return; setNow(Date.now()); const timer = setInterval(() => setNow(Date.now()), intervalMs); return () => clearInterval(timer); }, [foreground, intervalMs]);
  return now;
}

