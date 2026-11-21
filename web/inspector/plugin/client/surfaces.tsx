import { BotsScreen } from "./bots/screen";
import { InsightJobsScreen } from "./insight-jobs";

import { DiagnosticsScreen } from "./diagnostics";
import { RecoveryScreen } from "./recovery-screen";
/** @aiPurpose Compose the maintained observatory surfaces and recovery tab into the lazy client bundle. */
import { followText } from "../shared/i18n";
import type { ChunkExtras } from "./lazy-chunk";
import { LanguageScreen } from "./language-screen";
import type { ComponentType } from "react";
import type { PluginAgentPanelProps, PluginClientContext, PluginScreenProps, PluginWorkspacePanelProps } from "../../host/adapter";
import { ArchivedSurface, DashboardSurface, WorkspaceDashboard } from "./dashboard";
import { AgentInspector } from "./agent-inspector";
import { AttentionPopover } from "./attention-popover";
import { AttentionStoresContext } from "./attention-context";
import { CacheClockProvider } from "./cache-clock";
import { setChimePlayer } from "./alert-settings";
import { OverlayBoundary } from "../../uiux/systems/inspector/inspector.native";

// AgentX freezes exports before running the module body; this declaration must be hoisted.
/** @aiPurpose Provide chunk presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export default function chunk() { return { api: 1, createSurfaces }; }

function createSurfaces(client: PluginClientContext, extras?: ChunkExtras): Record<string, ComponentType<any>> {
  followText(extras?.text);
  // The settings' 试听 plays through the host, as the resident chime does.
  setChimePlayer(source => client.playAudio(source));
  const open = (screenId: string, params?: Record<string, string>) => client.openScreen(params ? { screenId, params } : { screenId });
  const Overview = (props: PluginScreenProps) => <DashboardSurface {...props} onOpenArchive={() => open("archives")} onOpenDiagnostics={() => open("diagnostics")} onOpenInsightJobs={() => open("insight-jobs")} />;
  const InsightJobs = (props: PluginScreenProps) => <InsightJobsScreen {...props} onBack={() => open("observatory")} onOpenAnalysis={() => open("observatory", { tab: "insights", t: Date.now().toString(36) })} onOpenReport={jobId => open("observatory", { tab: "insights", job: jobId, t: Date.now().toString(36) })} />;
  const Diagnostics = (props: PluginScreenProps) => <DiagnosticsScreen {...props} onBack={() => open("observatory")} />;
  const Archives = (props: PluginScreenProps) => <ArchivedSurface {...props} onOpenOverview={() => open("observatory")} onOpenDiagnostics={() => open("diagnostics")} onOpenInsightJobs={() => open("insight-jobs")} />;
  const Workspace = (props: PluginWorkspacePanelProps) => <WorkspaceDashboard {...props} onOpenDiagnostics={() => open("diagnostics")} onOpenInsightJobs={() => open("insight-jobs")} />;
  const Inspector = (props: PluginAgentPanelProps) => <AgentInspector {...props} onOpenOverview={agentId => open("observatory", { agent: agentId, t: Date.now().toString(36) })} />;
  // Each surface provides the stub's 需要处理 stores (its `attentionStores` prop) to its subtree: a detail's 知道了 re-reads the badge.
  const provide = (Surface: ComponentType<any>): ComponentType<any> => (props: any) => <AttentionStoresContext.Provider value={props.attentionStores ?? null}><Surface {...props} /></AttentionStoresContext.Provider>;
  // 提示缓存倒计时 for the views that show sessions waiting on you (2.17): read while one of them is open.
  const clocked = (Surface: ComponentType<any>): ComponentType<any> => (props: any) => <CacheClockProvider><Surface {...props} /></CacheClockProvider>;
  const Bots = (props: PluginScreenProps) => <BotsScreen {...props} onBack={() => open("observatory")} />;
  const surfaces = { "bots-inspector": Bots, "insight-jobs": InsightJobs, diagnostics: Diagnostics, recovery: provide(RecoveryScreen), observatory: provide(clocked(Overview)), archives: provide(Archives), "workspace-observatory": provide(clocked(Workspace)), "agent-inspector": provide(clocked(Inspector)),
    "attention-popover": clocked(AttentionPopover), language: LanguageScreen };
  // Create each wrapper once per initialized plugin context, preserving child identities on rerender.
  return Object.fromEntries(Object.entries(surfaces).map(([id, Surface]) => [id,
    (props: any) => <OverlayBoundary theme={props.theme} scopeKey={props.host?.id} mode={id === "attention-popover" ? "content" : "surface"}><Surface {...props} /></OverlayBoundary>]));
}
