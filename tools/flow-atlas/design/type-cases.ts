/** Compile-only negative examples; they do not replace runtime reference/schema checks. */
import type { EventPort, TraceEvent, ValueCell } from './contracts.js';
import type { Player, LayoutSession } from './api.js';

// @ts-expect-error Emitted events require a payload schema and direction.
const invalidEvent: EventPort = { id: 'created', label: 'created', kind: 'event' };
// @ts-expect-error Redacted/unknown values cannot masquerade as known payloads.
const invalidValue: ValueCell = { kind: 'known', reason: 'no reply' };
// @ts-expect-error Disconnect is a channel event, never an operation reply.
const invalidReply: TraceEvent = { id: 'x', label: 'x', kind: 'reply', after: [], evidence: { kind: 'authored' }, channelId: 'socket', state: 'disconnected' };

function consumer(player: Player, layout: LayoutSession) {
  player.setSpeed(0.5);
  // @ts-expect-error Speed cannot accept an arbitrary duration or invalid rate.
  player.setSpeed(-1);
  layout.move('object-id', { x: 20, y: 40 });
  // @ts-expect-error Moving layout cannot mutate business state through this API.
  layout.move('object-id', { status: 'completed' });
}
void invalidEvent; void invalidValue; void invalidReply; void consumer;
