// Export the actual composed HTML geometry as editable SVG rects, paths and text.
// No bitmap, foreignObject, embedded HTML or duplicate hand-drawn page source.
import fs from 'node:fs/promises';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {createRequire} from 'node:module';
const dir=path.dirname(fileURLToPath(import.meta.url));
const require=createRequire(path.resolve(dir,'../../plugin/package.json'));
const {chromium}=require('@playwright/test');
const manifest=JSON.parse(await fs.readFile(path.join(dir,'design.json'),'utf8'));
const browser=await chromium.launch({headless:true});
await fs.mkdir(path.join(dir,'svg'),{recursive:true});
for(const b of manifest.artboards){
 const page=await browser.newPage({viewport:{width:b.width,height:b.height}});
 await page.goto('file://'+path.join(dir,b.file));
 await page.evaluate(()=>document.fonts.ready);
 const svg=await page.evaluate(({title,width,height})=>{
  const enc=v=>String(v).replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&apos;'}[c]));
  const num=v=>Math.round(v*100)/100;
  const token=name=>getComputedStyle(document.documentElement).getPropertyValue('--'+name).trim();
  const transparent=v=>!v||v==='transparent'||v==='rgba(0, 0, 0, 0)'||v==='none';
  let defs=[],counter=0;
  const box=e=>{const r=e.getBoundingClientRect();return {x:r.x,y:r.y,w:r.width,h:r.height};};
  const clip=(r)=>{const id='clip-'+(++counter);defs.push(`<clipPath id="${id}"><rect x="${num(r.x)}" y="${num(r.y)}" width="${num(r.w)}" height="${num(r.h)}"/></clipPath>`);return id;};
  function textNode(node,style){
   const value=node.nodeValue||'';if(!value.trim())return '';
   let parts=[],last=null;
   for(let i=0;i<value.length;i++){
    const range=document.createRange();range.setStart(node,i);range.setEnd(node,i+1);const r=range.getBoundingClientRect();
    if(r.width<.01||r.height<.01)continue;
    const ch=value[i];
    if(last&&Math.abs(last.y-r.y)<1&&r.x-last.end<2&&r.x>=last.x){last.text+=ch;last.end=r.right;}
    else{last={x:r.x,y:r.y,end:r.right,text:ch};parts.push(last);}
   }
   return parts.map(p=>`<text x="${num(p.x)}" y="${num(p.y)}" dominant-baseline="text-before-edge" fill="${enc(style.color)}" font-family="${enc(style.fontFamily)}" font-size="${enc(style.fontSize)}" font-weight="${enc(style.fontWeight)}"${style.letterSpacing!=='normal'?` letter-spacing="${enc(style.letterSpacing)}"`:''}>${enc(p.text)}</text>`).join('');
  }
  function controlText(e,s,r){
   if(e.tagName==='INPUT'&&e.type==='checkbox'){const z=Math.min(r.w,r.h);return `<rect x="${num(r.x)}" y="${num(r.y)}" width="${num(z)}" height="${num(z)}" rx="2" fill="${e.checked?token('accent'):token('surface')}" stroke="${token('line-strong')}"/>${e.checked?`<path d="M${num(r.x+3)} ${num(r.y+7)}l3 3 6-6" fill="none" stroke="${token('surface')}" stroke-width="2"/>`:''}`;}
   const txt=e.tagName==='SELECT'?e.options[e.selectedIndex]?.text:e.value||e.placeholder||'';
   const x=r.x+parseFloat(s.paddingLeft)+parseFloat(s.borderLeftWidth);const y=r.y+(r.h-parseFloat(s.fontSize)*1.4)/2;
   return `<text x="${num(x)}" y="${num(y)}" dominant-baseline="text-before-edge" fill="${enc(e.value||e.tagName==='SELECT'?s.color:getComputedStyle(e,'::placeholder').color)}" font-family="${enc(s.fontFamily)}" font-size="${enc(s.fontSize)}" font-weight="${enc(s.fontWeight)}">${enc(txt)}</text>`;
  }
  function paint(e){
   if(!(e instanceof Element)||['SCRIPT','STYLE','LINK'].includes(e.tagName))return '';
   const s=getComputedStyle(e),r=box(e);if(s.display==='none'||s.visibility==='hidden'||Number(s.opacity)===0||r.w===0||r.h===0)return '';
   if(r.x>=width||r.y>=height||r.x+r.w<=0||r.y+r.h<=0)return '';
   if(e instanceof SVGElement){const xml=new XMLSerializer().serializeToString(e);return xml.replace('<svg ',`<svg x="${num(r.x)}" y="${num(r.y)}" width="${num(r.w)}" height="${num(r.h)}" color="${enc(s.color)}" `);}
   let shapes='';const rad=parseFloat(s.borderTopLeftRadius)||0;
   if(!transparent(s.backgroundColor))shapes+=`<rect x="${num(r.x)}" y="${num(r.y)}" width="${num(r.w)}" height="${num(r.h)}" rx="${num(rad)}" fill="${enc(s.backgroundColor)}"/>`;
   const edges=[['Top',r.x,r.y,r.x+r.w,r.y],['Right',r.x+r.w,r.y,r.x+r.w,r.y+r.h],['Bottom',r.x,r.y+r.h,r.x+r.w,r.y+r.h],['Left',r.x,r.y,r.x,r.y+r.h]];
   for(const [side,x1,y1,x2,y2] of edges){const w=parseFloat(s['border'+side+'Width']);const c=s['border'+side+'Color'];if(w&&!transparent(c)&&s['border'+side+'Style']!=='none')shapes+=`<path d="M${num(x1)} ${num(y1)}H${num(x2)}${y1!==y2?`V${num(y2)}`:''}" fill="none" stroke="${enc(c)}" stroke-width="${w}"/>`;}
   let children='';
   if(['INPUT','SELECT','TEXTAREA'].includes(e.tagName))children=controlText(e,s,r);
   else for(const child of e.childNodes)children+=child.nodeType===3?textNode(child,s):paint(child);
   if(['auto','hidden','scroll'].includes(s.overflow)||['auto','hidden','scroll'].includes(s.overflowY)||s.textOverflow==='ellipsis')children=`<g clip-path="url(#${clip(r)})">${children}</g>`;
   return `<g${e.dataset.dcId?` id="${enc(e.dataset.dcId)}"`:''}${s.opacity!=='1'?` opacity="${s.opacity}"`:''}>${shapes}${children}</g>`;
  }
  const body=paint(document.body);
  return `<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}" role="img" aria-labelledby="title desc"><title id="title">${enc(title)}</title><desc id="desc">Agent Inspector 像素案头。离线样例，真实 HTML 布局导出的可编辑矢量；文字保留为 text，图标保留为 path。交互请打开对应 HTML。</desc><defs>${defs.join('')}</defs>${body}</svg>`;
 },{title:b.title,width:b.width,height:b.height});
 await fs.writeFile(path.join(dir,'svg',b.id+'.svg'),svg+'\n');
 await page.close();
}
await browser.close();
process.stdout.write('Exported 6 editable vector artboards from their HTML source.\n');
