/* Focused evidence through a copied tool and the original AgentX HTTP/file entry. */
const assert = require('node:assert/strict');
const { spawnSync } = require('node:child_process');
const { createHash } = require('node:crypto');
const { cpSync, mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } = require('node:fs');
const { tmpdir } = require('node:os');
const { join, resolve } = require('node:path');
const { pathToFileURL } = require('node:url');
const { chromium } = require(process.env.PLAYWRIGHT_PATH || 'playwright');
const root = resolve(__dirname, '../../..');
const checkout = join(root, '.docs/object-atlas-v2');
const base = process.env.CANVAS_URL || 'http://192.168.2.178:18988/';
const checksum = data => createHash('sha256').update(data).digest('hex');
const load = path => JSON.parse(readFileSync(path, 'utf8'));
const openFile = path => pathToFileURL(path).href;
const writeJSON = (path, value) => writeFileSync(path, JSON.stringify(value, null, 2) + '\n');
const shot = (page, name) => page.screenshot({ path: join(__dirname, name + '.png') });
const state = page => page.evaluate(() => {
  const s = ContractWorkbench.state;
  return { layer: s.layerId, scene: s.scene, selected: s.selected };
});

async function run() {
  const tmp = mkdtempSync(join(tmpdir(), 'contract-canvas-'));
  const copy = join(tmp, 'tool');
  const checks = [], errors = [], requestSets = {}, artifacts = {};
  let browser;
  try {
    cpSync(join(root, 'tools/contract-canvas'), copy, { recursive: true });
    const build = (dir, ...args) => spawnSync(process.execPath, ['build.mjs', dir, '../viewer', ...args], { cwd: copy, encoding: 'utf8' });
    const ok = run => assert.equal(run.status, 0, run.stderr);
    ok(build('example'));
    const example = join(copy, 'example');
    const saved = ['index.html', 'data.js'].map(file => readFileSync(join(example, file)));
    for (const file of ['index.html', 'data.js']) rmSync(join(example, file));
    ok(build('example', '--write'));
    for (const [i, file] of ['index.html', 'data.js'].entries()) assert.deepEqual(readFileSync(join(example, file)), saved[i]);
    const exampleModel = load(join(example, 'model.json')), exampleView = load(join(example, 'view.json'));
    const invalid = join(copy, 'invalid'); mkdirSync(invalid);
    const reject = (model, view, diagnostic) => {
      writeJSON(join(invalid, 'model.json'), model); writeJSON(join(invalid, 'view.json'), view);
      const run = build('invalid', '--write');
      assert.equal(run.status, 1, run.stderr);
      assert.match(run.stderr, diagnostic);
    };
    const dangling = structuredClone(exampleModel);
    dangling.edges[1].to = 'AbsentCopy';
    reject(dangling, exampleView, /missing target anchor AbsentCopy/);
    const missingEdge = structuredClone(exampleModel);
    missingEdge.edges = missingEdge.edges.filter(e => e.kind !== 'returns');
    reject(missingEdge, exampleView, /missing output LoanSlip/);
    const mismatched = structuredClone(exampleModel);
    mismatched.nodes.find(node => node.id === 'LoanDesk.previewLoan').owner = 'LoanSlip';
    reject(mismatched, exampleView, /LoanDesk|LoanSlip/);
    reject(exampleModel, { ...exampleView, walkthrough: { ...exampleView.walkthrough, layerId: 'catalog' } }, /every checkpoint needs a subject/);
    assert.throws(() => readFileSync(join(invalid, 'index.html')), { code: 'ENOENT' });
    assert.throws(() => readFileSync(join(invalid, 'data.js')), { code: 'ENOENT' });
    checks.push('Copied tool verified and reproduced both library files byte-for-byte; rejects dangling anchor, missing output edge, invalid ownership and mismatched walkthrough before writing.');

    const minimal = join(copy, 'minimal'); mkdirSync(minimal);
    const tag = '</script><img id="canvas-injected" src="https://invalid.test/pixel" onerror="window.canvasInjected=1">';
    const hostileId = 'Shelf" onfocus="window.canvasInjected=2';
    const hostile = {
      schemaVersion: 1, id: 'minimal', title: tag, authority: 'Isolated input test.',
      layers: [{ id: 'shelf', name: 'Shelf', summary: tag, status: 'source-backed', sourceRefs: [] }],
      nodes: [{ id: hostileId, name: tag, kind: 'value', domain: '__proto__', layerId: 'shelf', provenance: 'source-backed', summary: tag, fields: [], ports: [], sourceRefs: ['loc'] }],
      sources: [{ id: 'loc', path: 'javascript:window.canvasInjected=3', line: 1, symbol: tag, authority: 'declaration' }],
      edges: [], scenarios: [],
    };
    writeJSON(join(minimal, 'model.json'), hostile);
    writeJSON(join(minimal, 'view.json'), { schemaVersion: 1 });
    ok(build('minimal', '--write'));
    assert(!readFileSync(join(minimal, 'data.js'), 'utf8').includes('</script>'));

    browser = await chromium.launch({ executablePath: process.env.CHROME_PATH || '/usr/bin/google-chrome', headless: true, args: ['--no-sandbox'] });
    const attach = async (id, url, options = {}) => {
      const page = await browser.newPage({ viewport: { width: 1440, height: 900 }, ...options });
      page.setDefaultTimeout(5000);
      const requests = requestSets[id] = [];
      page.on('pageerror', e => errors.push(id + ': ' + e.message));
      page.on('request', request => requests.push(request.url()));
      page.on('response', response => { if (response.status() >= 400) errors.push(id + ': ' + response.status() + ' ' + response.url()); });
      await page.goto(url);
      await page.locator('#diagram [data-layer-node]').first().waitFor();
      return page;
    };

    const demo = await attach('library', openFile(join(example, 'index.html')), { offline: true });
    assert.deepEqual(await state(demo), { layer: 'desk', scene: 'layer', selected: 'LoanDesk' });
    await demo.locator('#diagram [data-reference="CatalogCopy"]').first().click();
    await demo.locator('#reference-preview').waitFor({ state: 'visible' });
    assert.match(await demo.locator('#reference-preview').innerText(), /barcode/);
    await demo.locator('#reference-preview [data-ref-action="enter"]').click();
    assert.deepEqual(await state(demo), { layer: 'catalog', scene: 'type', selected: 'CatalogCopy' });
    await demo.locator('[data-layer-action="return"]').click();
    assert.deepEqual(await state(demo), { layer: 'desk', scene: 'layer', selected: 'LoanDesk' });
    await demo.locator('[data-walkthrough-action="open"]').click();
    await demo.locator('[data-walkthrough-step="1"]').first().click();
    await demo.locator('[data-walkthrough-subject="LoanDesk.previewLoan"]').click();
    assert.match(await demo.locator('#inspector-content').innerText(), /RETURNS[\s\S]*LoanSlip/);
    assert.equal(await demo.locator('#contract-walkthrough .provenance-tag').innerText(), '设计提案');
    await shot(demo, 'library-walkthrough');
    await demo.close();

    const bare = await attach('minimal', openFile(join(minimal, 'index.html')), { offline: true });
    assert.equal((await state(bare)).selected, hostileId);
    assert(await bare.locator('[data-scene="method"]').isDisabled());
    for (const sel of ['[data-scene="overview"]', '[data-scene="sequence"]', '[data-walkthrough-action]', '.scenario-lens']) assert(!(await bare.locator(sel).isVisible()), sel);
    assert.equal(await bare.locator('.inspector-title').textContent(), tag);
    await bare.locator('#inspector-content .source-disclosure summary').click();
    assert.equal(await bare.locator('.source-location').textContent(), hostile.sources[0].path + ':1');
    assert.equal(await bare.locator('a[href^="javascript:"]').count(), 0);
    await bare.locator('[data-scene="type"]').click();
    await bare.locator('[data-action="reset"]').click();
    assert.deepEqual(await state(bare), { layer: 'shelf', scene: 'layer', selected: hostileId });
    assert.equal(await bare.locator('#canvas-injected,[onfocus],[onerror]').count(), 0);
    assert.equal(await bare.evaluate(() => window.canvasInjected), undefined);
    await bare.close();
    checks.push('Offline library opens typed references and returns across layers, with a bound method and return in its own walkthrough. Minimal project can drill/reset without any optional views; script/tag/attribute/URL-looking input is inert.');

    const online = await attach('agentxHTTP', base);
    assert.deepEqual(await state(online), { layer: 'application', scene: 'layer', selected: 'Office' });
    assert.match(await online.locator('#layer-context').innerText(), /设计提案/);
    assert.equal(await online.locator('#diagram [data-layer-node="ConversationSource"]').count(), 0);
    assert.equal(await online.locator('#diagram [data-layer-node="Workspace"]').count(), 0);
    assert.equal(await online.title(), 'AgentX · Contract Canvas');
    const popup = online.locator('#reference-preview');
    await online.locator('[data-walkthrough-action="open"]').click();
    assert.equal(await online.locator('.walkthrough-steps button').count(), 5);
    await online.locator('#walkthrough-step-3').click();
    await online.locator('[data-walkthrough-contract="MessageDraft"] [data-reference="MessageDraft"]').click();
    await popup.locator('[data-ref-action="pin"]').click();
    assert(await online.locator('[data-pinned="MessageDraft"]').isVisible());
    assert.equal((await state(online)).scene, 'scenario');
    await shot(online, 'agentx-draft');
    await online.locator('[data-scene="layer"]').click();
    await online.locator('[data-layer-select="Office"]').click();
    await online.locator('[data-action="zoom-in"]').click();
    await online.locator('#search').fill('Office');
    await online.locator('.boundary-toggle').click();
    const edge = online.locator('[data-edge="Office.workspace|wraps|Workspace"]');
    await edge.locator('[data-layer-enter]').click();
    assert.deepEqual(await state(online), { layer: 'sdk', scene: 'type', selected: 'Workspace' });
    await online.locator('#inspector-content > .inspector-section > .source-disclosure summary').first().click();
    assert.match(await online.locator('#inspector-content .source-location:visible').first().innerText(), /packages\/agentx-client/);
    await online.locator('[data-layer-action="return"]').click();
    assert.deepEqual(await state(online), { layer: 'application', scene: 'layer', selected: 'Office' });
    assert.equal(await online.locator('#search').inputValue(), 'Office');
    assert.equal(await online.locator('.fit-button').innerText(), '125%');
    assert(await edge.locator('[data-layer-enter]').evaluate(el => document.activeElement === el));
    assert(await online.locator('[data-pinned="MessageDraft"]').isVisible());

    await online.locator('[data-layer="sdk"]').click();
    await online.locator('[data-scene="overview"]').click();
    assert.equal(await online.locator('#diagram .type-node').count(), 16);
    assert(await online.locator('#diagram [data-method="Workspace.members"]').isVisible());
    await online.locator('[data-lens="members"]').click();
    assert.deepEqual(await state(online), { layer: 'sdk', scene: 'method', selected: 'Workspace.members' });
    assert(await online.locator('#diagram [data-type="PageRequest<MemberCursor>"]').isVisible());
    assert.match(await online.locator('#diagram [data-type="NativeWorkspaceMembershipPaging"]').textContent(), /Native deferred/);
    await online.locator('#diagram [data-type="MemberPage"]').click();
    await popup.locator('[data-ref-action="enter"]').click();
    assert.equal((await state(online)).selected, 'MemberPage');
    await online.locator('#diagram [data-field="MemberPage.coverage"][data-type="MemberCoverage"]').click();
    await popup.locator('.reference-more summary').click();
    assert.match(await popup.innerText(), /MemberCursor/);
    await online.keyboard.press('Escape');
    await online.locator('[data-lens="recovery"]').click();
    assert.equal(await online.locator('#diagram [data-step]').count(), 7);
    assert.equal(await online.locator('#diagram [data-branch]').count(), 5);
    await online.locator('#diagram [data-step="3"]').click();
    assert.equal((await state(online)).selected, 'OutcomeUnknown');
    await online.locator('#diagram [data-branch="OperationStatus.Unknown"]').click();
    assert.equal((await state(online)).selected, 'OperationStatus.Unknown');
    await online.locator('#diagram [data-type="ObservationStopped"]').click();
    assert.match(await popup.innerText(), /ObservationStopped/);
    await online.keyboard.press('Escape');
    await shot(online, 'agentx-recovery');

    for (const file of ['index.html', 'model/graph.js', 'viewer/canvas.css', 'viewer/canvas.js', 'viewer/layout.js', 'viewer/reference-preview.js', 'viewer/layer-workspace.js', 'viewer/scenario-walkthrough.js']) {
      const actual = await online.request.get(file === 'index.html' ? base : new URL(file, base).href);
      assert(actual.ok(), file);
      const bytes = readFileSync(join(checkout, file));
      assert.deepEqual(await actual.body(), bytes, file);
      artifacts[file] = checksum(bytes);
    }
    await online.close();
    checks.push('Existing AgentX HTTP entry served exact published bytes. Desktop retained Office proposals and visibility, draft comparison, source disclosure and context restore, 16-type overview, typed member/paging views and original-operation receipt branches.');

    const narrow = await attach('agentxNarrow', base, { viewport: { width: 390, height: 844 }, hasTouch: true, isMobile: true });
    await narrow.locator('[data-walkthrough-action="open"]').tap();
    await narrow.locator('#walkthrough-step-3').tap();
    await narrow.locator('[data-walkthrough-subject="MessageDraft"]').tap();
    await narrow.locator('.context-bar [data-action="inspector"]').tap();
    assert(await narrow.locator('#inspector.open').isVisible());
    assert.match(await narrow.locator('.inspector-title').innerText(), /MessageDraft/);
    await narrow.keyboard.press('Escape');
    assert.equal(await narrow.locator('#inspector.open').count(), 0);
    assert(await narrow.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
    await shot(narrow, 'agentx-narrow');
    await narrow.close();

    const local = await attach('agentxFile', openFile(join(checkout, 'index.html')), { offline: true });
    await local.locator('.boundary-toggle').click();
    await local.locator('[data-edge="Office.members|delegates|Workspace.members"] [data-layer-enter]').click();
    assert.deepEqual(await state(local), { layer: 'sdk', scene: 'method', selected: 'Workspace.members' });
    await local.locator('#inspector-content [data-type="MemberPage"]:visible').first().click();
    assert.match(await local.locator('#reference-preview').innerText(), /MemberPage/);
    await local.close();
    for (const [id, urls] of Object.entries(requestSets)) {
      assert(urls.every(url => ['agentxHTTP', 'agentxNarrow'].includes(id) ? url.startsWith(base) : url.startsWith('file:')), id + ' loaded unexpected URLs');
    }
    assert.deepEqual(errors, []);
    artifacts['model/contract-model.json'] = checksum(readFileSync(join(checkout, 'model/contract-model.json')));
    artifacts['standalone example/index.html'] = checksum(saved[0]);
    artifacts['standalone example/data.js'] = checksum(saved[1]);
    checks.push('AgentX 390×844 touch walkthrough and inspector drawer fit the viewport. Direct-file entry loads local assets offline, follows exact method delegation and opens the return reference. No page errors or external requests.');
    writeJSON(join(__dirname, 'report.json'), {
      status: 'passed', browser: browser.version(), http: base, artifacts, checks, pageErrors: errors,
      requests: Object.fromEntries(Object.entries(requestSets).map(([id, urls]) => [id, urls.map(url => url.replace(tmp, '<isolated-copy>').replace(root, '<checkout>'))])),
      limits: 'Focused static browser/model evidence; no production AgentX, native effects or package distribution exercised.',
    });
    console.log(checks.join('\n'));
  } finally {
    await browser?.close();
    rmSync(tmp, { recursive: true, force: true });
  }
}
run().catch(error => { console.error(error); process.exitCode = 1; });
