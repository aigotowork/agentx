import type { HistoryRecord } from "../offline/model";

/** @aiPurpose Find text and routing metadata in the supplied records without reading more pages.
 * @aiUsage Supply only records from the selected exact channel.
 */
export function matchingRecords(items: readonly HistoryRecord[], value: string) {
  const query = value.trim().toLocaleLowerCase();
  if (!query) return [];
  return items.filter(item => {
    const fields = item.kind === "message" ? [item.author, item.thread, item.replyTo, item.intent, ...(item.recipients ?? []), item.text] : [item.text];
    return fields.some(field => field?.toLocaleLowerCase().includes(query));
  });
}

/** @aiPurpose Navigate matches within the records loaded in this pane.
 * @aiUsage Supply controlled query and match navigation; this view never fetches history.
 */
export function HistorySearch({ id, query, index, count, onQuery, onMove, onClose }: {
  id: string; query: string; index: number; count: number;
  onQuery(value: string): void; onMove(direction: -1 | 1): void; onClose(): void;
}) {
  const result = !query.trim() ? "" : count ? `${index < 0 ? "–" : index + 1}/${count}` : "无匹配";
  return <div className="history-search-bar" id={id} role="search" aria-label="查找已读入记录">
    <input type="search" autoFocus aria-label="查找已读入记录" placeholder="查找已读入记录…" value={query} onChange={event => onQuery(event.target.value)}
      onKeyDown={event => {
        if (event.nativeEvent.isComposing || event.nativeEvent.keyCode === 229) return;
        if (event.key === "Enter") { event.preventDefault(); onMove(event.shiftKey ? -1 : 1); }
        if (event.key === "Escape") { event.preventDefault(); onClose(); }
      }} />
    <small role="status">{result}</small>
    <button type="button" aria-label="上一个匹配" title="上一个匹配" disabled={!count} onClick={() => onMove(-1)}>↑</button>
    <button type="button" aria-label="下一个匹配" title="下一个匹配" disabled={!count} onClick={() => onMove(1)}>↓</button>
  </div>;
}
