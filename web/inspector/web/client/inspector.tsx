/** @aiPurpose Compose the original Inspector surfaces with stable, locally switchable presentation skins. */
import './host/i18n';
import {onTextChange,translator} from '../../plugin/shared/i18n';
import { lazy, Suspense, useCallback, useEffect, useMemo, useState, useSyncExternalStore } from 'react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import type { PluginSurfaceProps } from '../../host/adapter';
import { LiveDashboard } from '../../plugin/client/live/dashboard';
const DemoSurface = lazy(() => import('./demo-surface'));
const demo = new URLSearchParams(location.search).get('source') === 'demo';
import { Button, IconButton, Toolbar, space } from '../../uiux/systems/inspector/inspector.native';
import { Text, View } from 'react-native';
import { setHostTheme, ToastHost, useToast } from '../../host/ui';
import { inspectorSystem } from '../../uiux/systems/inspector/inspector.web';
import { appearanceUrl, readAppearance, saveAppearance, type Appearance } from '../../uiux/systems/inspector/inspector.web';
import { VisualStyleContext } from '../../uiux/systems/inspector/inspector.web';
import { ComponentPreview } from '../../uiux/systems/inspector/inspector.web';
import { FeedbackSettings, usePixelFeedback } from '../../uiux/systems/inspector/inspector.web';
import '../../uiux/systems/inspector/platforms/web/appearance.styles.css';
import '../../uiux/systems/inspector/platforms/web/host.styles.css';

const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
const search = new URLSearchParams(location.search);
const startingSurface = search.get('surface');
const initialSurface = startingSurface === 'bots' ? 'bots-inspector' : ['components', 'bots-inspector', 'agent-inspector', 'archives', 'diagnostics', 'insight-jobs', 'language', 'workspace-observatory'].includes(startingSurface ?? '') ? startingSurface! : 'observatory';
type Route = { screenId: string; params?: Record<string, string> };
// Access localStorage inside guarded methods: some browsers deny even its getter.
const storage = { getItem: (key: string) => localStorage.getItem(key), setItem: (key: string, value: string) => localStorage.setItem(key, value) };

