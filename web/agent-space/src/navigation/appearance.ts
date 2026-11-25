import { parseSpaceStyle, type SpaceStyle } from "../../uiux/systems/space/components/presentation";

const storageKey = "agentx.space.appearance";

export type SpaceAppearance = { style: SpaceStyle };

/** Resolve appearance from the canonical style URL, then the browser preference. */
export function readAppearance(allowSample: boolean): SpaceAppearance {
  const query = new URLSearchParams(location.search);
  const linked = parseSpaceStyle(query.get("style"));
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
  history.replaceState(history.state, "", url);
  try {
    localStorage.setItem(storageKey, style);
  } catch {
    // The URL still carries the selected style if preference storage is unavailable.
  }
}
