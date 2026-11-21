/** @aiPurpose Provide a shared reading panel without owning page layout or domain state. */
import * as React from "react";
import type { ReactNode } from "react";
import type { PluginTheme } from "../../../../../host/contracts";
import { Text, View } from "react-native";
import { fontSize, hairline, radius, space } from "../../foundations/native.tokens";
import { skinAttributes, skinStyle } from "../../themes/skin.native";
/** @aiPurpose Provide a reusable reading surface. @aiUsage Import through kit; callers own layout, data and state. */
/** @aiPurpose Compose Panel presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function Panel({
  theme,
  title,
  children,
}: {
  theme: PluginTheme;
  title?: string;
  children: ReactNode;
}) {
  return (
    <View
      {...skinAttributes("card")}
      style={{
        minWidth: 0,
        padding: space.lg,
        gap: space.md,
        borderRadius: radius.lg,
        borderWidth: hairline,
        borderColor: theme.colors.border,
        backgroundColor: theme.colors.surface0,
        ...skinStyle(theme, "card", false, theme.colors.surface0),
      }}
    >
      {title && (
        <Text
          accessibilityRole="header"
          style={{
            fontSize: fontSize.title,
            fontWeight: "600",
            color: theme.colors.foreground,
          }}
        >
          {title}
        </Text>
      )}
      {children}
    </View>
  );
}
