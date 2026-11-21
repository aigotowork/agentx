/** @aiPurpose Render safe Markdown with reusable full-code copy feedback across HTTP and HTTPS. */
import { Fragment, useMemo } from "react";

import { CopyButton } from "./CopyButton";
import { parseMarkdown, type Block, type Inline } from "./markdown-parser";

/** @aiPurpose Render the transplanted safe Markdown grammar with local copy actions.
 * @aiUsage HTML stays text; images remain inert references to avoid implicit network requests.
 */
export function Markdown({ text }: { text: string }) {
  const blocks = useMemo(() => parseMarkdown(text, { streaming: true, linkify: true }), [text]);
  return <div className="markdown"><Blocks blocks={blocks} /></div>;
}
function Blocks({ blocks }: { blocks: Block[] }) { return <>{blocks.map((block, index) => <MarkdownBlock key={index} block={block} />)}</>; }
function MarkdownBlock({ block }: { block: Block }) {
  switch (block.kind) {
    case "paragraph": return <p><Inlines values={block.inlines} /></p>;
    case "heading": { const Heading = block.level <= 2 ? "h2" : "h3"; return <Heading><Inlines values={block.inlines} /></Heading>; }
    case "code": return <CodeBlock language={block.language} text={block.text} />;
    case "quote": return <blockquote><Blocks blocks={block.blocks} /></blockquote>;
    case "rule": return <hr />;
    case "list": { const entries = block.items.map((item, index) => <li key={index}><Blocks blocks={item.blocks} /></li>); return block.ordered ? <ol start={block.start}>{entries}</ol> : <ul>{entries}</ul>; }
    case "table": return <div className="markdown-table-scroll" tabIndex={0} role="region" aria-label="消息表格"><table><thead><tr>{block.header.map((cell, index) => <th key={index} className={`align-${block.align[index] ?? "left"}`}><Inlines values={cell} /></th>)}</tr></thead><tbody>{block.rows.map((row, index) => <tr key={index}>{row.map((cell, column) => <td key={column} className={`align-${block.align[column] ?? "left"}`}><Inlines values={cell} /></td>)}</tr>)}</tbody></table></div>;
  }
}
function Inlines({ values }: { values: Inline[] }) { return <>{values.map((value, index) => <Fragment key={index}>{inline(value)}</Fragment>)}</>; }
function inline(value: Inline): React.ReactNode {
  switch (value.kind) {
    case "text": return value.text;
    case "break": return <br />;
    case "code": return <code>{value.text}</code>;
    case "bold": return <strong><Inlines values={value.children} /></strong>;
    case "italic": return <em><Inlines values={value.children} /></em>;
    case "strike": return <del><Inlines values={value.children} /></del>;
    case "image": return <span className="offline-image-reference">[图片：{value.alt || "未命名"}]</span>;
    case "link": return /^(?:https?:|mailto:)/i.test(value.url) ? <a href={value.url} target="_blank" rel="noopener noreferrer"><Inlines values={value.children} /></a> : <span title={value.url}><Inlines values={value.children} /></span>;
  }
}
function CodeBlock({ language, text }: { language: string; text: string }) {
  return <div className="markdown-code"><div className="markdown-code-toolbar"><small>{language || "文本"}</small><CopyButton text={text} label="复制代码" /></div><pre tabIndex={0}><code>{text}</code></pre></div>;
}
