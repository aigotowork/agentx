/** @aiPurpose Resolve one semantic theme for native component props and Web CSS variables from the same values. */
import type { PluginTheme } from "../../../../host/contracts";
import {
  originalColors,
  pixelColors,
  pixelShape,
  pixelMotion,
  space,
  fontSize,
  radius,
  control,
} from "../foundations/foundation.tokens";
import { contrast } from "../foundations/color.math";
import {readTokens,resolveTokens,type TokenValues} from '../../../framework/framework.public';
import tokenSource from './theme.tokens.json';
export const inspectorTokens=readTokens(tokenSource);
export function resolveInspectorTokens(values:TokenValues={},mode='pixel.light'):SystemOverrides {
 const resolved=resolveTokens(inspectorTokens,values,mode);
 return Object.fromEntries(['accent','step','shadow','press','icon','pressDepth','rhythm','chart'].map(id=>[id,resolved[id]])) as SystemOverrides;
}
export type SystemAppearance = {
  style: "original" | "pixel";
  theme: "light" | "dark";
};
export type SystemOverrides = {
  accent?: string;
  step?: number;
  shadow?: number;
  press?: number;
  icon?: number;
  pressDepth?: number;
  rhythm?: number;
  chart?: number;
};
export type SystemTheme = PluginTheme & {
  inspectorStyle: SystemAppearance["style"];
};
export type SystemVariables = Record<`--${string}`, string>;
const bounded = (
  value: number | undefined,
  fallback: number,
  min: number,
  max: number,
) => (Number.isFinite(value) ? Math.max(min, Math.min(max, value!)) : fallback);
/** @aiPurpose Resolve one coherent component theme and Web variable map. @aiUsage Call at the host boundary; overrides affect presentation only. */
export function resolveDesignSystem(
  appearance: SystemAppearance,
  overrides: SystemOverrides = {},
) {
  overrides={...resolveInspectorTokens({},appearance.style+'.'+appearance.theme),...overrides};
  const pixel = appearance.style === "pixel";
  const colors = {
    ...(pixel ? pixelColors : originalColors)[appearance.theme],
  };
  if (overrides.accent && /^#[0-9a-f]{6}$/i.test(overrides.accent) && overrides.accent.toLowerCase()!==colors.accent.toLowerCase()) {
    colors.accent = overrides.accent;
    colors.accentForeground =
      contrast(colors.accent, "#10162B") > contrast(colors.accent, "#FFFFFF")
        ? "#10162B"
        : "#FFFFFF";
  }
  const shape = {
    ...pixelShape,
    step: bounded(overrides.step, pixel ? pixelShape.step : radius.md, 2, 10),
    shadow: bounded(overrides.shadow, pixel ? pixelShape.shadow : 0, 0, 6),
    pressDepth: bounded(overrides.pressDepth, 2, 1, 2),
    panelStep: pixelShape.panelStep,
  };
  const motion = {
    ...pixelMotion,
    press: bounded(overrides.press, pixelMotion.press, 70, 110),
    icon: bounded(overrides.icon, pixelMotion.icon, 120, 400),
    rhythm: bounded(overrides.rhythm, pixelMotion.rhythm, 1000, 1800),
    chart: bounded(overrides.chart, pixelMotion.chart, 100, 260),
  };
  const theme: SystemTheme = { colors, inspectorStyle: appearance.style };
  const variables: SystemVariables = {};
  for (const [name, value] of Object.entries(colors)) {
    variables[`--ds-color-${name}`] = value;
    variables[`--host-${name}`] = `var(--ds-color-${name})`;
  }
  for (const [name, value] of Object.entries(shape))
    if (typeof value === "number") {
      variables[`--ds-shape-${name}`] = `${value}px`;
      variables[`--inspector-shape-${name}`] = `var(--ds-shape-${name})`;
    }
  for (const [name, value] of Object.entries(motion)) {
    variables[`--ds-motion-${name}`] = `${value}ms`;
    variables[`--inspector-motion-${name}`] = `var(--ds-motion-${name})`;
  }
  for (const [name, value] of Object.entries(space))
    variables[`--ds-space-${name}`] = `${value}px`;
  for (const [name, value] of Object.entries(fontSize))
    variables[`--ds-font-${name}`] = `${value}px`;
  for (const [name, value] of Object.entries(control))
    variables[`--ds-control-${name}`] = `${value}px`;
  variables["--ds-radius-panel"] = `${pixel ? 0 : radius.lg}px`;
  variables["--ds-elevation-panel"] = pixel
    ? `${shape.shadow}px ${shape.shadow}px 0 ${appearance.theme === "dark" ? "#070C19" : "#CFD8F4"}`
    : "0 4px 16px rgb(0 0 0 / 6%)";
  variables["--ds-background-grid"] = pixel
    ? `linear-gradient(${colors.border} 1px,transparent 1px),linear-gradient(90deg,${colors.border} 1px,transparent 1px)`
    : "none";
  variables["--ds-texture-size"] = `${space.lg}px`;
  return {
    appearance,
    theme,
    variables,
    shape,
    motion,
    assets: {
      icons: appearance.style,
      sound: pixel ? ("wood" as const) : ("metal" as const),
    },
  };
}
export type ResolvedDesignSystem = ReturnType<typeof resolveDesignSystem>;
