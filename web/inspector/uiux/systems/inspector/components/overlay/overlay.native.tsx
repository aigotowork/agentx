/** Portable, nonmodal tooltips. Boundaries own mounting; controls only supply intent and an anchor. */
import * as React from "react";
import type { PluginTheme } from "../../../../../host/contracts";
import { createContext, useCallback, useContext, useEffect, useId, useMemo, useRef, useState, useSyncExternalStore, type ReactNode, type RefObject } from "react";
import { StyleSheet, Text, View, type PressableProps } from "react-native";
import { createOverlaySession, initialTooltipIntent, placeTooltip, tooltipIntent, tooltipWanted, tooltipWidthLimit,
  type OverlaySession, type TooltipDismissReason, type TooltipIntentEvent } from "./overlay.model";
import { hairline, tooltipTokens } from "../../foundations/native.tokens";
import { subscribeTooltipDismiss } from "./overlay.web";
import { useForeground } from "../../platforms/foreground.native";

type Anchor = RefObject<View | null>;
interface OverlayContextValue {
  theme: PluginTheme | null;
  enabled: boolean;
  session: OverlaySession;
  request(owner: string, label: string, anchor: Anchor, dismissed: () => void): void;
  suspend(): () => void;
}
const OverlayContext = createContext<OverlayContextValue | null>(null);
const emptySnapshot = () => null;
const noSubscription = () => () => {};

export interface OverlayBoundaryProps {
  /** Surface roots pass their current host theme; modal boundaries inherit it. */
  theme?: PluginTheme;
  /** Changing the host/owner invalidates all retained measurements and intent. */
  scopeKey?: string;
  /** Content boundaries shrink with a host dialog; they never grow it to fill the screen. */
  mode?: "surface" | "content";
  active?: boolean;
  /** Suspend ancestor tooltips while this host modal is open. */
  modal?: boolean;
  children: ReactNode;
}

