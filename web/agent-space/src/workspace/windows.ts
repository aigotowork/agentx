/** Page-owned organization of exact conversations. The native reader supply owns their records. */
export interface ConversationWindow {
  id: string;
  minimized: boolean;
}

export interface ConversationWindows {
  panes: readonly ConversationWindow[];
  activeId: string | null;
  focused: boolean;
  recent: readonly string[];
}

const MAX_OPEN_WINDOWS = 8;

export const emptyWindows: ConversationWindows = { panes: [], activeId: null, focused: false, recent: [] };

export function openWindow(state: ConversationWindows, id: string): ConversationWindows {
  const existing = state.panes.find(pane => pane.id === id);
  if (existing) {
    if (id === state.activeId && !existing.minimized) return state;
    return { ...state, panes: state.panes.map(pane => pane.id === id ? { ...pane, minimized: false } : pane), activeId: id,
      recent: [id, ...state.recent.filter(previous => previous !== id)] };
  }
  // Prefer replacing an inactive shelf item at capacity; the underlying draft remains exact.
  const eviction = state.panes.length >= MAX_OPEN_WINDOWS
    ? (state.panes.find(pane => pane.minimized) ?? state.panes.find(pane => pane.id !== state.activeId))?.id
    : undefined;
  return { ...state, panes: [...state.panes.filter(pane => pane.id !== eviction), { id, minimized: false }], activeId: id,
    recent: [id, ...state.recent.filter(previous => previous !== eviction)] };
}

export function removeWindow(state: ConversationWindows, id: string, minimize: boolean): ConversationWindows {
  if (!state.panes.some(pane => pane.id === id)) return state;
  const panes = minimize
    ? state.panes.map(pane => pane.id === id ? { ...pane, minimized: true } : pane)
    : state.panes.filter(pane => pane.id !== id);
  const recent = state.recent.filter(previous => previous !== id);
  if (id !== state.activeId) return { ...state, panes, recent };
  const activeId = recent.find(previous => panes.some(pane => pane.id === previous && !pane.minimized)) ?? null;
  return { panes, activeId, focused: false, recent };
}

export function toggleWindowFocus(state: ConversationWindows, id: string): ConversationWindows {
  const next = openWindow(state, id);
  return { ...next, focused: !(state.focused && state.activeId === id) };
}

export function visibleWindows(state: ConversationWindows, capacity: 1 | 2): string[] {
  if (!state.activeId) return [];
  if (state.focused || capacity === 1) return [state.activeId];
  const regular = state.panes.filter(pane => !pane.minimized);
  const companion = state.recent.find(id => id !== state.activeId && regular.some(pane => pane.id === id));
  return regular.filter(pane => pane.id === state.activeId || pane.id === companion).map(pane => pane.id);
}
