/** Open a native modal or drawer. Returns true only when a closed dialog was opened. */
export function openDialog(dialog, trigger = document.activeElement) {
  if (!dialog?.isConnected || typeof dialog.showModal !== 'function' || dialog.open) return false;
  dialog.returnValue = '';
  dialog.showModal();
  if (trigger instanceof HTMLElement && !dialog.contains(trigger)) {
    dialog.addEventListener('close', () => {
      // Prefer the browser's restoration; don't undo deliberate product focus.
      if (!dialog.open && trigger.isConnected &&
          (document.activeElement === document.body || dialog.contains(document.activeElement))) {
        trigger.focus();
      }
    }, { once: true });
  }
  return true;
}

/** Close a dialog with an optional local intent value. Does not dispatch a product action. */
export function closeDialog(dialog, returnValue = '') {
  if (dialog?.open) dialog.close(returnValue);
}

/** Toggle an auto popover from a custom invoker. Prefer declarative popovertarget. */
export function togglePopover(trigger, panel) {
  if (!trigger?.isConnected || !panel?.isConnected || typeof panel.togglePopover !== 'function') return false;
  const open = !panel.matches(':popover-open');
  // Older implementations may accept the click but ignore the newer source option.
  if (open) {
    panel.toggleAttribute('data-ax-unanchored', !('popoverTargetElement' in trigger && trigger.popoverTargetElement === panel));
    panel.showPopover({ source: trigger });
  } else {
    panel.hidePopover();
  }
  return open;
}
