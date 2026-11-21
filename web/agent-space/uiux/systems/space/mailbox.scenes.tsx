/** @aiPurpose Safe explicit mailbox handling simulation using the real canonical Block. */
import { useState } from 'react';
import { Button, HumanMailboxBlock } from './space.web';
const messages = [{ id: 'uiux-letter-1', at: '2026-01-02T09:10:00Z', from: 'luna', sender: { name: 'luna', incarnation: 'uiux-1', stopped_event: null }, text: 'UIUX 本地示例：请查看当前结果。' }];
function Mailbox() {
  const [handled, setHandled] = useState(false), [pending, setPending] = useState(true), [open, setOpen] = useState(true);
  const shown = pending && handled ? [] : messages.map(message => ({ ...message, handled_at: handled ? '2026-01-02T09:15:00Z' : null }));
  return <div className="space-preview"><div className="space-preview-tools"><Button onClick={() => setOpen(true)}>打开信箱</Button><Button onClick={() => { setHandled(false); setPending(true); setOpen(true); }}>重置本地演示</Button></div>{open && <HumanMailboxBlock feed={{ messages: shown, hasMore: false, pending, unhandledCount: handled ? 0 : 1, history: setPending, handle: ids => { if (ids.includes('uiux-letter-1')) setHandled(true); } }} onClose={() => setOpen(false)} renderText={text => <p>{text}</p>} />}</div>;
}
/** @aiPurpose Bind the controlled mailbox scene.
 * @aiUsage Handling and history affect local fixture state only; no exact-sender navigation or native effect is simulated as real.
 */
export const scenes = { default: Mailbox };
