/** @aiPurpose Supply safe semantic DOM primitives for named investigation regions. */
export type Child = Node | string | null | undefined | false;

/**
 * @aiPurpose Construct semantic markup with text-only remote values.
 * @aiUsage Use inside named region implementations; styling belongs to semantic CSS classes.
 */
export function el<K extends keyof HTMLElementTagNameMap>(tag: K, className: string, ...children: Child[]): HTMLElementTagNameMap[K] {
  const node = document.createElement(tag);
  if (className) node.className = className;
  for (const child of children) {
    if (child !== null && child !== undefined && child !== false) node.append(child);
  }
  return node;
}

/** @aiPurpose Provide native keyboard/touch action controls. @aiUsage Supply a concise label, callback and optional selected/disabled state. */
export function button(label: string, action: () => void, options: { active?: boolean; disabled?: boolean; testid?: string } = {}): HTMLButtonElement {
  const node = el("button", options.active ? "control control-selected" : "control", label);
  node.type = "button";
  node.disabled = options.disabled ?? false;
  if (options.active !== undefined) node.setAttribute("aria-pressed", String(options.active));
  if (options.testid) node.dataset.testid = options.testid;
  node.addEventListener("click", action);
  return node;
}

/** @aiPurpose Express evidence status with explicit text and tone. @aiUsage Use contextual native meanings, never color alone. */
export function badge(text: string, tone: "neutral" | "warning" | "good" = "neutral"): HTMLElement {
  return el("span", `badge badge-${tone}`, text);
}

/** @aiPurpose Align labeled diagnostic facts compactly. @aiUsage Pass meaningful label/value pairs inside evidence regions. */
export function facts(entries: Array<[string, Child]>): HTMLDListElement {
  const list = el("dl", "facts");
  for (const [label, value] of entries) list.append(el("div", "fact", el("dt", "fact-label", label), el("dd", "fact-value", value)));
  return list;
}

/** @aiPurpose Keep supporting evidence locally discoverable. @aiUsage Supply a keyboard/touch summary and subordinate content. */
export function disclosure(label: string, ...content: Child[]): HTMLDetailsElement {
  return el("details", "disclosure", el("summary", "disclosure-label", label), ...content);
}

/** @aiPurpose Describe absent or unread evidence honestly. @aiUsage Explain the bounded observation rather than implying global absence. */
export function empty(message: string): HTMLElement { return el("p", "empty-state", message); }

/** @aiPurpose Name a cohesive evidence capability in semantic markup. @aiUsage Compose sections inside the owning subject pane. */
export function region(title: string, ...content: Child[]): HTMLElement {
  const section = el("section", "evidence-section", el("h3", "section-title", title), ...content);
  section.setAttribute("aria-label", title);
  return section;
}
