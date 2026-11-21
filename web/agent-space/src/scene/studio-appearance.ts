/** Canvas tokens, keyed by the same style variable as the office UI. */
import type { SpaceGameWorld, SpaceWorld } from "../../uiux/systems/space/components/presentation";

type Shell = { courtyard: string; shell: string; edge: string; shadow: string };
type Light = { light: string; flame: string; shade: string; spark: string; daylight: number };
export const STUDIO_CANVAS: Shell = { courtyard: "#eae3d2", shell: "#b9986d", edge: "#f0d6aa", shadow: "#473f3024" };
export const STUDIO_WORLDS = {
  lunar: { courtyard: "#101b32", shell: "#395d95", edge: "#81b2db", shadow: "#000918aa" },
  terrarium: { courtyard: "#dae4cb", shell: "#416e55", edge: "#cfe2a4", shadow: "#244c3b38" },
  newsprint: { courtyard: "#eddbc1", shell: "#22202d", edge: "#ec533d", shadow: "#22202d50" },
  inkpost: { courtyard: "#251a24", shell: "#372629", edge: "#b18457", shadow: "#130d1db5", ambience: { light: "#ffbc65", flame: "#ffbc65", shade: "#140e26", spark: "#f9c37b", daylight: .18 } },
  windward: { courtyard: "#a2c4c5", shell: "#345e69", edge: "#ecbe84", shadow: "#1d404e55", ambience: { light: "#fff2bd", flame: "#ffdc96", shade: "#18354d", spark: "#ecfae4", daylight: .10 } },
  jadecourt: { courtyard: "#171f38", shell: "#34425a", edge: "#b7c994", shadow: "#13102ac7", ambience: { light: "#81f6dc", flame: "#fbc182", shade: "#121628", spark: "#f7cf9e", daylight: .17 } },
  citrus: { courtyard: "#c7e3db", shell: "#71978e", edge: "#f9e7ad", shadow: "#496a653f", ambience: { light: "#ffedb5", flame: "#ffd385", shade: "#54776d", spark: "#ffedb0", daylight: .13 } },
  reverie: { courtyard: "#312d54", shell: "#514c79", edge: "#e0b7d9", shadow: "#29234baf", ambience: { light: "#dccccf", flame: "#ffd0dc", shade: "#242345", spark: "#faf0d8", daylight: .16 } },
  wash: { courtyard: "#dddcca", shell: "#79867e", edge: "#d6bc8d", shadow: "#425e6b38", ambience: { light: "#d9e3d1", flame: "#f4c38e", shade: "#566d76", spark: "#ffe1a5", daylight: .10 } },
  collage: { courtyard: "#e9d5b1", shell: "#35323c", edge: "#f2a247", shadow: "#af46256b", ambience: { light: "#ffeda8", flame: "#ffb251", shade: "#433451", spark: "#fff0be", daylight: .10 } },
} as const satisfies { [Key in SpaceWorld]: Shell & (Key extends SpaceGameWorld ? { ambience: Light } : {}) };
