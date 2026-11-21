import { readFile, readdir } from 'node:fs/promises';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import assert from 'node:assert/strict';
import puppeteer from 'puppeteer-core';

const proof = fileURLToPath(new URL('.', import.meta.url));
const pack = resolve(proof, '..');
const executablePath = process.env.CHROME_PATH;
assert(executablePath, 'Set CHROME_PATH to a local Chromium executable.');

async function assetsFromContract() {
  const scene = resolve(pack, '../../../..');
  const [studio, ambience, names] = await Promise.all([
    readFile(resolve(scene, 'studio-art.ts'), 'utf8'),
    readFile(resolve(scene, 'ambience.ts'), 'utf8'),
    readdir(pack),
  ]);
  const studioImages = studio.split('export const STUDIO_IMAGE = {')[1].split('} as const')[0];
  const ambienceImages = ambience.split('export const AMBIENCE_IMAGES = {')[1].split('} as const')[0];
  const shortcuts = Object.fromEntries(
    [...studio.matchAll(/const (ACTOR|GLYPH) = \[(\d+), (\d+),/g)].map(([, name, w, h]) => [name, [+w, +h]]),
  );
  const sizes = Object.fromEntries(
    [...(studioImages + ambienceImages).matchAll(/"?([\w-]+)"?: (?:\[(\d+), (\d+)|\b(ACTOR|GLYPH)\b)/g)]
      .map(([, name, w, h, alias]) => [name, alias ? shortcuts[alias] : [+w, +h]]),
  );
  assert.equal(Object.keys(sizes).length, 25);
  assert.deepEqual(names.filter(name => name.endsWith('.svg')).sort(), Object.keys(sizes).map(id => id + '.svg').sort());
  return Object.fromEntries(await Promise.all(Object.entries(sizes).map(async ([id, [w, h]]) => {
    const source = await readFile(resolve(pack, id + '.svg'), 'utf8');
    return [id, { w, h, source }];
  })));
}

const bottom = (id, x, y) => ({ id, x, y, a: .5, b: 1 });
const top = (id, x, y) => ({ id, x, y, a: 0, b: 0 });
const center = (id, x, y) => ({ id, x, y, a: .5, b: .5 });

const browser = await puppeteer.launch({ executablePath, args: ['--no-sandbox'] });
try {
  const page = await browser.newPage();
  await page.setViewport({ width: 800, height: 450, deviceScaleFactor: 1 });
  await page.setContent(`<style>
    *{box-sizing:border-box}body{margin:0;background:#17373d;color:#f4e1b5;font:13px system-ui}
    canvas{display:block}
    .cast{width:640px;padding:25px 27px 24px;background:linear-gradient(150deg,#335c5c,#25434b 75%,#20393e)}
    h1{margin:0 0 15px;font-size:24px;font-weight:550;letter-spacing:.06em}
    h1 span{float:right;margin-top:9px;font-size:10px;letter-spacing:.16em;text-transform:uppercase;color:#dfcfaa}
    .lineup{display:grid;grid-template-columns:repeat(4,1fr);gap:11px}
    figure{margin:0;min-width:0}
    .plate{height:107px;display:grid;place-items:center;border:1px solid #f0d19c55;background:linear-gradient(133deg,#dfc394,#bca27e)}
    figcaption{padding:7px 0 0;line-height:1.35}.role{display:block;font-size:11px;color:#d9c6a2}
  </style><canvas id="room" width="800" height="450"></canvas>
    <canvas id="thumbnail" width="480" height="270"></canvas><div class="cast" id="cast"></div>
    <canvas id="vista" width="800" height="450"></canvas>`);
  await page.evaluate(async ({ assets, placements }) => {
    const images = Object.fromEntries(await Promise.all(Object.entries(assets).map(async ([id, { source, w, h }]) => {
      const doc = new DOMParser().parseFromString(source, 'image/svg+xml');
      const root = doc.documentElement;
      if (root.localName !== 'svg' || doc.querySelector('parsererror,script,animate,animateTransform,animateMotion,foreignObject,style')) throw Error(id + ': invalid or active SVG');
      if ([...root.querySelectorAll('*')].some(el => [...el.attributes].some(attr => attr.name.startsWith('on') || (attr.localName === 'href' && !attr.value.startsWith('#'))))) throw Error(id + ': non-local resource');
      if (root.getAttribute('viewBox') !== `0 0 ${w} ${h}`) throw Error(id + ': unexpected viewBox');
      const image = new Image();
      image.src = 'data:image/svg+xml;charset=utf-8,' + encodeURIComponent(source);
      await image.decode();
      if (image.naturalWidth !== w || image.naturalHeight !== h) throw Error(id + ': unexpected intrinsic dimensions');
      return [id, image];
    })));
    const room = document.querySelector('#room');
    const ctx = room.getContext('2d');
    const draw = ({ id, x, y, a, b }) => {
      const image = images[id];
      ctx.drawImage(image, x - image.width * a, y - image.height * b);
    };
    const halo = (x, y, radius, opacity) => {
      const gradient = ctx.createRadialGradient(x,y,0,x,y,radius);
      gradient.addColorStop(0, '#fff2bdfd');
      gradient.addColorStop(.38, '#fff2bd60');
      gradient.addColorStop(1, '#fff2bd00');
      ctx.save();
      ctx.globalAlpha = opacity;
      ctx.fillStyle = gradient;
      ctx.fillRect(x-radius,y-radius,2*radius,2*radius);
      ctx.restore();
    };

    const { back, wall, furniture, people } = placements;
    const [x, y, w, h] = [136, 101, 528, 293];
    draw(back);
    document.querySelector('#vista').getContext('2d').drawImage(images['scene-vista'], 0, 0);
    ctx.fillStyle = '#1d404e55';
    ctx.fillRect(x+6,y+7,w+2,h+4);
    ctx.save();
    ctx.beginPath();
    ctx.rect(x,y,w,h);
    ctx.clip();
    for (let row = y; row < y+h; row += 128) {
      for (let column = x; column < x+w; column += 128) {
        ctx.drawImage(images['floor-oak'], column, row);
      }
    }
    draw(placements.rug);
    const shade = ctx.createLinearGradient(x,y+48,x+w*.78,y+h);
    shade.addColorStop(0,'#18354d6e');
    shade.addColorStop(.38,'#18354d00');
    shade.addColorStop(1,'#18354d79');
    ctx.fillStyle = shade;
    ctx.fillRect(x,y+48,w,h-48);
    halo(x+84,y+100,220,.1);
    halo(x+w-92,y+104,140,.07);
    halo(x+34,y+110,112,.56);
    for (let column = x; column < x+w; column += 160) ctx.drawImage(images['wall-plaster'],column,y);
    wall.forEach(draw);
    ctx.fillStyle = '#345e69';
    ctx.fillRect(x,y,6,h);
    ctx.fillRect(x+w-6,y,6,h);
    ctx.fillRect(x,y+h-6,32,6);
    ctx.fillRect(x+96,y+h-6,w-96,6);
    ctx.fillStyle = '#ecbe84';
    ctx.fillRect(x+1,y,2,h-6);
    ctx.fillRect(x+w-5,y,2,h-6);
    draw(placements.door);
    furniture.forEach(draw);
    halo(x+24,y+93,35,.58);
    ctx.restore();
    draw(placements.mail);
    people.forEach(draw);

    document.querySelector('#thumbnail').getContext('2d').drawImage(room,0,0,480,270);
    const cast = [
      ['maker','Ivo','Instrument maker'],
      ['designer','Mara','Aerial surveyor'],
      ['keeper','Edda','Keeper of the post'],
      ['scout','Tavi','Cliff route courier'],
    ];
    const article = document.querySelector('#cast');
    const header = document.createElement('h1');
    header.innerHTML = 'Windward <span>Cast · 64 × 72 at 1:1</span>';
    article.append(header);
    const row = document.createElement('div');
    row.className = 'lineup';
    article.append(row);
    for (const [id, name, role] of cast) {
      const figure = document.createElement('figure');
      figure.innerHTML = `<div class="plate"></div><figcaption>${name}<span class="role">${role}</span></figcaption>`;
      figure.firstElementChild.append(images['actor-' + id]);
      row.append(figure);
    }
  }, {
    assets: await assetsFromContract(),
    placements: {
      back: top('scene-vista', 0, 0),
      wall: [top('wall-window', 163, 102),top('wall-print', 373, 102),top('wall-window', 514, 102)],
      rug: top('rug', 445, 236),
      door: bottom('door-threshold', 200, 394),
      mail: bottom('human-mailbox', 96, 218),
      furniture: [
        bottom('plant', 636, 170), bottom('bookshelf', 375, 214), bottom('sofa', 492, 231),
        bottom('coffee', 607, 238), bottom('scene-lantern', 160, 224),
        bottom('desk', 254, 224), bottom('stool', 255, 248),
        bottom('desk', 359, 316), bottom('stool', 356, 342),
        bottom('cabinet', 609, 369),
      ],
      people: [
        bottom('actor-maker', 257, 238), center('glyph-work', 288, 195),
        bottom('actor-designer', 374, 302),
        bottom('actor-keeper', 487, 293), center('glyph-available', 518, 250),
        center('glyph-select', 295, 358), bottom('actor-scout', 295, 358),
        center('glyph-attention', 327, 315), center('envelope', 410, 299),
      ],
    },
  });

  for (const id of ['room', 'thumbnail', 'cast', 'vista']) {
    const element = await page.waitForSelector('#' + id);
    await element.screenshot({ path: resolve(proof, id + '.png') });
    await element.dispose();
  }
  console.log('25 matching, decoded SVGs; wrote the room, thumbnail, cast and vista proofs.');
} finally {
  await browser.close();
}