/** Mount outside scrolling content. A host Modal gets its own content-sized boundary. */
/** @aiPurpose Compose OverlayBoundary presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function OverlayBoundary({ theme, scopeKey, mode = "surface", active = true, modal = false, children }: OverlayBoundaryProps) {
  const parent = useContext(OverlayContext), colorsTheme = theme ?? parent?.theme ?? null;
  const foreground = useForeground(), enabled = active && foreground && colorsTheme !== null;
  const root = useRef<View>(null), [session] = useState(createOverlaySession);
  const snapshot = useSyncExternalStore(session.subscribe, session.getSnapshot, emptySnapshot);
  const request = useCallback((owner: string, label: string, anchor: Anchor, dismissed: () => void) => {
    if (!enabled || !root.current || !anchor.current) return;
    const revision = session.open(owner, label, dismissed);
    if (revision === null) return;
    // Both APIs report window coordinates on native and RN Web. Stale callbacks are rejected by session.
    root.current.measureInWindow((x, y, width, height) => {
      const boundary = { x, y, width, height };
      anchor.current?.measureInWindow((ax, ay, aw, ah) => {
        session.measure(owner, revision, { boundary, anchor: { x: ax, y: ay, width: aw, height: ah } });
      });
    });
  }, [enabled, session]);
  const suspend = useCallback(() => {
    const release = session.suspend(), releaseParent = parent?.suspend();
    return () => { release(); releaseParent?.(); };
  }, [session, parent]);
  const context = useMemo<OverlayContextValue>(() => ({ theme: colorsTheme, enabled, session, request, suspend }), [colorsTheme, enabled, session, request, suspend]);
  useEffect(() => { session.dismiss("scope"); }, [session, scopeKey, enabled]);
  useEffect(() => { if (modal && active) return parent?.suspend(); }, [modal, active, parent]);
  useEffect(() => () => session.dismiss("unmount"), [session]);
  useEffect(() => {
    if (!active) return;
    return subscribeTooltipDismiss(reason => {
      const hadTooltip = session.getSnapshot() !== null;
      session.dismiss(reason);
      return hadTooltip;
    });
  }, [active, session]);
  const dismiss = (reason: TooltipDismissReason) => session.dismiss(reason);
  const geometry = snapshot?.geometry, limit = geometry ? tooltipWidthLimit(geometry.boundary, tooltipTokens) : 0;
  const placement = geometry && snapshot?.size ? placeTooltip(geometry, snapshot.size, tooltipTokens) : null;
  const maxHeight = geometry ? Math.max(0, geometry.boundary.height - tooltipTokens.margin * 2) : 0;
  return <OverlayContext.Provider value={context}>
    <View ref={root} collapsable={false} style={mode === "surface" ? styles.surface : styles.content}
      onLayout={() => dismiss("layout")} onTouchStart={() => dismiss("press")} onPointerDown={() => dismiss("press")}
      onAccessibilityEscape={() => dismiss("escape")}>
      {children}
      {enabled && colorsTheme && snapshot && geometry && limit > 0 ? <View pointerEvents="none" style={styles.layer}>
        <View style={[styles.position, { width: limit, left: placement?.left ?? tooltipTokens.margin, top: placement?.top ?? tooltipTokens.margin, opacity: placement ? 1 : 0 }]}>
          <View key={`${snapshot.owner}:${snapshot.revision}`} nativeID={tooltipId(snapshot.owner)} role="tooltip" accessible accessibilityLabel={snapshot.label}
            onLayout={event => { session.layout(snapshot.owner, snapshot.revision, event.nativeEvent.layout); }}
            style={[styles.bubble, { maxWidth: limit, maxHeight, borderColor: colorsTheme.colors.border, backgroundColor: colorsTheme.colors.surface2 }]}>
            <Text style={[styles.label, { color: colorsTheme.colors.foreground }]}>{snapshot.label}</Text>
          </View>
        </View>
      </View> : null}
    </View>
  </OverlayContext.Provider>;
}

function tooltipId(owner: string): string { return `inspector-tooltip-${owner.replace(/[^a-zA-Z0-9_-]/g, "")}`; }
export type TooltipBinding = Pick<PressableProps, "onHoverIn" | "onHoverOut" | "onPointerEnter" | "onPointerLeave" | "onFocus" | "onBlur" | "onLongPress" | "onPressIn" | "onPressOut" | "onPress" | "onLayout" | "delayLongPress"> & {
  ref: Anchor;
  "aria-describedby"?: string;
};

/** Bind directly to the existing Pressable; this adds neither a layout wrapper nor a nested button. */
/** @aiPurpose Provide useTooltip presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function useTooltip({ label, onPress }: { label?: string | false; onPress?: () => void }): TooltipBinding {
  const context = useContext(OverlayContext), owner = useId(), ref = useRef<View>(null);
  const current = useRef({ label, onPress }); current.current = { label, onPress };
  const intent = useRef(initialTooltipIntent()), heldPress = useRef(false), pressing = useRef(false);
  const session = context?.session;
  const enabled = !!label && !!context?.enabled;
  const snapshot = useSyncExternalStore(enabled ? session!.subscribe : noSubscription, enabled ? session!.getSnapshot : emptySnapshot, emptySnapshot);
  const dismissed = useCallback(() => { intent.current = tooltipIntent(intent.current, "dismiss"); }, []);
  const reconcile = useCallback(() => {
    const text = current.current.label;
    if (typeof text !== "string" || !context) return;
    if (tooltipWanted(intent.current)) context.request(owner, text, ref, dismissed);
    else context.session.dismiss("leave", owner);
  }, [context, dismissed, owner]);
  const event = (kind: TooltipIntentEvent) => { intent.current = tooltipIntent(intent.current, kind); reconcile(); };
  useEffect(() => {
    if (session?.getSnapshot()?.owner !== owner) return;
    if (!label) session.dismiss("leave", owner); else reconcile();
  }, [label, session, owner, reconcile]);
  useEffect(() => () => { session?.dismiss("unmount", owner); }, [session, owner]);
  return {
    ref,
    "aria-describedby": snapshot?.owner === owner ? tooltipId(owner) : undefined,
    ...(enabled ? {
      onHoverIn: () => event("hover-in"), onHoverOut: () => event("hover-out"),
      onPointerEnter: pointer => { if (pointer.nativeEvent.pointerType !== "touch") event("hover-in"); },
      onPointerLeave: pointer => { if (pointer.nativeEvent.pointerType !== "touch") event("hover-out"); },
      onFocus: () => { if (!pressing.current) event("focus"); }, onBlur: () => event("blur"),
      onLayout: () => session?.dismiss("layout", owner),
      delayLongPress: tooltipTokens.longPressMs,
      onLongPress: () => { heldPress.current = true; event("hold"); },
    } satisfies Partial<TooltipBinding> : {}),
    onPressIn: () => { pressing.current = true; heldPress.current = false; session?.dismiss("press"); dismissed(); },
    onPressOut: () => { pressing.current = false; },
    onPress: () => {
      session?.dismiss("press"); dismissed();
      if (!heldPress.current) current.current.onPress?.();
    },
  };
}

/** Render prop for a semantic control that does not use the kit. Its accessible name stays on that control. */
/** @aiPurpose Compose Tooltip presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function Tooltip({ label, onPress, children }: { label: string; onPress?: () => void; children: (binding: TooltipBinding) => ReactNode }) {
  const binding = useTooltip({ label, onPress });
  return children(binding);
}

const styles = StyleSheet.create({
  surface: { flex: 1, minWidth: 0, minHeight: 0 },
  content: { flexShrink: 1, minWidth: 0, minHeight: 0 },
  layer: { position: "absolute", top: 0, right: 0, bottom: 0, left: 0, zIndex: tooltipTokens.layer, pointerEvents: "none" },
  position: { position: "absolute", alignItems: "flex-start" },
  bubble: { alignSelf: "flex-start", flexShrink: 1, paddingHorizontal: tooltipTokens.paddingHorizontal, paddingVertical: tooltipTokens.paddingVertical,
    borderRadius: tooltipTokens.radius, borderWidth: hairline, overflow: "hidden" },
  label: { fontSize: tooltipTokens.fontSize, lineHeight: tooltipTokens.lineHeight },
});
