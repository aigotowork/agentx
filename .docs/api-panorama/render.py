#!/usr/bin/env python3
"""Project the source-linked panorama tables into a portable offline browser view.

README.md owns the explanatory snapshot; native Rust/HTTP declarations own APIs.
This renderer neither registers operations nor generates the SDK or CLI; it renders the source-linked status/index view.
"""

import argparse
import html
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent
document = (ROOT / "README.md").read_text()
operations = []
for domain, table in re.findall(r"<!-- panorama:(\w+) -->(.*?)<!-- /panorama -->", document, re.S):
    rows = [line for line in table.splitlines() if line.startswith("| ")][2:]
    for row in rows:
        fields = [value.strip() for value in row.strip("|").split("|")]
        if len(fields) != 5:
            raise ValueError(f"Expected five columns: {row}")
        operations.append(dict(zip(["sdk", "http", "input", "result", "cli"], fields), domain=domain))

if not operations:
    raise ValueError("No panorama tables found")

payload = json.dumps(operations, ensure_ascii=False).replace("<", "\\u003c")
title = html.escape("AgentX · API / SDK / CLI 全景")
page = r'''<!doctype html>
<html lang="zh-CN"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>__TITLE__</title>
<style>
:root{--paper:#f7f8fa;--surface:#fff;--ink:#202d40;--muted:#5d6b7e;--line:#dde3ec;--accent:#2558c9;--tint:#edf3ff;--known:#166b4d;--proposal:#906213;--radius:8px;--gap:16px;--small-gap:8px;--font:14px;--small-font:12px;--title:22px;--mono:ui-monospace,SFMono-Regular,Consolas,monospace;--border:1px;--pad:20px;--nav:166px;--detail:330px;--head:64px;--diagram:122px;--shadow:0 5px 22px #2033480b;--focus:3px;--row:10px;--narrow:390px;--diagram-min-width:0px}
*{box-sizing:border-box}body{margin:0;background:var(--paper);color:var(--ink);font:var(--font)/1.55 system-ui,sans-serif}button,input{font:inherit}button,a,input{outline-offset:var(--focus)}button{cursor:pointer}header{padding:var(--small-gap) var(--pad);display:flex;align-items:center;justify-content:space-between;gap:var(--gap);min-height:var(--head);border-bottom:var(--border) solid var(--line);background:var(--surface)}h1{font-size:var(--title);margin:0;font-weight:650}header small,.muted{color:var(--muted)}a{color:var(--accent)}.status{font-size:var(--small-font);color:var(--proposal)}.pipeline{margin:var(--gap) var(--pad);height:var(--diagram);background:var(--surface);border:var(--border) solid var(--line);border-radius:var(--radius);padding:var(--small-gap);box-shadow:var(--shadow);overflow-x:auto}.pipeline svg{width:100%;min-width:var(--diagram-min-width);height:100%}.pipeline text{font-family:system-ui,sans-serif;fill:var(--ink)}.pipeline .node{fill:var(--paper);stroke:var(--line)}.pipeline .proposed{stroke-dasharray:5 4}.pipeline .active .node{fill:var(--tint);stroke:var(--accent)}.pipeline .edge{stroke:var(--line);fill:none}.pipeline .hint{fill:var(--muted)}.workspace{display:grid;grid-template-columns:var(--nav) minmax(0,1fr) var(--detail);gap:var(--gap);padding:0 var(--pad) var(--pad);height:calc(100dvh - var(--head) - var(--diagram) - var(--gap) - var(--gap))}.domains,.catalog,.detail{min-width:0;overflow:auto}.domains{display:flex;flex-direction:column;gap:var(--small-gap)}.domain{border:var(--border) solid transparent;background:transparent;text-align:left;padding:var(--row);border-radius:var(--radius);color:var(--muted)}.domain[aria-pressed=true]{color:var(--accent);background:var(--tint);border-color:var(--line)}.domain span{display:block;font-size:var(--small-font)}.catalog,.detail{background:var(--surface);border:var(--border) solid var(--line);border-radius:var(--radius)}.toolbar{display:flex;gap:var(--small-gap);padding:var(--small-gap);border-bottom:var(--border) solid var(--line);position:sticky;top:0;background:var(--surface);align-items:center}.toolbar input{width:100%;min-width:0;border:var(--border) solid var(--line);border-radius:var(--radius);padding:var(--small-gap);color:var(--ink);background:var(--paper)}.count{white-space:nowrap;font:var(--small-font) var(--mono);color:var(--muted)}table{border-collapse:collapse;width:100%;font-size:var(--small-font)}th,td{text-align:left;padding:var(--row);border-bottom:var(--border) solid var(--line);vertical-align:top;overflow-wrap:anywhere}th{color:var(--muted);font-weight:500}td:first-child{width:43%}td button{border:0;background:transparent;text-align:left;padding:0;color:var(--ink);font:inherit;overflow-wrap:anywhere;width:100%}td code{font-family:var(--mono)}tr[aria-selected=true]{background:var(--tint)}tr[aria-selected=true] button{color:var(--accent)}.cli{color:var(--muted);display:block;margin-top:var(--small-gap)}.detail{padding:var(--gap)}.detail h2{font:600 var(--font)/1.5 var(--mono);overflow-wrap:anywhere;margin:var(--small-gap) 0 var(--gap)}.badge{display:inline-block;border-radius:var(--radius);background:var(--tint);color:var(--known);font-size:var(--small-font);padding:0 var(--small-gap)}.detail dt{font-size:var(--small-font);color:var(--muted);margin-top:var(--gap)}.detail dd{margin:var(--small-gap) 0;overflow-wrap:anywhere}.detail pre{white-space:pre-wrap;overflow-wrap:anywhere;font:var(--small-font)/1.7 var(--mono);background:var(--paper);padding:var(--small-gap);border-radius:var(--radius)}.detail .boundary{border-top:var(--border) solid var(--line);padding-top:var(--gap);color:var(--muted);font-size:var(--small-font)}.empty{padding:var(--gap)}
@media(max-width:1000px){:root{--detail:270px;--nav:135px;--pad:12px;--gap:10px}.cli{font-size:var(--small-font)}}
@media(max-width:700px){:root{--title:18px;--diagram-min-width:1120px;--pad:10px}.workspace{grid-template-columns:minmax(0,1fr);grid-template-rows:auto minmax(120px,1fr) minmax(180px,.8fr)}.domains{flex-direction:row;overflow-x:auto;flex-shrink:0}.domain{white-space:nowrap;padding:var(--small-gap)}.domain span{display:none}.detail{padding:var(--small-gap)}.detail dl{margin:0}.detail dt{margin-top:var(--small-gap)}header small{display:none}.detail .boundary{display:none}h1{font-size:var(--title)}}
</style></head>
<body><header><div><h1>AgentX · API / SDK / CLI</h1><small>35 个 HTTP 方法／路径组合 → 59 行操作对照 · 2026-11-17 源码快照</small></div><div><div class="status">HTTP · OpenAPI · SDK / CLI 已实现</div><a href="README.md">完整契约与命令树 ↗</a></div></header>
<section class="pipeline" aria-label="原生契约到三套 HTTP 与已实现 SDK CLI 的链路">
<svg viewBox="0 0 1120 100" role="img" aria-labelledby="pipeline-title"><title id="pipeline-title">原生领域声明供给三个 HTTP 入口，经实际 OpenAPI 供给 SDK 与 CLI</title>
<defs><marker id="arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse"><path d="M 0 0 L 10 5 L 0 10 z" fill="currentColor"/></marker></defs>
<g class="edge" marker-end="url(#arrow)"><path d="M160 50H210V18H245"/><path d="M160 50H245"/><path d="M160 50H210V82H245"/><path d="M440 18H475V50H510"/><path d="M440 50H510"/><path d="M440 82H475V50H510"/><path d="M675 50H725"/><path d="M890 50H940"/></g>
<rect class="node" x="5" y="27" width="155" height="46" rx="6"/><text x="82" y="48" text-anchor="middle" font-size="14">Rust / serde</text><text class="hint" x="82" y="64" text-anchor="middle" font-size="10">领域对象与操作</text>
<g data-lane="management"><rect class="node" x="245" y="3" width="195" height="27" rx="5"/><text x="342" y="21" text-anchor="middle" font-size="12">Management · Agent / 消息</text></g>
<g data-lane="workbench"><rect class="node" x="245" y="36" width="195" height="27" rx="5"/><text x="342" y="54" text-anchor="middle" font-size="12">Workbench · Work / 验收</text></g>
<g data-lane="observation"><rect class="node" x="245" y="69" width="195" height="27" rx="5"/><text x="342" y="87" text-anchor="middle" font-size="12">Observation · 运行证据</text></g>
<rect class="node active" x="510" y="27" width="165" height="46" rx="6"/><text x="592" y="48" text-anchor="middle" font-size="14">OpenAPI · 已派生</text><text class="hint" x="592" y="64" text-anchor="middle" font-size="10">路由 + schema + 错误 + 编码</text>
<rect class="node active" x="725" y="27" width="165" height="46" rx="6"/><text x="807" y="48" text-anchor="middle" font-size="14">生成客户端 + 薄 SDK</text><text class="hint" x="807" y="64" text-anchor="middle" font-size="10">精确引用 / 回执 / codec</text>
<rect class="node active" x="940" y="27" width="175" height="46" rx="6"/><text x="1027" y="48" text-anchor="middle" font-size="14">agentx-http · 已实现 CLI</text><text class="hint" x="1027" y="64" text-anchor="middle" font-size="10">resource → action → options</text></svg></section>
<main class="workspace"><nav class="domains" aria-label="契约领域" id="domains"></nav><section class="catalog" aria-label="操作对照"><div class="toolbar"><input type="search" id="search" aria-label="搜索 SDK、HTTP、参数或 CLI" placeholder="搜索方法、资源、参数…"><output class="count" id="count" aria-live="polite"></output></div><table><thead><tr><th>建议 SDK / CLI</th><th>现有 HTTP</th></tr></thead><tbody id="rows"></tbody></table><p id="empty" class="empty" hidden>没有匹配操作</p></section><aside class="detail" id="detail" aria-label="选中操作的参数与结果"></aside></main>
<noscript>请打开 README.md 阅读完整静态全景。</noscript>
<script type="application/json" id="catalog">__DATA__</script>
<script>
'use strict';
const operations=JSON.parse(document.querySelector('#catalog').textContent);
const domains=[['all','全部操作','59 行 · 含路由别名'],['management','Agent 管理','30 · Agent / 消息 / 监控'],['workbench','任务协作','15 · Work / 检查 / 接受'],['observation','运行诊断','14 · 请求 / 配置 / 事件']];
const policies={management:'独立 management URL；普通 JSON；G 与精确 AgentRef。Operation 回执只表示对应操作的进展。',workbench:'独立 workbench URL；整数标记 codec；只允许 OperatorCommand。AgentReport 未公开。',observation:'独立 observation URL；整数标记 codec；只读已有证据，不启动 daemon。'};
const sources={management:'../../src/management/web.rs',workbench:'../../src/collaboration/service.rs',observation:'../../src/observability/web/api.rs'};
let domain='all',query='',selected=Number(location.hash.replace('#op-',''));
if(!location.hash.startsWith('#op-')||!operations[selected])selected=0;
function escapeHtml(value){return String(value).replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));}
function PipelineView(){document.querySelectorAll('[data-lane]').forEach(node=>node.classList.toggle('active',node.dataset.lane===operations[selected].domain));}
function DomainNav(){document.querySelector('#domains').innerHTML=domains.map(([id,name,hint])=>`<button class="domain" data-domain="${id}" aria-pressed="${id===domain}">${name}<span>${hint}</span></button>`).join('');}
function OperationDetail(){const item=operations[selected];document.querySelector('#detail').innerHTML=`<span class="badge">HTTP 已有源码 · ${escapeHtml(item.domain)}</span><h2>${escapeHtml(item.sdk)}</h2><dl><dt>HTTP</dt><dd><code>${escapeHtml(item.http)}</code></dd><dt>输入 · G / R / W / A / O 展开见完整文档</dt><dd>${escapeHtml(item.input)}</dd><dt>结果与效果</dt><dd>${escapeHtml(item.result)}</dd><dt>CLI（历史索引命名）</dt><dd><pre>agentx-http ${escapeHtml(item.cli)}</pre></dd></dl><p class="boundary">${escapeHtml(policies[item.domain])}</p><a href="${sources[item.domain]}">原生来源 ↗</a>`;PipelineView();}
function OperationList(){const matches=operations.map((item,index)=>({item,index})).filter(({item})=>(domain==='all'||item.domain===domain)&&Object.values(item).join(' ').toLowerCase().includes(query));if(matches.length&&!matches.some(({index})=>index===selected))selected=matches[0].index;document.querySelector('#rows').innerHTML=matches.map(({item,index})=>`<tr aria-selected="${index===selected}" data-operation="${index}"><td><button data-select="${index}" aria-label="查看 ${escapeHtml(item.sdk)}"><code>${escapeHtml(item.sdk)}</code><span class="cli">${escapeHtml(item.cli)}</span></button></td><td><code>${escapeHtml(item.http)}</code></td></tr>`).join('');document.querySelector('#count').value=`${matches.length} / ${operations.length}`;document.querySelector('#empty').hidden=matches.length!==0;OperationDetail();}
document.querySelector('#domains').addEventListener('click',event=>{const button=event.target.closest('[data-domain]');if(!button)return;domain=button.dataset.domain;DomainNav();OperationList();});
document.querySelector('#search').addEventListener('input',event=>{query=event.target.value.toLowerCase();OperationList();});
document.querySelector('#rows').addEventListener('click',event=>{const row=event.target.closest('[data-operation]');if(!row)return;selected=Number(row.dataset.operation);history.replaceState(null,'',`#op-${selected}`);OperationList();});
window.addEventListener('hashchange',()=>{const index=Number(location.hash.replace('#op-',''));if(location.hash.startsWith('#op-')&&operations[index]){selected=index;domain='all';query='';document.querySelector('#search').value='';DomainNav();OperationList();}});
DomainNav();OperationList();
</script></body></html>'''
(ROOT / "index.html").write_text(page.replace("__TITLE__", title).replace("__DATA__", payload))
print(f"Rendered {len(operations)} source-linked operation rows into {ROOT / 'index.html'}")

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--serve", action="store_true", help="Serve the panorama and linked public source files")
    parser.add_argument("--port", type=int, default=0, help="Preview port; default: an available port")
    args = parser.parse_args()
    if args.serve:
        from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
        import socket

        project = ROOT.parents[1]
        allowed = {"/": ROOT / "index.html", "/index.html": ROOT / "index.html", "/README.md": ROOT / "README.md"}
        for source in re.findall(r"\]\((\.\./\.\./src/[^)#]+)(?:#[^)]*)?\)", document):
            local = (ROOT / source).resolve()
            if local.is_relative_to(project / "src") and local.is_file():
                allowed["/" + str(local.relative_to(project))] = local

        class Preview(SimpleHTTPRequestHandler):
            """Expose only the view, its document and explicitly linked source files."""

            def translate_path(self, path):
                return str(allowed.get(path.split("?", 1)[0], ROOT / ".not-served"))

        server = ThreadingHTTPServer(("0.0.0.0", args.port), Preview)
        probe = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        probe.connect(("192.0.2.1", 9))
        print(f"PREVIEW http://{probe.getsockname()[0]}:{server.server_port}/index.html", flush=True)
        probe.close()
        try:
            server.serve_forever()
        except KeyboardInterrupt:
            pass
        finally:
            server.server_close()
