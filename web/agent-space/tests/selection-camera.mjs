/** Bounded user-reported selection camera reproduction on the running preview; does not own its server. */
import assert from 'node:assert/strict';
import { chromium } from '/home/long/.npm/_npx/e41f203b7505f1fb/node_modules/playwright/index.mjs';
import { mkdir, writeFile } from 'node:fs/promises';
const out = process.env.AGENT_SPACE_EVIDENCE ?? '/tmp/agent-space-evidence-mofe-20261115/camera';
await mkdir(out, { recursive: true });
const browser = await chromium.launch({ executablePath: '/usr/bin/google-chrome', headless: true, args: ['--no-sandbox'] });
const records = [];
try {
  for (const width of (process.env.AGENT_SPACE_WIDTHS?.split(',').map(Number) ?? [1440, 390, 320])) {
    const page = await browser.newPage({ viewport: { width, height: width === 1440 ? 1000 : 844 } });
    await page.addInitScript(() => { const native = CanvasRenderingContext2D.prototype.scale; CanvasRenderingContext2D.prototype.scale = function(x,y) { window.__lastCameraScale = {x,y}; return native.call(this,x,y); }; });
    await page.goto(process.env.AGENT_SPACE_URL ?? 'http://127.0.0.1:18967');
    await page.getByLabel('工作区范围').selectOption('/projects/atlas/web');
    await page.locator('.office-actor').first().waitFor();
    await page.getByRole('button', { name: '暂停动作', exact: true }).click();
    async function capture(step) {
      await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
      const data = await page.evaluate(() => ({ scale: window.__lastCameraScale, landmarks: [...document.querySelectorAll('.hall-room-name,.office-prop')].map(e => {const r=e.getBoundingClientRect(); return {x:r.x,y:r.y};}), viewport: {width:innerWidth,height:innerHeight}, stage: {x:document.querySelector(".space-scene-stage").getBoundingClientRect().x,y:document.querySelector(".space-scene-stage").getBoundingClientRect().y,width:document.querySelector(".space-scene-stage").clientWidth,height:document.querySelector(".space-scene-stage").clientHeight}, actors: [...document.querySelectorAll('.office-actor')].map(e => { const r=e.getBoundingClientRect();return { name:e.getAttribute('aria-label'), x:r.x,y:r.y,visible:r.x>=0&&r.right<=innerWidth&&r.y>=0&&r.bottom<=innerHeight }; }), draft: document.querySelector('textarea')?.value, composer: document.querySelector('textarea')?.getBoundingClientRect().toJSON() }));
      records.push({ width, step, ...data }); await page.screenshot({path:`${out}/${width}-${step}.png`}); return data;
    }
    await capture('overview');
    await page.getByRole('button', {name:'放大大厅',exact:true}).click(); const chosen = await capture('chosen-zoom');
    const actor=page.getByRole('button', { name: /@luna · 工作中/ });
    try { await actor.click({timeout:3000}); } catch { await page.getByRole('button',{name:'名册',exact:true}).click(); await page.getByRole('button',{name:'@luna · 工作中',exact:true}).click(); }
    const selected = await capture('selected'); await page.getByRole('textbox',{name:'Agent 消息草稿'}).fill('camera continuity');
    await page.getByRole('button',{name:'关闭并返回办公室',exact:true}).click(); const closed = await capture('closed');
    await page.getByRole('button',{name:'聊天',exact:true}).click(); await page.getByRole('button',{name:'@luna · 工作中',exact:true}).click();
    await page.getByRole('button',{name:'场景',exact:true}).click(); const returned = await capture('mode-return');
    await page.setViewportSize({width:width===1440?1100:width,height:480}); const resized = await capture('resized');
    if (process.env.AGENT_SPACE_ASSERT_CAMERA === '1') {
      for (const state of [selected, closed, returned, resized]) assert.equal(state.scale.x, chosen.scale.x, `${width}: camera zoom changed`);
      assert.deepEqual(closed.landmarks, chosen.landmarks, `${width}: close did not restore office camera`);
      assert.equal(returned.draft, 'camera continuity'); assert.equal(resized.draft, 'camera continuity');
      assert(resized.composer.bottom <= 480 && resized.composer.width > 0, `${width}: resized composer clipped`);
    }
    await page.getByRole('button',{name:'关闭并返回办公室',exact:true}).click(); const resizedClosed = await capture('resize-closed');
    if (process.env.AGENT_SPACE_ASSERT_CAMERA === '1') {
      assert.equal(resizedClosed.scale.x, chosen.scale.x);
      // DOM landmarks include the stage origin, which changes when the short-screen header compacts.
      const dx = resizedClosed.stage.x - chosen.stage.x + (resizedClosed.stage.width - chosen.stage.width) / 2, dy = resizedClosed.stage.y - chosen.stage.y + (resizedClosed.stage.height - chosen.stage.height) / 2;
      chosen.landmarks.slice(0,1).forEach((a,i) => { assert(Math.abs(resizedClosed.landmarks[i].x-a.x-dx)<1, `${width}: resized return x`); assert(Math.abs(resizedClosed.landmarks[i].y-a.y-dy)<1, `${width}: resized return y`); });
    }
    await page.getByRole('button',{name:'聊天',exact:true}).click(); await page.getByRole('button',{name:'场景',exact:true}).click(); const unselectedReturn=await capture('unselected-mode-return');
    if (process.env.AGENT_SPACE_ASSERT_CAMERA === '1') assert.deepEqual(unselectedReturn.landmarks,resizedClosed.landmarks);
    await page.setViewportSize({width,height:width===1440?1000:844});
    await page.getByRole('button',{name:'名册',exact:true}).click(); await page.getByRole('button',{name:'@luna · 工作中',exact:true}).click();
    if (width < 760) await page.getByRole('button',{name:'关闭并返回办公室',exact:true}).click();
    await page.getByRole('button',{name:'全景',exact:true}).click(); const panorama=await capture('panorama');
    const afterPanorama=await capture('panorama-stable');
    if (process.env.AGENT_SPACE_ASSERT_CAMERA === '1') { assert.equal(await page.getByRole('region',{name:'人物对话',exact:true}).isVisible(),false); assert.deepEqual(panorama.landmarks,afterPanorama.landmarks); assert(panorama.scale.x < chosen.scale.x); }
    await page.close();
  }
} finally { await browser.close(); await writeFile(`${out}/result.json`,JSON.stringify(records,null,2)); }
console.log(JSON.stringify(records.map(({width,step,scale,actors,draft,composer})=>({width,step,scale,visibleActors:actors.filter(a=>a.visible).length,draft,composer})),null,2));
