import assert from 'node:assert/strict';
import { readFile, readdir } from 'node:fs/promises';
import puppeteer from 'puppeteer-core';

const here = new URL('./', import.meta.url);
const scene = new URL('../../../../', here);
const [studio, ambience, entries] = await Promise.all([
  readFile(new URL('studio-art.ts', scene), 'utf8'),
  readFile(new URL('ambience.ts', scene), 'utf8'),
  readdir(here),
]);
const aliases = Object.fromEntries([...studio.matchAll(/const (ACTOR|GLYPH) = \[(\d+), (\d+),/g)]
  .map(([, id, w, h]) => [id, [+w, +h]]));
const imageTables = studio.split('export const STUDIO_IMAGE = {')[1].split('} as const')[0]
  + ambience.split('export const AMBIENCE_IMAGES = {')[1].split('} as const')[0];
const sizes = Object.fromEntries([...imageTables.matchAll(/"?([\w-]+)"?: (?:\[(\d+), (\d+)|\b(ACTOR|GLYPH)\b)/g)]
  .map(([, id, w, h, alias]) => [id + '.svg', alias ? aliases[alias] : [+w, +h]]));
assert.equal(Object.keys(sizes).length, 25);
assert.deepEqual(entries.filter(file => file.endsWith('.svg')).sort(), Object.keys(sizes).sort());
const assets = await Promise.all(Object.entries(sizes).map(async ([file, size]) => ({
  file, size, source: await readFile(new URL(file, here), 'utf8'),
})));

const browser = await puppeteer.launch({ executablePath: process.env.CHROME_PATH ?? '/usr/bin/google-chrome', args: ['--no-sandbox'] });
try {
  const page = await browser.newPage();
  const failures = [];
  page.on('requestfailed', request => failures.push(request.url()));
  page.on('pageerror', error => failures.push(error.message));
  await page.setViewport({ width: 828, height: 660, deviceScaleFactor: 1 });
  await page.goto(new URL('proof.html', here).href, { waitUntil: 'load' });
  await page.evaluate(async assets => {
    await Promise.all(assets.map(async ({ file, size: [w, h], source }) => {
      const doc = new DOMParser().parseFromString(source, 'image/svg+xml');
      const svg = doc.documentElement;
      const fail = reason => { throw Error(file + ': ' + reason); };
      if (svg.localName !== 'svg' || doc.querySelector('parsererror,script,style,image,animate,animateMotion,animateTransform,set,foreignObject')) fail('invalid or non-static vector');
      if (svg.getAttribute('viewBox') !== '0 0 ' + w + ' ' + h || svg.getAttribute('width') !== String(w) || svg.getAttribute('height') !== String(h)) fail('unexpected intrinsic dimensions');
      for (const el of [svg, ...svg.querySelectorAll('*')]) {
        for (const attr of el.attributes) {
          if (attr.name.startsWith('on') || (attr.localName === 'href' && !attr.value.startsWith('#')) || /url\(\s*[^#]/i.test(attr.value)) fail('external resource or active content');
        }
      }
      const image = new Image();
      image.src = URL.createObjectURL(new Blob([source], { type: 'image/svg+xml' }));
      await image.decode();
      if (image.naturalWidth !== w || image.naturalHeight !== h) fail('browser decoded unexpected dimensions');
      const canvas = Object.assign(document.createElement('canvas'), { width: w, height: h });
      const ctx = canvas.getContext('2d');
      ctx.drawImage(image, 0, 0);
      if (!ctx.getImageData(0, 0, w, h).data.some((channel, index) => index % 4 === 3 && channel)) fail('empty render');
    }));
  }, assets);
  const room = await page.$('.scene');
  const cast = await page.$('.cast');
  const capture = (element, name) => element.screenshot({ path: new URL(name + '.png', here).pathname });
  await capture(room, 'room');
  await capture(cast, 'cast');
  await page.setViewport({ width: 628, height: 660, deviceScaleFactor: 1 });
  assert.equal(await room.evaluate(el => el.getBoundingClientRect().width), 600);
  await capture(room, 'thumbnail');
  const narrow = await browser.newPage();
  narrow.on('requestfailed', request => failures.push(request.url()));
  narrow.on('pageerror', error => failures.push(error.message));
  await narrow.setViewport({ width: 390, height: 844, deviceScaleFactor: 1, isMobile: true, hasTouch: true });
  await narrow.goto(new URL('proof.html', here).href, { waitUntil: 'load' });
  assert.equal(await narrow.evaluate(() => document.documentElement.scrollWidth), 390);
  assert.deepEqual(await narrow.$$eval('.cast img', images => images.map(el => [el.width, el.height])), [[64, 72], [64, 72], [64, 72], [64, 72]]);
  await narrow.screenshot({ path: new URL('narrow.png', here).pathname, clip: { x: 0, y: 0, width: 390, height: 460 } });
  assert.deepEqual(failures, []);
  console.log('Chrome decoded and painted all 25 SVGs at exact sizes; captured 800px and 600px rooms, 64×72 cast, and a 390px layout with no overflow.');
} finally {
  await browser.close();
}
