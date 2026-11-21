import { parseSpaceStyle, type SpaceStyle } from "../../uiux/systems/space/components/presentation";

const storageKey = "agentx.space.appearance";

export type SpaceAppearance = { style: SpaceStyle; legacyStudio?: boolean };

/** Explicit appearance precedes browser preference; the old art hint is read-only compatibility. */
export function readAppearance(allowSample: boolean): SpaceAppearance {
  const query = new URLSearchParams(location.search);
  const linked = parseSpaceStyle(query.get("style"));
  if (query.get("art") === "studio") {
    // Preserve the old combined Folio/Signal review links until a user selects a style.
    if (linked === "folio" || linked === "signal") return { style: linked, legacyStudio: true };
    if (!linked || linked === "classic") return { style: "studio" };
  }
  if (linked) return { style: linked };
  const fallback = allowSample && query.get("supply") === "studio" ? "studio" : "classic";
  try {
    return { style: parseSpaceStyle(localStorage.getItem(storageKey)) ?? fallback };
  } catch {
    return { style: fallback };
  }
}

/** Replace only the presentation hint of the current entry, leaving its exact route intact. */
export function saveAppearance(style: SpaceStyle): void {
  const url = new URL(location.href);
  url.searchParams.set("style", style);
  url.searchParams.delete("art");
  history.replaceState(history.state, "", url);
  try {
    localStorage.setItem(storageKey, style);
  } catch {
    // The URL still carries the selected style if preference storage is unavailable.
  }
}
