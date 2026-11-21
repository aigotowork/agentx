/** Export the style registry as a compact static catalogue for the running review build. */
import { access, copyFile, mkdir, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { parseArgs } from "node:util";
import { SPACE_STYLE_GROUPS } from "../../../web/agent-space/uiux/systems/space/components/presentation.ts";

const { positionals: [output, reference, captures], values } = parseArgs({
  allowPositionals: true,
  options: {
    office: { type: "string", default: "/studio/catalog" },
    caption: { type: "string", default: "同一个办公室 · 六位 Agent" },
    "reference-label": { type: "string", default: "Design Canvas" },
  },
});
if (!output || !reference || !captures) throw new Error("Usage: node catalogue.mjs BUILD_REVIEW_DIR REFERENCE_URL CAPTURES [--office PATH] [--caption TEXT] [--reference-label TEXT]");
await mkdir(output, { recursive: true });
const captureFiles = value => ["office", "reader"].map(state => `${value}-${state}.webp`);
const captured = new Set((await Promise.all(SPACE_STYLE_GROUPS.flatMap(group => group.styles).map(async ({ value }) => {
  try {
    await Promise.all(captureFiles(value).map(name => access(join(captures, name))));
    return value;
  } catch (error) {
    if (error.code !== "ENOENT") throw error;
    return null;
  }
}))).filter(value => value !== null));
const archiveGroups = SPACE_STYLE_GROUPS.map(group => ({ ...group, styles: group.styles.filter(style => captured.has(style.value)) })).filter(group => group.styles.length > 0);
if (!captured.size) throw new Error("No paired office and reader captures");
const escape = value => String(value).replace(/[&<>"']/g, char => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[char]);
const entry = style => "../?" + new URLSearchParams({ style, office: values.office });
function card(style) {
  const link = entry(style.value);
  const swatches = style.colors.map(color => `<span style="--swatch:${escape(color)}"></span>`).join("");
  return `<article class="sample"><a class="sample-cover" href="${escape(link)}" aria-label="试玩${escape(style.label)}"><img src="${style.value}-office.webp" alt="${escape(style.label)}办公室截图" width="1440" height="900" loading="lazy"></a><div class="sample-body"><div><h3>${escape(style.label)}</h3><p>${escape(style.note)}</p></div><div class="swatches" aria-hidden="true">${swatches}</div></div><div class="sample-actions"><a href="${escape(link)}">试玩 <span aria-hidden="true">↗</span></a><a href="${style.value}-reader.webp">会话截图</a></div></article>`;
}
const groups = archiveGroups.map(group => `<section id="${group.id}" aria-labelledby="${group.id}-title"><h2 id="${group.id}-title">${escape(group.label)} <span>${String(group.styles.length).padStart(2, "0")}</span></h2><div class="samples">${group.styles.map(card).join("")}</div></section>`).join("");
const tabs = archiveGroups.map(group => `<a href="#${group.id}">${escape(group.label)}</a>`).join("");
await Promise.all([...captured].flatMap(captureFiles).map(name => copyFile(join(captures, name), join(output, name))));
await writeFile(join(output, "index.html"), `<!doctype html>
<html lang="zh-CN"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>Space 风格目录</title><link rel="stylesheet" href="./catalogue.css"></head>
<body><header class="catalogue-header"><div class="catalogue-brand"><span class="brand-icon" aria-hidden="true"><svg width="26" height="26" viewBox="0 0 26 26" fill="none"><path d="M3 9 13 4l10 5v10l-10 4-10-4V9Z M3 9l10 5 10-5 M13 14v9 M8 11l10-5" stroke="currentColor" stroke-linejoin="round" stroke-width="1.5"/></svg></span><div class="brand-copy"><strong>AgentX Space</strong><small>STYLE LIBRARY</small></div></div><a href="${escape(reference)}">${escape(values["reference-label"])} ↗</a></header>
<main><div class="catalogue-intro"><div><h1>办公室风格目录</h1><p>${escape(values.caption)} · ${captured.size} 套风格</p></div><a class="catalogue-entry" href="${escape(entry("classic"))}">进入 Space 试玩 ↗</a></div>
<nav class="catalogue-nav" aria-label="风格分类">${tabs}</nav>${groups}</main></body></html>\n`);
await copyFile(new URL("./catalogue.css", import.meta.url), join(output, "catalogue.css"));
