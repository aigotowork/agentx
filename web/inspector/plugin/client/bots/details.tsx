import { Text, View } from 'react-native';
import { Button, KeyValue, SectionHeader, Sheet } from '../../../uiux/systems/inspector/inspector.native';
import { t } from '../../shared/i18n';
import { botLoad, readable } from '../../shared/domain/bots-observation';
import { count, ExecutionRow, stateLabel, TeamMembers, type Context, type Subject } from './presentation';
/** @aiPurpose Compose ObjectDetails presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function ObjectDetails(context: Context & { stack: Subject[]; close: () => void; back: () => void; compact: boolean }) {
  const { theme, styles, snapshot, stack, close, back, compact } = context, subject = stack.at(-1);
  if (!subject) return null;
  const bot = subject.kind === 'bot' ? snapshot.bots.items.find(b => b.id === subject.id) : undefined;
  const team = subject.kind === 'team' ? snapshot.teams.items.find(t => t.id === subject.id) : undefined;
  const conversation = subject.kind === 'conversation' ? snapshot.conversations.items.find(c => c.id === subject.id) : undefined;
  const agent = subject.kind === 'agent' ? snapshot.agents.items.find(a => a.id === subject.id && a.runtimeId === subject.runtimeId) : undefined;
  const rows = subject.kind === 'team' ? snapshot.conversations.items.filter(c => c.teamId === subject.id) : subject.kind === 'bot' ? snapshot.conversations.items.filter(c => c.botId === subject.id) : [];
  const value = (label: string, text: string | null) => <KeyValue theme={theme} compact={compact} label={label} value={text ?? t('botsInspector.unknown')} />;
  return <Sheet title={bot?.name ?? team?.name ?? conversation?.title ?? agent?.title ?? subject.id} open onClose={close}><View style={styles.detail}>
    {stack.length > 1 ? <Button theme={theme} label={t('botsInspector.detailBack')} icon="ArrowLeft" variant="ghost" size="sm" onPress={back} /> : null}
    {value(t('botsInspector.identity'), subject.id)}{value(t('botsInspector.authority'), snapshot.authorityId)}
    {!bot && !team && !conversation && !agent ? <Text style={styles.muted}>{t('botsInspector.unseen')}</Text> : null}
    {team ? <><SectionHeader theme={theme} title={t('botsInspector.roles')} /><Text style={styles.muted}>{team.leadId ? t('botsInspector.memberLoad') : t('botsInspector.noLead')}</Text><TeamMembers {...context} memberIds={team.memberIds} leadId={team.leadId} /></> : null}
    {bot ? <>{value(t('botsInspector.provider'), bot.provider)}{value(t('botsInspector.model'), bot.model)}<Text style={styles.text}>{bot.title}</Text><Text style={styles.muted}>{t('botsInspector.load', { running: count(botLoad(snapshot, bot.id).running), total: count(botLoad(snapshot, bot.id).total) })}</Text>
      {snapshot.teams.items.filter(t => t.memberIds.includes(bot.id)).map(team => <Button key={team.id} theme={theme} label={team.name} icon="Users" variant="ghost" onPress={() => context.open({ kind: 'team', id: team.id })} />)}</> : null}
    {conversation ? <>{value(t('botsInspector.state'), stateLabel(conversation.status))}{value(t('botsInspector.updated'), conversation.updatedAt)}<Button theme={theme} label={`${t('botsInspector.botOwner')} · ${snapshot.bots.items.find(b => b.id === conversation.botId)?.name ?? conversation.botId}`} variant="ghost" onPress={() => context.open({ kind: 'bot', id: conversation.botId })} />{conversation.teamId ? <Button theme={theme} label={`${t('botsInspector.teamOwner')} · ${snapshot.teams.items.find(t => t.id === conversation.teamId)?.name ?? conversation.teamId}`} variant="ghost" onPress={() => context.open({ kind: 'team', id: conversation.teamId! })} /> : value(t('botsInspector.teamOwner'), t('botsInspector.none'))}</> : null}
    {agent ? <>{value(t('botsInspector.state'), stateLabel(agent.state))}{value(t('botsInspector.runtime'), agent.runtimeId)}{value(t('botsInspector.generation'), `${agent.serverId} · ${agent.generation}`)}{value(t('botsInspector.provider'), agent.provider)}{value(t('botsInspector.model'), agent.model)}{value(t('botsInspector.updated'), agent.updatedAt)}</> : null}
    {subject.kind === 'bot' || subject.kind === 'team' ? <><SectionHeader theme={theme} title={subject.kind === 'team' ? t('botsInspector.teamAttribution') : t('botsInspector.relations')} count={readable(snapshot.conversations) ? rows.length : undefined} />{rows.map(item => <ExecutionRow key={item.id} {...context} item={item} />)}{!readable(snapshot.conversations) ? <Text style={styles.muted}>{t('botsInspector.noData')}</Text> : null}</> : null}
    <SectionHeader theme={theme} title={t('botsInspector.details')} /><Text style={styles.muted}>{t('botsInspector.catalogOnly')}</Text><Text style={styles.muted}>{t('botsInspector.noHistory')}</Text>
  </View></Sheet>;
}
