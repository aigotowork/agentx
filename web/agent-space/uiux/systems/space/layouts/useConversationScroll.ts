import { useLayoutEffect, useRef, useState } from "react";

type RecordAnchor = { id: string; offset: number };
export type LatestBookmark = { token: object; following: boolean; seen: number };
export interface ConversationScroll {
  key: string;
  positions: Map<string, number>;
  anchors?: Map<string, RecordAnchor>;
  ready: boolean;
  seek?: { id: string };
  latest?: {
    token: object;
    arrivals: number;
    beforeLayout: string;
    bookmarks: Map<string, LatestBookmark>;
    onOlder?(): void;
    firstId?: string;
  };
}

/** Preserve a semantic record position across prepend, reflow and exact-reader remounts. */
export function useConversationScroll(view?: ConversationScroll) {
  const scroll = useRef<HTMLDivElement>(null);
  const previousTop = useRef(0), programmaticTop = useRef<number | undefined>(undefined);
  const extent = useRef({ height: 0, length: 0 });
  const anchorSpace = useRef(0);
  const touchStart = useRef<number | undefined>(undefined);
  const [unread, setUnread] = useState(0);
  const { key, ready, positions, anchors, seek, latest } = view ?? {};
  const token = latest?.token, arrivals = latest?.arrivals ?? 0, bookmarks = latest?.bookmarks;

  function remember(region: HTMLElement) {
    if (!key || !ready || !region.clientHeight) return;
    positions?.set(key, region.scrollTop);
    if (!anchors) return;
    const rows = region.querySelectorAll<HTMLElement>('[data-history-record]');
    const top = region.getBoundingClientRect().top;
    let low = 0, high = rows.length;
    while (low < high) {
      const mid = (low + high) >>> 1;
      if (rows[mid].getBoundingClientRect().bottom <= top) low = mid + 1;
      else high = mid;
    }
    const row = rows[Math.min(low, rows.length - 1)];
    if (row) anchors.set(key, { id: row.dataset.historyRecord!, offset: top - row.getBoundingClientRect().top });
  }

  function place(region: HTMLElement, top: number, capture = true) {
    region.scrollTop = top;
    programmaticTop.current = previousTop.current = region.scrollTop;
    extent.current = { height: region.clientHeight, length: region.scrollHeight };
    if (capture) remember(region);
  }

  function spaceForAnchor(region: HTMLElement, size: number) {
    if (anchorSpace.current === size) return;
    anchorSpace.current = size;
    if (size) region.style.setProperty("--conversation-anchor-space", `${size}px`);
    else region.style.removeProperty("--conversation-anchor-space");
  }

  function goToEnd(region: HTMLElement) {
    spaceForAnchor(region, 0);
    place(region, region.scrollHeight);
  }

  function restore(region: HTMLElement, fallback: number) {
    const bookmark = key && anchors?.get(key);
    const row = bookmark && recordNode(region, bookmark.id);
    const offset = row && bookmark ? Math.min(bookmark.offset, row.getBoundingClientRect().height - 1) : 0;
    const target = row ? region.scrollTop + relativeTop(row, region) + offset : fallback;
    if (token) {
      // A short newest page can leave the anchored row above the natural bottom after a prepend.
      // Reserve only the missing trailing space so it can remain at the same screen offset.
      const naturalHeight = region.scrollHeight - anchorSpace.current;
      const extra = Math.ceil(Math.max(0, target + region.clientHeight - naturalHeight));
      spaceForAnchor(region, extra);
    } else spaceForAnchor(region, 0);
    place(region, target, !token || !row);
  }

  function follow(following: boolean) {
    if (!bookmarks || !token || !key) return;
    const existing = bookmarks.get(key);
    const seen = following || existing?.token !== token ? arrivals : existing.seen;
    bookmarks.set(key, { token, following, seen });
    setUnread(following ? 0 : arrivals - seen);
  }

  useLayoutEffect(() => {
    const region = scroll.current;
    if (!region || !key || !ready) return;
    function apply() {
      if (!region || !key || !region.clientHeight) return;
      if (!token) { restore(region, positions?.get(key) ?? 0); return; }
      const bookmark = bookmarks?.get(key);
      const anchor = anchors?.get(key);
      // An evicted reader starts anew unless its original record is still in the recent page.
      const following = bookmark?.token === token ? bookmark.following : bookmark?.following !== false || !anchor || !recordNode(region, anchor.id);
      if (following) goToEnd(region);
      else restore(region, positions?.get(key) ?? 0);
      follow(following);
    }
    apply();
    const geometry = new Map<Element, [number, number]>();
    const observer = new ResizeObserver(entries => {
      let changed = false;
      for (const { target } of entries) {
        const { width, height } = target.getBoundingClientRect();
        const previous = geometry.get(target);
        if (!previous || width !== previous[0] || (token && height !== previous[1])) {
          geometry.set(target, [width, height]);
          changed = true;
        }
      }
      if (changed) apply();
    });
    const observe = (target: Element) => {
      const { width, height } = target.getBoundingClientRect();
      geometry.set(target, [width, height]);
      observer.observe(target);
    };
    observe(region);
    if (token) for (const child of region.children) observe(child);
    return () => observer.disconnect();
    // DOM mutations that can move the bookmark: a prepend, live append or top boundary change.
  }, [key, ready, positions, anchors, token, bookmarks, arrivals, latest?.firstId, latest?.beforeLayout]);

  useLayoutEffect(() => {
    const region = scroll.current, node = region && seek && recordNode(region, seek.id);
    if (!region || !node) return;
    place(region, region.scrollTop + relativeTop(node, region));
    follow(atEnd(region));
  }, [key, seek]);

  function scrolled(region: HTMLDivElement) {
    if (!region.clientHeight) return;
    // Browser layout can clamp or round a programmatic position after the children reflow.
    // Those events must not turn a preserved bookmark into an intentional jump to the end.
    if (token && (extent.current.height !== region.clientHeight || extent.current.length !== region.scrollHeight)) return;
    const programmed = programmaticTop.current;
    programmaticTop.current = undefined;
    if (programmed !== undefined && Math.abs(programmed - region.scrollTop) <= 2) return;
    const prior = previousTop.current;
    previousTop.current = region.scrollTop;
    remember(region);
    if (region.scrollTop !== prior) {
      const reachedEnd = region.scrollTop > prior && atEnd(region);
      if (reachedEnd && anchorSpace.current) goToEnd(region);
      follow(reachedEnd);
    }
    if (region.scrollTop < prior && region.scrollTop <= region.clientHeight / 4) latest?.onOlder?.();
  }

  function demand(region: HTMLElement, direction: number) {
    if (direction < 0 && region.scrollTop <= 1) latest?.onOlder?.();
  }

  return {
    scroll,
    unread: token ? unread : 0,
    jumpToLatest() {
      const region = scroll.current;
      if (!region) return;
      goToEnd(region);
      follow(true);
      region.focus({ preventScroll: true });
    },
    scrolled,
    demand,
    touchStart,
  };
}

function atEnd(region: HTMLElement, margin = 2) {
  return region.clientHeight > 0 && region.scrollHeight - region.clientHeight - region.scrollTop <= margin;
}

function recordNode(region: HTMLElement, id: string) {
  return region.querySelector<HTMLElement>(`[data-history-record="${CSS.escape(id)}"]`);
}

function relativeTop(node: HTMLElement, region: HTMLElement) {
  return node.getBoundingClientRect().top - region.getBoundingClientRect().top;
}
