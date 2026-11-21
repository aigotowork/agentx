import type { ReactNode } from 'react';

const glyphs = {
  agents: 'M5 21v-2a4 4 0 0 1 4-4h6a4 4 0 0 1 4 4v2M16 7a4 4 0 1 1-8 0 4 4 0 0 1 8 0Z',
  workspace: 'M3 7V5a1 1 0 0 1 1-1h5l2 3h9a1 1 0 0 1 1 1v10a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V7Z',
  operations: 'M8 5h12M8 12h12M8 19h12M3 5h.01M3 12h.01M3 19h.01',
  search: 'M17 17l4 4m-2-10a8 8 0 1 1-16 0 8 8 0 0 1 16 0Z',
  plus: 'M12 5v14M5 12h14',
  refresh: 'M20 7a9 9 0 1 0 1 9M20 3v5h-5',
  chevron: 'm9 18 6-6-6-6',
  down: 'm6 9 6 6 6-6',
  back: 'm11 6-6 6 6 6m-6-6h14',
  message: 'M21 11a8 8 0 0 1-8 8H4l2-4a8 8 0 1 1 15-4Z',
  transcript: 'M6 3h9l4 4v14H6V3Zm8 0v5h5M9 12h7m-7 4h7',
  send: 'm21 3-7 18-4-8-8-4 19-6ZM10 13 21 3',
  reply: 'm9 14-5-5 5-5m-5 5h10a5 5 0 0 1 5 5v4',
  bolt: 'm13 2-9 12h7l-1 8 10-12h-7l1-8Z',
  check: 'm5 12 4 4L19 6',
  clock: 'M12 7v5l3 2m6-2a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z',
  warning: 'M12 9v4m0 3h.01M11.1 4.5 3.3 18a1 1 0 0 0 .9 1.5h15.6a1 1 0 0 0 .9-1.5L12.9 4.5a1 1 0 0 0-1.8 0Z',
  sliders: 'M4 7h8m4 0h4M4 12h3m4 0h9M4 17h11m4 0h1M12 5v4M7 10v4m8 1v4',
  close: 'm6 6 12 12M18 6 6 18',
  stop: 'M6 6h12v12H6z',
  resume: 'm8 5 11 7-11 7V5Z',
  info: 'M12 11v5m0-8h.01m9 4a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z',
  route: 'M5 7a2 2 0 1 0 0-.01ZM19 17a2 2 0 1 0 0-.01ZM9 7h6a3 3 0 1 1 0 6H9a2 2 0 0 0 0 4h6',
} as const;

export type Glyph = keyof typeof glyphs;

/** @aiPurpose Render a decorative icon from the console's common stroke set. @aiUsage Label its enclosing control when it has no text. */
export function Icon({ name }: { name: Glyph }) {
  return <svg className="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.65" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d={glyphs[name]} /></svg>;
}

/** @aiPurpose Give empty and loading surfaces a clear title and optional next step. @aiUsage Keep the title specific to the owning resource. */
export function EmptyState({ icon, title, children }: { icon: Glyph; title: string; children?: ReactNode }) {
  return <div className="empty-state"><span className="empty-mark"><Icon name={icon} /></span><strong>{title}</strong>{children && <div className="empty-description">{children}</div>}</div>;
}

const knownStates: Record<string, [string, string]> = {
  active: ['工作中', 'active'], working: ['工作中', 'active'], listening: ['等待消息', 'ready'],
  launching: ['启动中', 'active'], blocked: ['受阻', 'warn'], error: ['异常', 'bad'],
  inactive: ['未活跃', 'neutral'], stopped: ['已停止', 'neutral'],
  Accepted: ['已受理', 'active'], Running: ['执行中', 'active'], Succeeded: ['已完成', 'ready'],
  Failed: ['失败', 'bad'], Unknown: ['结果未知', 'warn'], rejected: ['请求被拒绝', 'bad'], unconfirmed: ['回执待确认', 'warn'],
};

/** @aiPurpose Present native states in operator language while retaining the precise native value. @aiUsage Do not derive status from capability. */
export function State({ value }: { value: string }) {
  const [label, tone] = knownStates[value] ?? knownStates[value.toLowerCase()] ?? [value, 'neutral'];
  return <span className="status-label" data-tone={tone} title={value}><i aria-hidden="true" />{label}</span>;
}

const dateAndTime = new Intl.DateTimeFormat('zh-CN', { month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit', hour12: false });
const timeFormat = new Intl.DateTimeFormat('zh-CN', { hour: '2-digit', minute: '2-digit', hour12: false });

/** @aiPurpose Format a timestamp and retain the original value for inspection. @aiUsage Use timeOnly where a conversation already groups by local day. */
export function Timestamp({ value, timeOnly = false }: { value: string; timeOnly?: boolean }) {
  const date = new Date(value);
  const label = Number.isNaN(date.valueOf()) ? value : (timeOnly ? timeFormat : dateAndTime).format(date);
  return <time title={value} dateTime={value}>{label}</time>;
}
