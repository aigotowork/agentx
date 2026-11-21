/** @aiPurpose Copy text from a user gesture across HTTP and HTTPS while preserving editing state. */

/**
 * @aiPurpose Copy full text and reject explicitly when the browser refuses both available paths.
 * @aiUsage Invoke directly from a click; await before showing success and present rejection to the user.
 */
export async function copyText(text: string): Promise<void> {
  try {
    if (typeof globalThis.navigator?.clipboard?.writeText === "function") {
      await globalThis.navigator.clipboard.writeText(text);
      return;
    }
  } catch { /* A denied async clipboard may still allow gesture-based copy. */ }
  if (!copyWithSelection(text)) throw new Error("复制失败，请选中文本后手动复制。");
}

function copyWithSelection(text: string): boolean {
  if (typeof document === "undefined" || !document.body || typeof document.execCommand !== "function") return false;
  const active = document.activeElement as HTMLElement | null;
  const control = active?.tagName === "INPUT" || active?.tagName === "TEXTAREA" ? active as HTMLInputElement | HTMLTextAreaElement : null;
  const start = control?.selectionStart; const end = control?.selectionEnd; const direction = control?.selectionDirection;
  const selection = document.getSelection();
  const ranges = selection ? Array.from({ length: selection.rangeCount }, (_, index) => selection.getRangeAt(index).cloneRange()) : [];
  const transfer = document.createElement("textarea");
  transfer.className = "clipboard-transfer"; transfer.value = text; transfer.readOnly = true;
  transfer.tabIndex = -1; transfer.setAttribute("aria-hidden", "true");
  try {
    const boundary = active?.closest(".space-system") ?? document.body;
    boundary.append(transfer);
    transfer.focus({ preventScroll: true }); transfer.select(); transfer.setSelectionRange(0, text.length);
    return document.execCommand("copy");
  } catch { return false; }
  finally {
    transfer.remove();
    try {
      active?.focus({ preventScroll: true });
      if (control && start != null && end != null) control.setSelectionRange(start, end, direction ?? undefined);
      if (selection && ranges.length) {
        selection.removeAllRanges();
        for (const range of ranges) selection.addRange(range);
      }
    } catch { /* The original selection may have been removed by a copy event. */ }
  }
}
