import { Text, View } from 'react-native';
import type { PluginTheme } from '../../../host/contracts';
import { Badge, Button, Row, SectionHeader, StatusDot, toneColor, type Tone } from '../../../uiux/systems/inspector/inspector.native';
import { t } from '../../shared/i18n';
import { botLoad, readable, type BotsSnapshot, type Observed, type SourceStatus, type SourceIssue, type ConversationObservation, type AgentObservation } from '../../shared/domain/bots-observation';
import type { BotsStyles } from './styles';
export type Subject = { kind: 'team' | 'bot' | 'conversation'; id: string } | { kind: 'agent'; id: string; runtimeId: string };
export type Context = { theme: PluginTheme; styles: BotsStyles; snapshot: BotsSnapshot; open: (subject: Subject) => void };
/** @aiPurpose Provide count presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export const count = (n: number | null) => n === null ? t('botsInspector.unknown') : String(n);
/** @aiPurpose Provide qualityLabel presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export const qualityLabel = (status: SourceStatus) => t(`botsInspector.quality_${status}`);
/** @aiPurpose Provide issueLabel presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export const issueLabel = (issue: SourceIssue) => issue ? t(`botsInspector.issue_${issue}`) : null;
/** @aiPurpose Provide stateLabel presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export const stateLabel = (status: string) => t(`botsInspector.state_${status.replaceAll('-', '_')}`);
/** @aiPurpose Provide stateTone presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export const stateTone = (state: string): Tone => state === 'failed' || state === 'rejected' ? 'danger' : state === 'running' ? 'accent' : state === 'succeeded' ? 'success' : state === 'unknown' || state === 'pending' || state === 'accepted' ? 'warning' : 'neutral';
/** @aiPurpose Provide qualityTone presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export const qualityTone = (status: SourceStatus): Tone => status === 'available' ? 'success' : status === 'partial' ? 'warning' : status === 'unconfigured' || status === 'unsupported' ? 'neutral' : 'danger';
/** @aiPurpose Compose Quality presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function Quality({ theme, source }: { theme: PluginTheme; source: Observed<unknown> }) { return <Badge theme={theme} label={qualityLabel(source.status)} tone={qualityTone(source.status)} />; }
/** @aiPurpose Compose SourceLine presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function SourceLine({ theme, styles, source }: Pick<Context, 'theme' | 'styles'> & { source: Observed<unknown> }) {
  return <View style={styles.wrap}><Quality theme={theme} source={source} /><Text style={styles.muted}>{readable(source) ? t('botsInspector.loaded') + ' · ' + source.items.length : issueLabel(source.issue) ?? t('botsInspector.noData')}</Text>{source.gap ? <Text style={styles.muted}>{t('botsInspector.gap')}</Text> : null}{source.truncated ? <Text style={styles.muted}>{t('botsInspector.truncated')}</Text> : null}</View>;
}
/** @aiPurpose Compose ExecutionRow presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function ExecutionRow({ theme, styles, snapshot, open, item }: Context & { item: ConversationObservation | AgentObservation }) {
  const conversation = 'botId' in item, state = conversation ? item.status : item.state;
  const owner = conversation ? snapshot.bots.items.find(b => b.id === item.botId)?.name ?? item.botId : item.runtimeId;
  const kind = conversation ? t('botsInspector.conversation') : item.ownership === 'foreign' ? t('botsInspector.foreign') : t('botsInspector.agent');
  return <Row theme={theme} title={item.title || item.id} subtitle={`${kind} · ${owner}`} leading={<StatusDot theme={theme} tone={stateTone(state)} />} trailing={<Badge theme={theme} label={stateLabel(state)} tone={stateTone(state)} />} onPress={() => open(conversation ? { kind: 'conversation', id: item.id } : { kind: 'agent', id: item.id, runtimeId: item.runtimeId })}>
    {item.attention ? <Text style={styles.muted}>{t('botsInspector.needsReview')}</Text> : null}
  </Row>;
}
/** @aiPurpose Compose TeamMembers presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function TeamMembers({ theme, styles, snapshot, open, memberIds, leadId }: Context & { memberIds: string[]; leadId: string | null }) {
  const max = Math.max(1, ...memberIds.map(id => botLoad(snapshot, id).running ?? 0));
  return <View>{memberIds.map(id => { const bot = snapshot.bots.items.find(b => b.id === id), load = botLoad(snapshot, id); return <View key={id} style={styles.member}>
    <View style={styles.memberIdentity}><Button theme={theme} label={bot?.name ?? id} size="sm" variant="ghost" onPress={() => open({ kind: 'bot', id })} /></View>
    <Badge theme={theme} label={id === leadId ? t('botsInspector.lead') : t('botsInspector.member')} tone={id === leadId ? 'accent' : 'neutral'} />
    <View style={styles.track}>{load.running !== null ? <View style={[styles.segment, { width: `${load.running / max * 100}%`, backgroundColor: toneColor(theme, 'accent', 'mark') }]} /> : null}</View>
    <Text style={styles.count}>{count(load.running)}</Text>
  </View>; })}</View>;
}
/** @aiPurpose Compose Distribution presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function Distribution({ theme, styles, snapshot }: Context) {
  if (!readable(snapshot.conversations)) return null;
  const states = ['running', 'pending', 'idle', 'succeeded', 'failed', 'cancelled', 'unknown'] as const;
  const rows = states.map(state => ({ state, n: snapshot.conversations.items.filter(c => c.status === state).length })).filter(r => r.n);
  return <View style={styles.section}><SectionHeader theme={theme} title={t('botsInspector.distribution')} count={snapshot.conversations.items.length} />
    <View style={styles.distribution}>{rows.map(({ state, n }) => <View key={state} style={[styles.segment, { flex: n, backgroundColor: toneColor(theme, stateTone(state), 'mark') }]} />)}</View>
    <View style={styles.wrap}>{rows.map(({ state, n }) => <View key={state} style={styles.line}><StatusDot theme={theme} tone={stateTone(state)} /><Text style={styles.muted}>{stateLabel(state)} {n}</Text></View>)}</View>
  </View>;
}
