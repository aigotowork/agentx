/** @aiPurpose Own platform-neutral foundation values shared by runtime themes and the design studio. */
import type { PluginTheme } from "../../../../host/contracts";
export const originalColors = {
  light: {
    surface0: "#ffffff",
    surface1: "#fafafa",
    surface2: "#f4f4f5",
    border: "#e4e4e7",
    foreground: "#1a1a1e",
    foregroundMuted: "#71717a",
    accent: "#20744A",
    accentForeground: "#ffffff",
    statusSuccess: "#3e704a",
    statusWarning: "#7b5d39",
    statusDanger: "#9d433b",
  },
  dark: {
    surface0: "#181B1A",
    surface1: "#1E2120",
    surface2: "#272A29",
    border: "#252B2A",
    foreground: "#fafafa",
    foregroundMuted: "#A1A5A4",
    accent: "#20744A",
    accentForeground: "#ffffff",
    statusSuccess: "#6cb17b",
    statusWarning: "#c09664",
    statusDanger: "#d8847b",
  },
} satisfies Record<string, PluginTheme["colors"]>;
export const space = {
  xxs: 2,
  xs: 4,
  sm: 8,
  md: 12,
  lg: 16,
  xl: 24,
  xxl: 32,
} as const;
export const fontSize = {
  meta: 12,
  small: 13,
  body: 14,
  content: 15,
  title: 16,
  heading: 18,
} as const;
export const radius = { sm: 6, md: 8, lg: 12, full: 999 } as const;
export const control = { sm: 28, md: 32 } as const;
/** Canonical pixel color roles; original themes continue to come from the host. */
export const pixelColors = {
  light: {
    surface0: "#FFFFFF",
    surface1: "#F3F6FF",
    surface2: "#E8EDFF",
    border: "#D9E0F0",
    foreground: "#19254B",
    foregroundMuted: "#62708D",
    accent: "#315BFF",
    accentForeground: "#FFFFFF",
    statusSuccess: "#008563",
    statusWarning: "#A15A00",
    statusDanger: "#D53050",
  },
  dark: {
    surface0: "#10162B",
    surface1: "#18213B",
    surface2: "#263754",
    border: "#354464",
    foreground: "#F1F5FF",
    foregroundMuted: "#9AAACB",
    accent: "#7096FF",
    accentForeground: "#10162B",
    statusSuccess: "#43E4AD",
    statusWarning: "#FFD64A",
    statusDanger: "#FF738F",
  },
} satisfies Record<string, PluginTheme["colors"]>;

export const pixelHues = {
  blue: ["#315BFF", "#7096FF"],
  sky: ["#006CB8", "#6DC8FF"],
  cyan: ["#007F98", "#4CD9F0"],
  teal: ["#008578", "#43E4CA"],
  green: ["#008563", "#43E4AD"],
  lime: ["#487100", "#B4E85E"],
  amber: ["#FFC938", "#FFD64A"],
  orange: ["#C94B0A", "#FFB349"],
  red: ["#D53050", "#FF738F"],
  pink: ["#DB248E", "#FF7ED0"],
  fuchsia: ["#9B24C4", "#E69CFF"],
  violet: ["#713CE0", "#BB95FF"],
  indigo: ["#4A43CF", "#A8AAFF"],
  gray: ["#62708D", "#9AAACB"],
} as const;

/** Canonical shape tokens. Outlines do not consume layout space or change touch targets. */
export const pixelShape = {
  corner: 0,
  dotCorner: 0,
  outline: 2,
  shadow: 2,
  step: 4,
  panelStep: 6,
  compactStep: 2,
  metricStripe: 3,
  hero: "#315BFF",
} as const;
export const pixelMotion = {
  hover: 80,
  press: 90,
  release: 110,
  enter: 160,
  chart: 180,
  rhythm: 1200,
  scan: 800,
  soundGap: 100,
  icon: 240,
  strike: 200,
  debris: 360,
} as const;
