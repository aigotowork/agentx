/** @aiPurpose Compact durable image/file references shared by composer and reader. */
import type { Attachment } from '../../../../../management/api/native';
const raster = new Set(['image/png', 'image/jpeg', 'image/gif', 'image/webp']);
/** @aiPurpose Display service-backed attachments and optional draft removal.
 * @aiUsage Supply registered native uploads; onRemove is available only in the pending composer.
 */
export function Attachments({ items, onRemove }: { items: readonly Attachment[]; onRemove?: (id: string) => void }) {
  return <div className="context-attachments" aria-label="附件">{items.map(item => <div className="context-attachment" key={item.id}><a href={item.url} target="_blank" rel="noreferrer" title={`${item.name} · ${item.media_type}`} aria-label={`打开附件 ${item.name}`}>{raster.has(item.media_type) && <img src={item.url} alt={item.name} loading="lazy" />}<span>{item.name}<small>{item.size.toLocaleString()} B</small></span></a>{onRemove && <button type="button" onClick={() => onRemove(item.id)} aria-label={`移除 ${item.name}`} title={`移除 ${item.name}`}>×</button>}</div>)}</div>;
}
