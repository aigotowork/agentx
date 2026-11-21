/** @aiPurpose Own browser-local style/brightness preferences independently of business navigation and state. */
import type { VisualStyle } from "../../themes/skin.native";

export type Appearance = { style: VisualStyle; theme: "light" | "dark" };
export const appearanceKey = "agentx.inspector.appearance.v1";
const validStyle = (value: unknown): value is VisualStyle => value === "original" || value === "pixel";
const validTheme = (value: unknown): value is Appearance["theme"] => value === "light" || value === "dark";

/** @aiPurpose Read explicit URL overrides before validated local preferences. @aiUsage Storage failure keeps the UI usable with original/dark defaults. */
export function readAppearance(search: URLSearchParams, storage: Pick<Storage, "getItem">): Appearance {
  let stored: Partial<Appearance> = {};
  try {
    const value: unknown = JSON.parse(storage.getItem(appearanceKey) ?? "null");
    if (value && typeof value === "object") stored = value;
  } catch { /* Storage can be denied; preference persistence is optional. */ }
  const style = search.get("style"), theme = search.get("theme");
  return { style: validStyle(style) ? style : validStyle(stored.style) ? stored.style : "original", theme: validTheme(theme) ? theme : validTheme(stored.theme) ? stored.theme : "dark" };
}

/** @aiPurpose Persist visual choices without touching host preferences or recovery policy. @aiUsage Call only for local appearance changes; denied storage remains nonfatal. */
export function saveAppearance(value: Appearance, storage: Pick<Storage, "setItem">): void {
  try { storage.setItem(appearanceKey, JSON.stringify(value)); } catch { /* Keep the in-memory choice. */ }
}

/** @aiPurpose Preserve route parameters when sharing or switching visual style. @aiUsage This changes only style/theme query keys. */
export function appearanceUrl(url: URL, appearance: Appearance): URL {
  const next = new URL(url);
  next.searchParams.set("style", appearance.style);
  next.searchParams.set("theme", appearance.theme);
  return next;
}
