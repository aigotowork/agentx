/** Browser-only event adapter. No DOM globals or types escape this module. */
import { Platform } from "react-native";

type BrowserEvent = { key?: string; defaultPrevented?: boolean; preventDefault(): void; stopImmediatePropagation(): void };
type BrowserEvents = {
  addEventListener(type: string, listener: (event: BrowserEvent) => void, options?: boolean): void;
  removeEventListener(type: string, listener: (event: BrowserEvent) => void, options?: boolean): void;
};
declare const document: BrowserEvents;
declare const window: BrowserEvents;

/** Capture Escape before a host dialog dismisses; only consume it when a tooltip was actually closed. */
export function subscribeTooltipDismiss(dismiss: (reason: "escape" | "scroll" | "layout") => boolean): () => void {
  if (Platform.OS !== "web" || typeof document === "undefined" || typeof window === "undefined") return () => {};
  const key = (event: BrowserEvent) => {
    if (event.key !== "Escape" || event.defaultPrevented || !dismiss("escape")) return;
    event.preventDefault(); event.stopImmediatePropagation();
  };
  const scroll = () => { dismiss("scroll"); }, resize = () => { dismiss("layout"); };
  document.addEventListener("keydown", key, true);
  document.addEventListener("scroll", scroll, true);
  window.addEventListener("resize", resize);
  return () => {
    document.removeEventListener("keydown", key, true);
    document.removeEventListener("scroll", scroll, true);
    window.removeEventListener("resize", resize);
  };
}
