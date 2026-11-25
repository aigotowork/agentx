/** Gesture state and overlay ownership are independent of native effects. */
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { LongPress } from '../src/interactions/long-press.ts';
import { OverlayStack } from '../uiux/systems/space/components/overlay.ts';

test('Escape dismisses newest highest interactive layer plus hints, then underlying layer', () => {
  const stack = new OverlayStack(), closed = [];
  stack.add({ priority: 20, close: () => closed.push('reader') });
  const remove = stack.add({ priority: 100, close: () => closed.push('dialog') });
  stack.add({ priority: 200, passive: true, close: () => closed.push('hint') });
  assert.equal(stack.dismiss(), true); assert.deepEqual(closed, ['hint', 'dialog']);
  remove(); closed.length = 0; stack.dismiss(); assert.deepEqual(closed, ['hint', 'reader']);
  const newer = stack.add({ priority: 20, close: () => closed.push('newer') });
  assert.equal(stack.active.priority, 20); closed.length = 0; stack.dismiss(); assert.deepEqual(closed, ['hint', 'newer']); newer();
});

test('hold fires once, consumes one click, and drift/release/cancel never operate', t => {
  t.mock.timers.enable({ apis: ['setTimeout'] });
  const press = new LongPress(); let fired = 0;
  const start = () => press.begin(1, 0, 0, () => fired++);
  start(); t.mock.timers.tick(499); assert.equal(fired, 0);
  t.mock.timers.tick(1); assert.equal(fired, 1); press.release(1);
  assert.equal(press.consumeClick(), true); assert.equal(press.consumeClick(), false);
  start(); press.move(2, 20, 0); t.mock.timers.tick(500); assert.equal(fired, 2);
  press.consumeClick(); start(); press.move(1, 11, 0); t.mock.timers.tick(500); assert.equal(fired, 2);
  start(); press.release(1); t.mock.timers.tick(500); assert.equal(fired, 2);
  start(); press.cancel(); t.mock.timers.tick(500); assert.equal(fired, 2);
});
