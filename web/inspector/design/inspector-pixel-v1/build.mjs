import fs from 'node:fs/promises';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';
import {initial,renderApp} from './src/app.mjs';
import {glyphs} from './src/icons.mjs';
const dir=path.dirname(fileURLToPath(import.meta.url));
const read=p=>fs.readFile(path.join(dir,p),'utf8');
const hash=s=>createHash('sha256').update(s).digest('hex');
const tokens=JSON.parse(await read('src/tokens.json'));
const css=`:root{${Object.entries(tokens).filter(([k])=>k!=='themes').map(([k,v])=>`--${k}:${v};`).join('')}}\n${await read('src/styles.css')}\n${Object.entries(tokens.themes||{}).map(([name,values])=>`[data-theme="${name}"]{${Object.entries(values).map(([k,v])=>`--${k}:${v};`).join('')}}`).join('\n')}`;
const shared=(await read('src/icons.mjs')).replaceAll('export ','')+'\n'+(await read('src/app.mjs')).replace(/^import .*;\n/,'').replaceAll('export ','');
const runtime=await read('src/runtime.js');
const boards=[
 {id:'visual-language',title:'01 像素案头 · 视觉与组件',scene:'system',width:1440,height:1040,x:0,y:0,group:'视觉体系 / 推荐'},
 {id:'work-overview',title:'02 工作巡检 · 找到需要你的 Agent',scene:'overview',width:1440,height:960,x:1520,y:0,group:'工作路径 / 调查'},
 {id:'recovery-investigation',title:'03 恢复调查 · 原因、证据与局部操作',scene:'recovery',width:1440,height:960,x:3040,y:0,group:'工作路径 / 调查'},
 {id:'delivery-unknown',title:'04 投递未知 · 保留操作、不补发',scene:'unknown',width:1280,height:900,x:0,y:1160,group:'结果 / 覆盖 / 窄屏'},
 {id:'usage-coverage',title:'05 用量与覆盖 · 保留日志的边界',scene:'usage',width:1440,height:1040,x:1360,y:1160,group:'结果 / 覆盖 / 窄屏'},
 {id:'mobile-workspace',title:'06 窄屏 · 同一对象与调查工作区',scene:'mobile',width:390,height:844,x:2880,y:1160,group:'结果 / 覆盖 / 窄屏'}
];
let previous={};try{previous=JSON.parse(await read('.generated.json'));}catch{}
const written={};
async function write(file,content){let old;try{old=await read(file);}catch{}if(old&&previous[file]&&hash(old)!==previous[file])throw Error(`保留用户编辑：${file} 与生成基线不同，请先合并编辑再生成。`);await fs.mkdir(path.dirname(path.join(dir,file)),{recursive:true});await fs.writeFile(path.join(dir,file),content);written[file]=hash(content);}
function html(title,body,script=''){return `<!doctype html><html lang="zh-CN"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>${title} · Agent Inspector</title><style>${css}</style></head><body>${body}${script}</body></html>`;}
for(const b of boards){const s=initial(b.scene);if(b.scene==='unknown'){s.tab='result';s.receipt='unknown';}const scripts=`<script id="initial-state" type="application/json">${JSON.stringify(s)}</script><script>${shared}\n${runtime}</script>`;await write(`${b.id}.html`,html(b.title,`<div id="app">${renderApp(s)}</div>`,scripts));}
await fs.mkdir(path.join(dir,'icons'),{recursive:true});
for(const [name,d]of Object.entries(glyphs))await write(`icons/${name}.svg`,`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" width="32" height="32" role="img" aria-label="${name}"><path fill="${tokens.ink}" d="${d}"/></svg>\n`);
const manifest={version:1,title:'Agent Inspector · 像素案头 01',kind:'prototype',description:'参考 bots 画布内部的轻像素语言，转译为高密度 Agent 管理工作区。6 张中文离线画板，含搜索/筛选、调查、确认、未知反馈与用量下钻。所有动作只改本地状态。',artboards:boards.map(({scene,...b})=>({...b,file:`${b.id}.html`}))};
// Native canvas coordinates are user-owned once a design is present.
try{const old=JSON.parse(await read('design.json'));for(const b of manifest.artboards){const o=old.artboards?.find(x=>x.id===b.id);if(o){b.x=o.x;b.y=o.y;}}}catch{}
await fs.writeFile(path.join(dir,'design.json'),JSON.stringify(manifest,null,2)+'\n');
const galleryCss=`.review-page{padding:var(--space-8);max-width:var(--viewport-max);margin:auto}.review-head{display:flex;justify-content:space-between;gap:var(--space-6);padding-bottom:var(--space-6);border-bottom:var(--border-strong)}.review-head h1{font-size:var(--display);margin-block:var(--space-3)}.review-head p{font-size:var(--text-sm);color:var(--muted)}.review-symbol{width:var(--sidebar-foot);height:var(--sidebar-foot);background:var(--accent-soft);padding:var(--space-5);color:var(--accent)}.review-symbol svg{width:100%;height:100%}.review-grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(280px,1fr));gap:var(--space-5);margin-top:var(--space-6)}.review-card{display:block;text-decoration:none;color:var(--ink);background:var(--surface);border:var(--border-strong);box-shadow:var(--shadow);padding:var(--space-5)}.review-card:hover{border-color:var(--accent)}.review-card h2{font-size:var(--text-lg);margin-block:var(--space-3)}.review-card p{font-size:var(--text-xs);color:var(--muted)}.review-card>span{font-size:var(--text-xs);color:var(--accent);display:block;margin-top:var(--space-4)}.review-foot{margin-top:var(--space-8);font-size:var(--text-sm);line-height:var(--line-loose);color:var(--muted)}.review-links{display:flex;gap:var(--space-4);flex-wrap:wrap;margin-top:var(--space-4)}.review-links a{color:var(--accent)}`;
const {icon}=await import('./src/icons.mjs');
const gallery=`<main class="review-page"><header class="review-head"><div><div class="eyebrow">AGENT INSPECTOR / DESIGN STUDY 01</div><h1>像素案头</h1><p>轻像素的管理工作区 · 中文可操作原型 · 离线样例</p><div class="review-links"><a href="recovery-investigation.html">直接开始调查 →</a><a href="README.md">设计说明与适配取舍</a><a href="svg/recovery-investigation.svg">查看可编辑 SVG</a></div></div><div class="review-symbol">${icon('inspector')}</div></header><section class="review-grid">${boards.map((b,i)=>`<a class="review-card" href="${b.id}.html" data-dc-id="review-${b.id}"><div class="eyebrow">0${i+1} / ${b.width} × ${b.height}</div><h2>${b.title.slice(3)}</h2><p>${['视觉、图标、控件与表达强度的推荐。','从关注队列选中对象，查证据，再返回。','停止证据、局部展开与模拟发送确认。','独立的未知反馈，保留原操作，不自动补发。','精确用量、独立覆盖边界与返回连续性。','390 宽完整工作区；同一文件支持 320 宽。'][i]}</p><span>打开可操作画板 →</span></a>`).join('')}</section><div class="review-foot"><p>走查建议：搜索「支付」→选中 Agent→展开来源→继续原 Agent→选择「投递未知」→模拟发送→刷新观察→原会话→返回队列。按 Escape 关闭局部面板或返回。图标支持 focus / hover / 触摸长按提示。</p><p>本候选未接入业务、未改现行 RN kit；画布内可按元素批注。所有数字为同一批离线样例。</p><div class="review-links">${boards.map(b=>`<a href="svg/${b.id}.svg">${b.title.slice(3)} SVG</a>`).join('')}</div></div></main>`;
await write('index.html',html('像素案头 · 评审入口',gallery).replace('</style>',galleryCss+'</style>'));
await fs.writeFile(path.join(dir,'.generated.json'),JSON.stringify(written,null,2)+'\n');
process.stdout.write(`Generated ${boards.length} self-contained artboards and ${Object.keys(glyphs).length} vector icons.\n`);
