/** @aiPurpose Adapt existing plugin surfaces to browser widgets, local skin icons and finite host hooks. */
import { useCallback } from "react";
import { ScrollView, FlatList, TextInput } from "react-native";
import type { z } from "zod";
import { useState, useEffect } from "react";
import { provider, agentx } from "../../../host/provider";
import type { Operation, SettingsDefinition } from "../../../host/contracts";
import { previewLanguage, setPreviewLanguage } from "../../../plugin/mock/i18n.js";
import { fixtureRecapSettings, saveRecapSettings, subscribeRecapSettings } from "../../../plugin/mock/recap-fixture.js";
export { ScrollView, FlatList, TextInput };
export { copyText, SidebarRow } from "../../../host/ui";

/** @aiPurpose Supply the page's explicitly simulated AgentX directory and actions.
 * @aiUsage Use within Inspector only; no connection to real Agents exists. */
export const useAgentX = () => agentx;
/** @aiPurpose Bind a finite validated operation to the selected local provider.
 * @aiUsage Pass the shared domain operation descriptor; unknown operations reject. */
export function useRpc<I extends z.ZodType, O extends z.ZodType>(contract: Operation<I,O>) {
  return useCallback((input:z.input<I>):Promise<z.output<O>> => provider.invoke(contract,input), [contract.name]);
}
/** @aiPurpose Expose absence of an implicit workspace context.
 * @aiUsage Screens supply workspace identity in their own props. */
export function useWorkspace() { return null; }
/** @aiPurpose Expose absence of implicit active Agent context.
 * @aiUsage Screens resolve selected mock Agents through the directory. */
export function useAgent() { return null; }
/** @aiPurpose Supply only the local simulated host.
 * @aiUsage Use for host-aware navigation in Inspector surfaces. */
export function useHosts() { return [{id:'fixture-host',serverId:'fixture-host',label:'AgentX · 本地模拟'}]; }
/** @aiPurpose Maintain local settings without writing any native configuration.
 * @aiUsage Bind a shared settings descriptor; recap changes notify local consumers. */
export function useSettings<S extends z.ZodType>(definition:SettingsDefinition<S>) {
  const initial = () => definition.id === 'language' ? {language:previewLanguage} : definition.id === 'recap' ? {...fixtureRecapSettings} : definition.schema.parse({});
  const [values,setValues] = useState<z.output<S>>(() => definition.schema.parse(initial()));
  useEffect(() => {if(definition.id!=='recap')return;const unsubscribe=subscribeRecapSettings(() => setValues(definition.schema.parse({...fixtureRecapSettings})));return () => {unsubscribe();};},[definition.id]);
  const save = async(next:z.input<S>,_revision?:string) => {const parsed=definition.schema.parse(next);setValues(parsed);if(definition.id==='language')setPreviewLanguage((parsed as {language:string}).language==='auto'?'zh-CN':(parsed as {language:string}).language);if(definition.id==='recap')saveRecapSettings(parsed);return true;};
  return {status:'ready' as 'ready'|'loading'|'error'|'invalid',revision:'mock',values,saving:false,saveError:undefined as string|undefined,error:undefined as string|undefined,save,reset:()=>save(initial() as z.input<S>),async reload(){}};
}
/** @aiPurpose Provide openExternalUrl presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export async function openExternalUrl(url: string) {
  const parsed = new URL(url);
  if (!["https:", "http:"].includes(parsed.protocol))
    throw new Error("链接类型不支持");
  window.open(parsed.href, "_blank", "noopener,noreferrer");
}
export {Icon,Modal,setHostTheme,useToast,ToastHost,SettingsSection,SettingsCard,SettingsRow,SettingsSelect,SettingsSwitch,SettingsAction,Loading} from "../../../uiux/systems/inspector/platforms/host.web";