function InspectorHost() {
  useSyncExternalStore(onTextChange,()=>translator().hash);
  const [route, setRoute] = useState<Route>({ screenId: initialSurface, params: Object.fromEntries(search) });
  const [appearance, setAppearance] = useState<Appearance>(() => readAppearance(search, storage));
  const [width, setWidth] = useState(innerWidth);
  const system = inspectorSystem(appearance);
  const theme = system.theme;
  const feedback = usePixelFeedback(appearance.style,system);
  setHostTheme(theme);
  const toast = useToast();
  const open = useCallback((next: Route) => {
    setRoute(next);
    const url = new URL(location.href);
    const retained = Object.fromEntries(['style', 'theme', 'language', 'source'].flatMap(key => url.searchParams.has(key) ? [[key, url.searchParams.get(key)!]] : []));
    url.search = new URLSearchParams({ ...retained, ...next.params, ...(next.screenId === 'observatory' ? {} : { surface: next.screenId }) }).toString();
    history.replaceState(null, '', url);
  }, []);
  useEffect(() => { const resize = () => setWidth(innerWidth); addEventListener('resize', resize); return () => removeEventListener('resize', resize); }, []);
  useEffect(() => {
    const root = document.documentElement;
    root.style.colorScheme = appearance.theme;
    root.dataset.style = appearance.style;
    root.dataset.theme = appearance.theme;
    saveAppearance(appearance, storage);
    history.replaceState(null, '', appearanceUrl(new URL(location.href), appearance));
    for (const [key, value] of Object.entries(system.variables)) root.style.setProperty(key,value);
  }, [system, appearance]);
  const navigation = useMemo(() => ({
    openAgent: ({ agentId, serverId }: { agentId: string; serverId: string }) => {
      if (serverId !== 'fixture-host') { toast.error('该对象不属于当前 AgentX 主机'); return; }
      open({ screenId: 'agent-inspector', params: { agentId } });
    },
    openWorkspace: ({workspaceId}:{workspaceId:string}) => open({screenId:'workspace-observatory',params:{workspaceId}}),
  }), [open]);
  const props = { theme, layout: { compact: width < 700, platform: 'web' }, host: { id: demo ? 'fixture-host' : 'native-api', label: demo ? 'AgentX · 本地模拟' : 'AgentX · API' }, navigation, workspaceId: route.params?.workspaceId, params: route.params ?? {}, agentId: route.params?.agentId } as PluginSurfaceProps;
  const toggleStyle = () => setAppearance(value => ({ ...value, style: value.style === 'pixel' ? 'original' : 'pixel' }));
  const styleLabel = appearance.style === 'pixel' ? '切换为原版风格' : '切换为像素风格';
  return <VisualStyleContext.Provider value={appearance.style}>
    <View style={{ paddingHorizontal: space.md, borderBottomWidth: 1, borderBottomColor: theme.colors.border, backgroundColor: theme.colors.surface0 }}>
      {width < 700 ? <Text accessibilityLabel={demo ? "AgentX · 本地模拟" : "AgentX · API"} style={{ color: theme.colors.foregroundMuted, fontSize: 11, paddingTop: space.xs, flexShrink: 0 }}>{demo ? "AgentX · 本地模拟" : "AgentX · API"}</Text> : null}
      <Toolbar theme={theme} border={false}>
        {width >= 700 ? <Text numberOfLines={1} style={{ color: theme.colors.foreground, fontSize: 14, fontWeight: '600', flexShrink: 1 }}>Agent Inspector</Text> : null}
        {width >= 700 ? <Text numberOfLines={1} style={{ color: theme.colors.foregroundMuted, fontSize: 11, flex: 1 }}>{route.screenId === 'components' ? '组件预览 · 本地样例' : demo ? 'AgentX · 本地模拟' : 'AgentX · API'}</Text> : null}
        <Button theme={theme} size="sm" tone="neutral" label="Inspector" onPress={() => open({ screenId: 'observatory' })} />
        {demo && <Button theme={theme} size="sm" tone="neutral" label="团队" onPress={() => open({ screenId: 'bots-inspector' })} />}
        <IconButton theme={theme} icon="Component" label="组件预览" tooltip={false} onPress={() => open({screenId:'components'})} />
        <IconButton theme={theme} icon="RotateCcw" label={demo ? "重置本地模拟" : "重新连接"} tooltip={false} onPress={() => location.reload()} />
        {width < 700 ? <IconButton theme={theme} icon="Palette" label={styleLabel} active={appearance.style === 'pixel'} tooltip={false} onPress={toggleStyle} />
          : <Button theme={theme} size="sm" icon="Palette" label={appearance.style === 'pixel' ? '像素风' : '原版'} accessibilityLabel={styleLabel} onPress={toggleStyle} />}
        <IconButton theme={theme} icon={appearance.theme === 'dark' ? 'Sun' : 'Moon'} label="切换主题" tooltip={false} onPress={() => setAppearance(value => ({ ...value, theme: value.theme === 'dark' ? 'light' : 'dark' }))} />
        <FeedbackSettings theme={theme} feedback={feedback} />
      </Toolbar>
    </View>
    <div className="inspector-frame">
      {route.screenId === 'components' ? <ComponentPreview theme={theme} feedback={feedback} onBack={() => open({ screenId: 'observatory' })} /> : demo ? <Suspense fallback={<div className="host-connection">正在载入本地模拟…</div>}><DemoSurface route={route} open={open} props={props} /></Suspense> : <LiveDashboard theme={theme} compact={width < 700} route={route} />}
    </div>
    <ToastHost />
  </VisualStyleContext.Provider>;
}
/** @aiPurpose Compose Inspector presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export default function Inspector() { return <QueryClientProvider client={queryClient}><InspectorHost /></QueryClientProvider>; }
