import assert from 'node:assert/strict';
import { test } from 'node:test';
import { foregroundPoll } from '../foreground-poll.ts';

test('automatic work pauses when hidden and coalesces repeated foreground events', async ({ mock }) => {
  mock.timers.enable({ apis: ['setTimeout'] });
  const document = Object.assign(new EventTarget(), { hidden: true });
  globalThis.document = document;
  const lifetime = new AbortController();
  let calls = 0;
  let release;
  const visibility = hidden => { document.hidden = hidden; document.dispatchEvent(new Event('visibilitychange')); };
  try {
    foregroundPoll(() => { calls++; return new Promise(resolve => { release = resolve; }); }, 2000, lifetime.signal);
    mock.timers.tick(5000);
    assert.equal(calls, 0);
    visibility(false);
    visibility(false);
    assert.equal(calls, 1);
    release();
    await Promise.resolve();
    visibility(true);
    mock.timers.tick(5000);
    assert.equal(calls, 1);
    visibility(false);
    assert.equal(calls, 2);
    lifetime.abort();
    release();
    visibility(false);
    mock.timers.tick(5000);
    assert.equal(calls, 2);
  } finally {
    lifetime.abort();
    delete globalThis.document;
  }
});
