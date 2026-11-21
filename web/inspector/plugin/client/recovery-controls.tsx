/** @aiPurpose Provide small accessible icon actions, checkboxes and status indicators using AgentX theme roles. */
import { Pressable, Text, View } from "react-native";
import { Icon } from "../../host/ui";
import { Button, Chip, IconButton } from "../../uiux/systems/inspector/inspector.native";
import type { RecoveryStyles } from "./recovery.styles";

export type Tone = "muted" | "warning" | "danger" | "success" | "accent";
/** @aiPurpose Expose an icon with optional short count or label. @aiUsage Provide a full accessible label; hover/focus and long press reveal its meaning. */
/** @aiPurpose Compose IconAction presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function IconAction({ label, icon, value, onPress, disabled, primary, selected, tone = "muted", styles: s }: {
  label: string; icon: string; value?: string | number; onPress(): void; disabled?: boolean; primary?: boolean;
  selected?: boolean; tone?: Tone; styles: RecoveryStyles;
}) {
  const kitTone = tone === "warning" ? "warning" : tone === "danger" ? "danger" : tone === "success" ? "success" : tone === "accent" ? "info" : "neutral";
  return primary
    ? <Button theme={s.theme} icon={icon} label={String(value ?? "")} accessibilityLabel={label} tooltip={label} onPress={onPress} disabled={disabled} size="sm" variant="primary" />
    : value !== undefined
      ? <Chip theme={s.theme} icon={icon} label={String(value)} accessibilityLabel={label} selected={selected} tone={kitTone} disabled={disabled} tooltip={label} onPress={onPress} />
      : <IconButton theme={s.theme} icon={icon} label={label} active={selected} disabled={disabled} onPress={onPress} tone={kitTone} />;

}
/** @aiPurpose Render a compact checkbox with keyboard/touch semantics. @aiUsage Pair its accessible label with the associated row or filter name. */
/** @aiPurpose Compose Check presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function Check({ label, checked, disabled, onPress, styles: s }: {
  label: string; checked: boolean; disabled?: boolean; onPress(): void; styles: RecoveryStyles;
}) {
  return <Pressable accessibilityRole="checkbox" accessibilityLabel={label} accessibilityState={{ checked, disabled: !!disabled }}
    disabled={disabled} onPress={onPress} style={({ pressed }) => [s.button, checked && s.selected, disabled && s.disabled, pressed && s.pressed]}>
    <Icon name={checked ? "SquareCheck" : "Square"} color={checked ? s.accent.color : s.muted.color} size={15} />
  </Pressable>;
}
/** @aiPurpose Combine status color with an icon and short text. @aiUsage Color is supplementary; supply the semantic status through label. */
/** @aiPurpose Compose Status presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function Status({ icon, label, value, tone, styles: s }: {
  icon: string; label: string; value: string | number; tone: Tone; styles: RecoveryStyles;
}) {
  return <View accessible accessibilityLabel={label} style={s.line}>
    <Icon name={icon} color={s[tone].color} size={13} /><Text style={s[tone]}>{value}</Text>
  </View>;
}
/** @aiPurpose Provide toggle presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export const toggle = (values: string[], value: string) => values.includes(value) ? values.filter((v) => v !== value) : [...values, value];
/** @aiPurpose Provide stamp presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export const stamp = (value: string) => new Date(value).toLocaleString();
/** @aiPurpose Provide shortTime presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export const shortTime = (value: string) => new Date(value).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
