/** @aiPurpose Draw a decorative pixel activity mark; the domain still owns the accompanying status text. */
import * as React from "react";
import { View } from "react-native";
import { motionAttributes } from "../../themes/skin.native";

/** @aiPurpose Compose PixelActivity presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function PixelActivity({ color, size = 16, active = true }: { color: string; size?: number; active?: boolean }) {
  return <View accessibilityElementsHidden importantForAccessibility="no-hide-descendants" style={{ width: size, height: size, flexDirection: "row", alignItems: "flex-end", justifyContent: "space-between" }}>
    {[0.5, 1, 0.7].map((height, index) => <View key={index} {...motionAttributes(active ? `work-${index}` : "static")} style={{ width: size / 4, height: size * height, backgroundColor: color, transformOrigin: "bottom" }} />)}
  </View>;
}

/** Four discrete cells communicate indeterminate loading without pretending to be measured progress. */
/** @aiPurpose Compose PixelLoading presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function PixelLoading({ color, size = 16 }: { color: string; size?: number }) {
  return <View accessibilityElementsHidden importantForAccessibility="no-hide-descendants" style={{ width: size, height: size, flexDirection: "row", flexWrap: "wrap", gap: size / 8 }}>
    {[0, 1, 2, 3].map(index => <View key={index} {...motionAttributes(`scan-${index}`)} style={{ width: size * 3 / 8, height: size * 3 / 8, backgroundColor: color }} />)}
  </View>;
}
