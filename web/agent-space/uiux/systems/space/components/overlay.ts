/** One foremost interactive layer. Hints close alongside it and never steal an Escape. */
export interface OverlayEntry { priority: number; passive?: boolean; close(): void; contains?(node: Node | null): boolean }
export class OverlayStack {
  private entries = new Map<symbol, OverlayEntry>();
  add(entry: OverlayEntry): () => void { const id = Symbol(); this.entries.set(id, entry); return () => { this.entries.delete(id); }; }
  get active(): OverlayEntry | undefined {
    return [...this.entries.values()].filter(entry => !entry.passive).sort((a, b) => a.priority - b.priority).at(-1);
  }
  dismiss(): boolean {
    const target = this.active;
    const hints = [...this.entries.values()].filter(entry => entry.passive);
    for (const hint of hints) hint.close();
    target?.close();
    return !!target || hints.length > 0;
  }
}
export function editingTarget(target: EventTarget | null): boolean {
  return target instanceof Element && !!target.closest('input,textarea,select,[contenteditable="true"],[role="textbox"]');
}
