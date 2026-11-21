import { Text, View } from 'react-native';
import { Badge, Button, EmptyState, InfoButton, Row, SectionHeader, StatRow } from '../../../uiux/systems/inspector/inspector.native';
import { t } from '../../shared/i18n';
import { botLoad, loadedCount, readable, teamLoad, unassignedBots, type BotsSnapshot } from '../../shared/domain/bots-observation';
import { count, Distribution, ExecutionRow, SourceLine, TeamMembers, type Context } from './presentation';
/** @aiPurpose Provide sourceSections presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export const sourceSections = (snapshot: BotsSnapshot) => [
  { label: t('botsInspector.bots'), source: snapshot.bots }, { label: t('botsInspector.teams'), source: snapshot.teams },
  { label: t('botsInspector.conversation'), source: snapshot.conversations }, { label: t('botsInspector.runtime'), source: snapshot.runtimes }, { label: t('botsInspector.agent'), source: snapshot.agents },
];
/** @aiPurpose Compose OverviewBoard presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function OverviewBoard(context: Context & { compact: boolean }) {
  const { theme, styles, snapshot, open, compact } = context;
  const attention = snapshot.conversations.items.filter(c => c.status === 'failed' || c.attention), agents = snapshot.agents.items.filter(a => a.state === 'failed' || a.attention);
  const attentionCount = readable(snapshot.conversations) && readable(snapshot.agents) ? attention.length + agents.length : null;
  return <>
    <View style={styles.summary}><StatRow theme={theme} size="lg" columns={compact ? 2 : 4} stats={[
      { label: t('botsInspector.botCount'), value: count(loadedCount(snapshot.bots)) }, { label: t('botsInspector.teamCount'), value: count(loadedCount(snapshot.teams)) },
      { label: t('botsInspector.runningCount'), value: count(loadedCount(snapshot.conversations, c => c.status === 'running')), tone: 'accent' }, { label: t('botsInspector.attentionCount'), value: count(attentionCount), tone: attentionCount ? 'warning' : 'neutral' },
    ]} /><Text style={styles.muted}>{t('botsInspector.loaded')}</Text></View>
    <Distribution {...context} />
    <View style={styles.grid}>{compact ? <><AttentionQueue {...context} /><TeamWorkload {...context} /></> : <><TeamWorkload {...context} /><AttentionQueue {...context} /></>}</View>
  </>;
}

function TeamWorkload(context: Context) {
  const { theme, styles, snapshot, open } = context;
  return <View style={styles.primary}><SectionHeader theme={theme} title={t('botsInspector.teamLoad')} action={<InfoButton theme={theme} title={t('botsInspector.memberLoad')} lines={[t('botsInspector.catalogOnly'), t('botsInspector.teamAttribution')]} />} />
      <View style={styles.panel}>{snapshot.teams.items.map(team => <View key={team.id} style={styles.team}>
        <Row theme={theme} title={team.name} subtitle={t('botsInspector.members', { count: team.memberIds.length })} trailing={team.leadId === null ? <Badge theme={theme} tone="warning" label={t('botsInspector.noLead')} /> : null} onPress={() => open({ kind: 'team', id: team.id })} />
        <TeamMembers {...context} memberIds={team.memberIds} leadId={team.leadId} />
      </View>)}{!snapshot.teams.items.length ? <EmptyState theme={theme} icon="Users" title={readable(snapshot.teams) ? t('botsInspector.noTeams') : t('botsInspector.noData')} /> : null}</View>
    </View>;
}
function AttentionQueue(context: Context) {
  const { theme, styles, snapshot } = context;
  const attention = snapshot.conversations.items.filter(c => c.status === 'failed' || c.attention), agents = snapshot.agents.items.filter(a => a.state === 'failed' || a.attention);
  const attentionCount = readable(snapshot.conversations) && readable(snapshot.agents) ? attention.length + agents.length : null;
  return <View style={styles.secondary}><SectionHeader theme={theme} title={t('botsInspector.attention')} count={attentionCount ?? undefined} />
      <View style={styles.panel}>{attention.slice(0, 6).map(item => <ExecutionRow key={item.id} {...context} item={item} />)}{agents.slice(0, 6).map(item => <ExecutionRow key={JSON.stringify([item.runtimeId, item.id])} {...context} item={item} />)}{!attention.length && !agents.length ? <EmptyState theme={theme} icon="CircleCheck" title={attentionCount === null ? t('botsInspector.noData') : t('botsInspector.noAttention')} /> : null}</View>
      <Text style={styles.muted}>{t('botsInspector.catalogOnly')}</Text>
    </View>;
}

/** @aiPurpose Compose TeamsBoard presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function TeamsBoard(context: Context & { search: string }) {
  const { theme, styles, snapshot, search, open } = context, teams = snapshot.teams.items.filter(t => `${t.name} ${t.id}`.toLowerCase().includes(search.toLowerCase()));
  return <><SourceLine {...context} source={snapshot.teams} /><View style={styles.panel}>{teams.map(team => { const load = teamLoad(snapshot, team.id); return <View key={team.id} style={styles.team}><Row theme={theme} title={team.name} subtitle={t('botsInspector.load', { running: count(load.running), total: count(load.total) })} trailing={<Badge theme={theme} tone={team.leadId ? 'neutral' : 'warning'} label={team.leadId ? snapshot.bots.items.find(b => b.id === team.leadId)?.name ?? team.leadId : t('botsInspector.noLead')} />} onPress={() => open({ kind: 'team', id: team.id })} /><TeamMembers {...context} memberIds={team.memberIds} leadId={team.leadId} /></View>; })}</View>{!teams.length ? <EmptyState theme={theme} icon="Users" title={search ? t('botsInspector.noMatches') : readable(snapshot.teams) ? t('botsInspector.noTeams') : t('botsInspector.noData')} /> : null}</>;
}
/** @aiPurpose Compose BotsBoard presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function BotsBoard(context: Context & { search: string }) {
  const { theme, styles, snapshot, search, open } = context, bots = snapshot.bots.items.filter(b => `${b.name} ${b.title} ${b.id}`.toLowerCase().includes(search.toLowerCase())), unassigned = unassignedBots(snapshot);
  return <><SourceLine {...context} source={snapshot.bots} /><View style={styles.panel}>{bots.map(bot => { const load = botLoad(snapshot, bot.id), team = snapshot.teams.items.find(t => t.memberIds.includes(bot.id)); return <Row key={bot.id} theme={theme} title={bot.name} subtitle={`${bot.title || bot.provider} · ${team?.name ?? (unassigned?.some(b => b.id === bot.id) ? t('botsInspector.unassigned') : t('botsInspector.unknown'))}`} trailing={bot.archived ? <Badge theme={theme} label={t('botsInspector.archived')} /> : null} onPress={() => open({ kind: 'bot', id: bot.id })}>
    <Text style={styles.muted}>{t('botsInspector.load', { running: count(load.running), total: count(load.total) })}{load.failed ? ` · ${t('botsInspector.failedCount', { count: load.failed })}` : ''}</Text>
  </Row>; })}</View>{!bots.length ? <EmptyState theme={theme} icon="Bot" title={search ? t('botsInspector.noMatches') : readable(snapshot.bots) ? t('botsInspector.noBots') : t('botsInspector.noData')} /> : null}</>;
}
export type ExecutionFilter = 'all' | 'running' | 'attention';
/** @aiPurpose Compose ExecutionBoard presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function ExecutionBoard(context: Context & { search: string; filter: ExecutionFilter; setFilter: (filter: ExecutionFilter) => void }) {
  const { theme, styles, snapshot, search, filter, setFilter } = context;
  const items = [...snapshot.conversations.items, ...snapshot.agents.items].filter(item => {
    const state = 'botId' in item ? item.status : item.state;
    return `${item.title} ${item.id}`.toLowerCase().includes(search.toLowerCase()) && (filter === 'all' || filter === 'running' && state === 'running' || filter === 'attention' && (state === 'failed' || item.attention));
  });
  return <><View style={styles.wrap}>{([['all', t('botsInspector.all')], ['running', t('botsInspector.running')], ['attention', t('botsInspector.needsReview')]] as const).map(([value, label]) => <Button key={value} theme={theme} label={label} size="sm" variant={value === filter ? 'secondary' : 'ghost'} onPress={() => setFilter(value)} />)}</View>
    <SourceLine {...context} source={snapshot.conversations} /><SourceLine {...context} source={snapshot.agents} /><View style={styles.panel}>{items.map(item => <ExecutionRow key={'botId' in item ? `c:${item.id}` : JSON.stringify([item.runtimeId, item.id])} {...context} item={item} />)}</View>
    {!items.length ? <EmptyState theme={theme} icon="Activity" title={search || filter !== 'all' ? t('botsInspector.noMatches') : readable(snapshot.conversations) && readable(snapshot.agents) ? t('botsInspector.noExecutions') : t('botsInspector.noData')} /> : null}
  </>;
}
/** @aiPurpose Compose SourcesBoard presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function SourcesBoard({ theme, styles, snapshot }: Context) {
  return <><View style={styles.section}><SectionHeader theme={theme} title={t('botsInspector.authority')} /><Text selectable style={styles.mono}>{snapshot.authorityId ?? t('botsInspector.unknown')}</Text><Text style={styles.muted}>{t('botsInspector.observed')} · {new Date(snapshot.observedAt).toLocaleString()}</Text></View>
    {sourceSections(snapshot).map(({ label, source }) => <View key={label} style={styles.section}><SectionHeader theme={theme} title={label} /><SourceLine theme={theme} styles={styles} source={source} />{source.observedAt ? <Text style={styles.muted}>{new Date(source.observedAt).toLocaleString()}</Text> : null}</View>)}
    <SectionHeader theme={theme} title={t('botsInspector.runtime')} /><View style={styles.panel}>{snapshot.runtimes.items.map(runtime => <Row key={runtime.id} theme={theme} title={runtime.title} subtitle={`${runtime.mode} · ${runtime.health}`}><Text selectable style={styles.mono}>{runtime.id} · {runtime.serverId} · {runtime.generation}</Text></Row>)}</View>
    <SectionHeader theme={theme} title={t('botsInspector.coverage')} /><Text style={styles.muted}>{t('botsInspector.catalogOnly')}</Text><Text style={styles.muted}>{t('botsInspector.noHistory')}</Text><Text style={styles.muted}>{t('botsInspector.missingMetrics')}</Text>
  </>;
}
