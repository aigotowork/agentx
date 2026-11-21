/** @aiPurpose Preserve original host palettes and select opt-in Inspector pixel palettes. */
import type { PluginTheme } from "../../../../host/contracts";
import { resolveDesignSystem } from "./theme.resolve";
import type { Appearance } from "../platforms/web/appearance.preferences";
/** AgentX host palettes. The production kit still owns every metric, layout and component token. */
const systems = {
  original: {
    light: resolveDesignSystem({ style: "original", theme: "light" }),
    dark: resolveDesignSystem({ style: "original", theme: "dark" }),
  },
  pixel: {
    light: resolveDesignSystem({ style: "pixel", theme: "light" }),
    dark: resolveDesignSystem({ style: "pixel", theme: "dark" }),
  },
};
export const light = systems.original.light.theme;
export const dark = systems.original.dark.theme;
/** @aiPurpose Select stable semantic values for components and CSS together. @aiUsage Apply variables at the Web host boundary; never key surfaces by appearance. */
export function inspectorSystem(appearance: Appearance) {
  return systems[appearance.style][appearance.theme];
}
/** @aiPurpose Select one of four stable theme objects. @aiUsage Reuse surfaces and their state; style never becomes a React key. */
export function inspectorTheme(appearance: Appearance): PluginTheme {
  return inspectorSystem(appearance).theme;
}
