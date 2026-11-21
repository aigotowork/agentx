/** @aiPurpose Preserve host color policy and resolve opt-in pixel palette roles through the public kit. */
import type { PluginTheme } from "../../../../host/contracts";
import { Platform, type ViewStyle } from "react-native";
import { isPixelTheme, pixelHues, pixelShape } from "../themes/skin.native";

/** Semantic tones, or any palette hue for categories (e.g. `tone="violet"` for prompts). */
export type Tone = "neutral" | "accent" | "info" | "success" | "warning" | "danger" | Hue;
import { contrast } from "./color.math";
export { contrast } from "./color.math";
export const isDarkTheme = (theme: PluginTheme) => contrast(theme.colors.surface0, "#ffffff") > contrast(theme.colors.surface0, "#000000");
/** `#rrggbb` + alpha → `#rrggbbaa`; other formats are returned unchanged. */
export function withAlpha(color: string, alpha: number): string {
  return /^#[0-9a-f]{6}$/i.test(color) ? `${color}${Math.round(Math.min(1, Math.max(0, alpha)) * 255).toString(16).padStart(2, "0")}` : color;
}

/**
 * Vivid palette. AgentX's status colors are deliberately muted; plugin content uses these so states and
 * categories read at a glance on phones and desktops. Per theme lightness:
 * `solid` marks, fills, bars and icons · `text` colored text (≥ 4.5:1 on the theme surfaces) ·
 * `soft` tinted grounds (badges, selected chips, banners) · `line` tinted borders.
 */
export type Hue = "blue" | "sky" | "cyan" | "teal" | "green" | "lime" | "amber" | "orange" | "red" | "pink" | "fuchsia" | "violet" | "indigo" | "gray";
// Bright on dark (400 shades, neon-like), saturated on light (500/600 marks, 600/700 text); text keeps ≥ 4.5:1 on surface0/2.
const palette: Record<Hue, readonly [lightSolid: string, lightText: string, darkSolid: string, darkText: string]> = {
  blue: ["#3B82F6", "#2563EB", "#60A5FA", "#60A5FA"], sky: ["#0284C7", "#0369A1", "#38BDF8", "#38BDF8"],
  cyan: ["#0891B2", "#0E7490", "#22D3EE", "#22D3EE"], teal: ["#0D9488", "#0F766E", "#2DD4BF", "#2DD4BF"],
  green: ["#059669", "#047857", "#34D399", "#34D399"], lime: ["#65A30D", "#4D7C0F", "#A3E635", "#A3E635"],
  amber: ["#D97706", "#B45309", "#FBBF24", "#FBBF24"], orange: ["#EA580C", "#C2410C", "#FB923C", "#FB923C"],
  red: ["#F43F5E", "#BE123C", "#FB7185", "#FB7185"], pink: ["#EC4899", "#BE185D", "#F472B6", "#F472B6"],
  fuchsia: ["#D946EF", "#A21CAF", "#E879F9", "#E879F9"], violet: ["#8B5CF6", "#7C3AED", "#A78BFA", "#A78BFA"],
  indigo: ["#6366F1", "#4F46E5", "#818CF8", "#818CF8"], gray: ["#71717A", "#52525B", "#A1A1AA", "#D4D4D8"],
};
export function hue(theme: PluginTheme, name: Hue, use: "solid" | "text" | "soft" | "line" = "solid"): string {
  if (isPixelTheme(theme)) {
    const dark = isDarkTheme(theme), solid = pixelHues[name][dark ? 1 : 0];
    if (use === "text" && name === "amber" && !dark) return theme.colors.statusWarning;
    return use === "soft" ? withAlpha(solid, dark ? 0.22 : 0.11) : use === "line" ? withAlpha(solid, dark ? 0.65 : 0.5) : solid;
  }
  const [lightSolid, lightText, darkSolid, darkText] = palette[name], dark = isDarkTheme(theme), solid = dark ? darkSolid : lightSolid;
  return use === "text" ? dark ? darkText : lightText : use === "soft" ? withAlpha(solid, dark ? 0.22 : 0.13) : use === "line" ? withAlpha(solid, dark ? 0.55 : 0.4) : solid;
}
/**
 * Linear gradient background. Native (new architecture) uses `experimental_backgroundImage`, web a CSS `backgroundImage`.
 * `stops` are palette hues or colours; `alpha` fades hue stops. An opaque gradient keeps its first stop as the
 * `backgroundColor` fallback; a translucent wash falls back to transparent (a solid first stop would tint the whole
 * view wherever the image does not draw). `fallback` overrides either choice.
 */
