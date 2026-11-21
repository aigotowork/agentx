import { useMemo, useState } from 'react';
import { Text, View } from 'react-native';
import { ScrollView } from '../../../host/ui';
import { useRpc, type PluginScreenProps } from '../../../host/adapter';
import { useQuery } from '@tanstack/react-query';
import { getBotsSnapshot } from '../../shared/bots/rpc';
import { emptyBotsSnapshot } from '../../shared/domain/bots-observation';
import { t } from '../../shared/i18n';
import { Banner, EmptyState, IconButton, SearchField, Tabs, Toolbar } from '../../../uiux/systems/inspector/inspector.native';
import { useForeground } from '../../../uiux/systems/inspector/inspector.native';
import { botsStyles } from './styles';
import { issueLabel, qualityLabel, type Subject } from './presentation';
import { BotsBoard, ExecutionBoard, OverviewBoard, SourcesBoard, TeamsBoard, type ExecutionFilter } from './boards';
import { ObjectDetails } from './details';
type Tab = 'overview' | 'teams' | 'bots' | 'execution' | 'sources';
/** @aiPurpose Compose BotsScreen presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function BotsScreen(props: PluginScreenProps & { onBack?: () => void }) { return <BotsHostScreen key={props.host.id} {...props} />; }
function BotsHostScreen({ theme, layout, host, onBack }: PluginScreenProps & { onBack?: () => void }) {
  const read = useRpc(getBotsSnapshot), foreground = useForeground();
  const query = useQuery({ queryKey: ['observatory', 'bots', host.id, 'snapshot'], queryFn: () => read({}), enabled: foreground, refetchInterval: foreground ? 15_000 : false, retry: false });
  const [tab, setTab] = useState<Tab>('overview'), [search, setSearch] = useState(''), [filter, setFilter] = useState<ExecutionFilter>('all'), [botId, setBotId] = useState<string | null>(null), [selection, setSelection] = useState<{ authorityId: string | null; stack: Subject[] }>({ authorityId: null, stack: [] });
  const styles = useMemo(() => botsStyles(theme, layout.compact), [theme, layout.compact]);
  const snapshot = query.data ?? emptyBotsSnapshot(new Date().toISOString());
  const stack = selection.authorityId === snapshot.authorityId ? selection.stack : [];
  const open = (subject: Subject) => setSelection(previous => ({ authorityId: snapshot.authorityId, stack: [...(previous.authorityId === snapshot.authorityId ? previous.stack : []), subject].slice(-12) }));
  const context = { theme, styles, snapshot, open };
  const unconfigured = query.data?.connection === 'unconfigured';
  const limited = snapshot.connection !== 'available' || [snapshot.bots, snapshot.teams, snapshot.conversations, snapshot.runtimes, snapshot.agents].some(s => s.status !== 'available');
  return <View style={styles.root} testID="bots-inspector">
    <Toolbar theme={theme}>{onBack ? <IconButton theme={theme} label={t('botsInspector.back')} icon="ArrowLeft" onPress={onBack} /> : null}<Text numberOfLines={1} style={[styles.muted, styles.fill]}>{query.isPending ? t('botsInspector.loading') : `${qualityLabel(snapshot.connection)} · ${snapshot.authorityId ?? t('botsInspector.unknown')}`}</Text><IconButton theme={theme} label={t('botsInspector.sourceDetails')} icon="Database" onPress={() => setTab('sources')} /><IconButton theme={theme} label={t('botsInspector.refresh')} icon="RefreshCw" onPress={() => { void query.refetch(); }} disabled={query.isFetching || !foreground} /></Toolbar>
    <ScrollView horizontal style={styles.tabRegion} contentContainerStyle={styles.tabs}><Tabs theme={theme} value={tab} onChange={setTab} options={[
      { value: 'overview', label: t('botsInspector.overview') }, { value: 'teams', label: t('botsInspector.teams') }, { value: 'bots', label: t('botsInspector.bots') }, { value: 'execution', label: t('botsInspector.execution') }, { value: 'sources', label: t('botsInspector.sources') },
    ]} /></ScrollView>
    {query.isError ? <Banner theme={theme} tone="danger" message={t('botsInspector.queryFailed')} /> : null}
    {query.data && limited && !unconfigured ? <Banner theme={theme} tone="warning" message={issueLabel(snapshot.issue) ?? t('botsInspector.sourceLimited')} /> : null}
    {['teams', 'bots', 'execution'].includes(tab) ? <Toolbar theme={theme}><SearchField theme={theme} value={search} onChangeText={setSearch} placeholder={t('botsInspector.search')} accessibilityLabel={t('botsInspector.search')} /></Toolbar> : null}
    <ScrollView style={styles.fill} contentContainerStyle={styles.content}>
      {unconfigured && tab !== 'sources' ? <EmptyState theme={theme} icon="Unplug" title={t('botsInspector.notConnected')} message={t('botsInspector.configure')} action={{ label: t('botsInspector.sourceDetails'), onPress: () => setTab('sources') }} /> : query.isPending && !query.data ? <EmptyState theme={theme} icon="Loader" title={t('botsInspector.loading')} /> : <>
        {tab === 'overview' ? <OverviewBoard {...context} compact={layout.compact} /> : null}
        {tab === 'teams' ? <TeamsBoard {...context} search={search} /> : null}
        {tab === 'bots' ? <BotsBoard {...context} search={search} /> : null}
        {tab === 'execution' ? <ExecutionBoard {...context} search={search} filter={filter} setFilter={setFilter} /> : null}
        {tab === 'sources' ? <SourcesBoard {...context} /> : null}
      </>}
    </ScrollView>
    <ObjectDetails {...context} stack={stack} compact={layout.compact} close={() => setSelection({ authorityId: snapshot.authorityId, stack: [] })} back={() => setSelection(previous => ({ ...previous, stack: previous.stack.slice(0, -1) }))} />
  </View>;
}
