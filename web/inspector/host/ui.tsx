/** Canonical browser primitives; importing design controls never initializes fixture data. */
export { ScrollView, FlatList, TextInput } from "react-native";
export { Icon, Modal, setHostTheme, useToast, ToastHost, SettingsSection, SettingsCard, SettingsRow, SettingsSelect, SettingsSwitch, SettingsAction, Loading } from "../uiux/systems/inspector/platforms/host.web";
export async function copyText(text: string) {
  if (navigator.clipboard?.writeText)
    return navigator.clipboard.writeText(text);
  // Clipboard API is restricted on plain LAN HTTP; retain the explicit user gesture using the browser fallback.
  const input = document.createElement("textarea");
  input.value = text;
  input.className = "host-clipboard";
  document.body.appendChild(input);
  input.select();
  const copied = document.execCommand("copy");
  input.remove();
  if (!copied) throw new Error("浏览器未允许复制");
}
/** @aiPurpose Render compact optional sidebar navigation using the same host theme.
 * @aiUsage The full Web host uses its toolbar; this supports preserved embedded compositions. */
export function SidebarRow({label,onPress,trailing,active}:{label:string;onPress:()=>void;trailing?:React.ReactNode;active?:boolean;icon?:string}) {return <button type="button" onClick={onPress} aria-current={active?'page':undefined}>{label}{trailing}</button>;}