export function gradient(theme: PluginTheme, stops: readonly (Hue | string)[], options: { angle?: number; alpha?: number; fallback?: string } = {}): ViewStyle {
  const colors = stops.map(stop => { const color = stop in palette ? hue(theme, stop as Hue) : stop; return options.alpha === undefined ? color : withAlpha(color, options.alpha); });
  if (isPixelTheme(theme)) return { backgroundColor: options.fallback ?? colors[0] };
  const css = `linear-gradient(${options.angle ?? 135}deg, ${colors.join(", ")})`;
  const translucent = colors.some(color => /^#[0-9a-f]{8}$/i.test(color) && !/ff$/i.test(color) || color === "transparent");
  const fallback = options.fallback ?? (translucent ? "transparent" : colors[0]);
  return { backgroundColor: fallback, ...(Platform.OS === "web" ? { backgroundImage: css } : { experimental_backgroundImage: css }) } as ViewStyle;
}
/** Soft coloured glow around a mark or card (dark themes glow more). */
export function glow(theme: PluginTheme, color: string, radius = 14): ViewStyle {
  if (isPixelTheme(theme)) return { boxShadow: `${pixelShape.shadow}px ${pixelShape.shadow}px 0 ${theme.colors.border}` } as ViewStyle;
  return { boxShadow: `0px 0px ${radius}px ${withAlpha(color, isDarkTheme(theme) ? 0.45 : 0.3)}` } as ViewStyle;
}
/** @aiPurpose Keep text readable on saturated pixel fills. @aiUsage Original filled components retain their existing white foreground. */
export function solidForeground(theme: PluginTheme, background: string): string {
  if (!isPixelTheme(theme)) return "#ffffff";
  return contrast(background, pixelColorsInk) >= contrast(background, "#ffffff") ? pixelColorsInk : "#ffffff";
}
const pixelColorsInk = "#10162B";
// Red and amber stay reserved for failures and warnings, so a project never looks like an alarm.
const nameHues: readonly Hue[] = ["blue", "violet", "pink", "teal", "indigo", "orange", "cyan", "green"];
/** A stable hue per name (project, repository, workspace), so the same project has the same color in every plugin. */
export function hueFor(key: string): Hue {
  let hash = 2166136261;
  for (let index = 0; index < key.length; index++) { hash ^= key.charCodeAt(index); hash = Math.imul(hash, 16777619); }
  // Final avalanche so similar names spread over the hues.
  hash ^= hash >>> 16; hash = Math.imul(hash, 0x85ebca6b); hash ^= hash >>> 13; hash = Math.imul(hash, 0xc2b2ae35); hash ^= hash >>> 16;
  return nameHues[(hash >>> 0) % nameHues.length]!;
}
export const toneHues: Record<"info" | "success" | "warning" | "danger", Hue> = { info: "blue", success: "green", warning: "amber", danger: "red" };
/**
 * Tone → color: info blue, success green, warning amber, danger red from the vivid palette; neutral is muted text.
 * Themes may ship a dark accent (AgentX dark uses #20744A on #181B1A), so accent-toned text and marks fall back
 * to the foreground color whenever they would not be readable.
 */
export function toneColor(theme: PluginTheme, tone: Tone, use: "text" | "mark" | "soft" | "line" = "text"): string {
  const { colors } = theme;
  if (tone === "accent") {
    if (use === "soft" || use === "line") return withAlpha(colors.accent, use === "soft" ? 0.16 : 0.45);
    return contrast(colors.accent, use === "text" ? colors.surface2 : colors.surface0) >= (use === "text" ? 4.5 : 3.2) ? colors.accent : colors.foreground;
  }
  if (tone === "neutral") return use === "soft" ? colors.surface2 : use === "line" ? colors.border : colors.foregroundMuted;
  return hue(theme, tone in toneHues ? toneHues[tone as keyof typeof toneHues] : tone as Hue, use === "mark" ? "solid" : use);
}

