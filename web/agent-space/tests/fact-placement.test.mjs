/** Regression for a portal covering its own actor after flip/clamp. */
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { placeFacts } from '../uiux/systems/space/components/FactButton.geometry.ts';

test('facts stay inside viewport and outside trigger across every edge, side and constrained-height route', () => {
  const cases = [
    [{ left: 180, right: 224, top: 300, bottom: 354 }, { width: 390, height: 844 }, { width: 280, height: 190 }],
    [{ left: 180, right: 224, top: 12, bottom: 66 }, { width: 390, height: 844 }, { width: 280, height: 190 }],
    [{ left: 180, right: 224, top: 780, bottom: 834 }, { width: 390, height: 844 }, { width: 280, height: 190 }],
    [{ left: 170, right: 214, top: 130, bottom: 184 }, { width: 390, height: 300 }, { width: 280, height: 280 }],
    [{ left: 8, right: 52, top: 130, bottom: 184 }, { width: 390, height: 300 }, { width: 280, height: 280 }],
    [{ left: 338, right: 382, top: 130, bottom: 184 }, { width: 390, height: 300 }, { width: 280, height: 280 }],
  ];
  for (const [anchor, viewport, size] of cases) {
    const result = placeFacts(anchor, size, viewport); assert.ok(result);
    const width = Math.min(size.width, viewport.width - 16), height = Math.min(size.height, result.maxHeight);
    assert.ok(result.x >= 8 && result.y >= 8 && result.x + width <= viewport.width - 8 && result.y + height <= viewport.height - 8);
    const separated = result.x + width <= anchor.left - 8 || result.x >= anchor.right + 8 || result.y + height <= anchor.top - 8 || result.y >= anchor.bottom + 8;
    assert.ok(separated, JSON.stringify({ anchor, result, viewport }));
  }
});
