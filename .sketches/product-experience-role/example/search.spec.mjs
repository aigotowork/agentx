// DESIGN PROJECTION ONLY: no host exists here and these tests have not been run.
import { test, expect } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';

const sourceBytes = readFileSync(new URL('./source.md', import.meta.url));
const flowBytes = readFileSync(new URL('./flow.json', import.meta.url));
const flow = JSON.parse(flowBytes);
const step = id => flow.steps.find(item => item.id === id);
const rows = page => page.getByTestId('task-list').getByTestId('task-row');
const search = page => page.getByRole('textbox', { name: '搜索任务', exact: true });
const empty = page => page.getByText('没有匹配任务', { exact: true });

async function initial(page) {
  await expect(search(page)).toHaveValue(flow.fixture.initialQuery);
  await expect(rows(page)).toHaveCount(flow.fixture.rows.length);
  for (const [index, task] of flow.fixture.rows.entries()) {
    await expect(rows(page).nth(index)).toHaveAttribute('data-task-id', task.id);
    await expect(rows(page).nth(index)).toHaveText(task.title);
  }
}

// Small direct bindings expose meaning; this is not a generic flow interpreter.
async function open(page) {
  await test.step('ST-1 / AS-0 / CR-1,CR-2: initial fixture', async () => {
    await page.goto(flow.page.path);
    await initial(page);
  });
}

async function capture(page, info, stepId, assertionId) {
  await info.attach(`${stepId}-${assertionId}-screenshot`, {
    body: await page.screenshot(), contentType: 'image/png'
  });
}

test.beforeEach(async ({ context }, info) => {
  // Configuration requires explicit ownership confirmation and chosen origin.
  const origin = new URL(info.project.use.baseURL).origin;
  // Restrict browser requests to this disposable host and read-only methods.
  // serviceWorkers are blocked in config; this is not a sandbox for server effects.
  await context.route('**/*', route => {
    const request = route.request();
    const allowed = new URL(request.url()).origin === origin
      && ['GET', 'HEAD'].includes(request.method());
    return allowed ? route.continue() : route.abort('blockedbyclient');
  });
  await info.attach('source.md', { body: sourceBytes, contentType: 'text/markdown' });
  await info.attach('flow.json', { body: flowBytes, contentType: 'application/json' });
  await info.attach('basis.json', {
    body: Buffer.from(JSON.stringify({
      kind: 'executed-basis-if-run', sourceRevision: flow.source.revision,
      sourceSHA256: createHash('sha256').update(sourceBytes).digest('hex'),
      flowRevision: flow.revision,
      flowSHA256: createHash('sha256').update(flowBytes).digest('hex'),
      fixtureRevision: flow.fixture.revision,
      build: process.env.PX_EXAMPLE_BUILD, origin,
      runId: process.env.PX_EXAMPLE_RUN_ID, test: info.title,
      retry: info.retry, timestamp: new Date().toISOString()
    })), contentType: 'application/json'
  });
});

test('PW-1 / SC-1 / BR-1 / CR-1: find intended task', async ({ page }, info) => {
  await open(page);
  await test.step('ST-2 / AS-1 / CR-1: matching title', async () => {
    await search(page).fill(step('ST-2').value);
    await expect(search(page)).toHaveValue(step('ST-2').value);
    await expect(rows(page)).toHaveCount(1);
    await expect(rows(page).first()).toHaveAttribute('data-task-id', flow.fixture.rows[0].id);
    await expect(rows(page).first()).toHaveText(flow.fixture.rows[0].title);
    await capture(page, info, 'ST-2', 'AS-1');
  });
});

test('PW-2 / SC-1 / BR-2 / CR-2: recover from no match', async ({ page }, info) => {
  await open(page);
  await test.step('ST-3 / AS-2 / CR-2: no matching tasks', async () => {
    await search(page).fill(step('ST-3').value);
    await expect(search(page)).toHaveValue(step('ST-3').value);
    await expect(rows(page)).toHaveCount(0);
    await expect(empty(page)).toBeVisible();
    await capture(page, info, 'ST-3', 'AS-2');
  });
  await test.step('ST-4 / AS-3 / CR-2: clear and continue', async () => {
    await page.getByRole('button', { name: '清除筛选', exact: true }).click();
    await initial(page);
    await expect(empty(page)).toBeHidden();
    await capture(page, info, 'ST-4', 'AS-3');
  });
});
