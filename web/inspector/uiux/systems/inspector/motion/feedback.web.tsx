/** @aiPurpose Own optional browser feedback and cleanup; it never calls RPC or decides domain outcomes. */
import { useCallback, useEffect, useRef, useState } from "react";
import type { PluginTheme } from "../../../../host/contracts";
import {
  Button,
  ChoiceGroup,
  IconButton,
  Sheet,
  Switch,
} from "../components/kit/kit.native";
import {
  pixelMotion,
  pixelShape,
  type VisualStyle,
} from "../themes/skin.native";
import {
  feedbackDefaults,
  readFeedback,
  saveFeedback,
  type FeedbackPreferences,
  type ToolCursor,
} from "./feedback.preferences";
import { toolCursor } from "../assets/tools/tools.cursor";
import { createTapSound, type TapSound } from "../assets/sound/sound.web";
import "./feedback.styles.css";

const storage = {
  getItem: (key: string) => localStorage.getItem(key),
  setItem: (key: string, value: string) => localStorage.setItem(key, value),
};
const actionable =
  'button, [role="button"], [role="tab"], [role="switch"], a[href], summary';
function feedbackTarget(event: Event): Element | null {
  const target = event.target instanceof Element ? event.target : null;
  const control = target?.closest(actionable);
  if (
    !control ||
    control.closest(
      '[disabled], [aria-disabled="true"], [aria-busy="true"], [data-feedback="silent"]',
    ) ||
    target?.closest('input, textarea, select, [contenteditable="true"]')
  )
    return null;
  return control;
}

/** @aiPurpose Provide usePixelFeedback presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function usePixelFeedback(
  style: VisualStyle,
  system?: import("../themes/theme.resolve").ResolvedDesignSystem,
) {
  const [preferences, setPreferences] = useState(() => readFeedback(storage));
  const [systemReduced, setSystemReduced] = useState(
    () => matchMedia("(prefers-reduced-motion: reduce)").matches,
  );
  const [soundUnavailable, setSoundUnavailable] = useState(false);
  const audio = useRef<ReturnType<typeof createTapSound> | null>(null);
  const soundKind = system?.assets.sound ?? "wood";
  const latest = useRef({ style, preferences, soundKind });
  latest.current = { style, preferences, soundKind };
  const previewSound = useCallback(async (kind?: TapSound) => {
    const current = latest.current;
    if (
      current.style !== "pixel" ||
      !current.preferences.sound ||
      document.hidden
    )
      return;
    audio.current ??= createTapSound();
    if (
      !(await audio.current.play(
        current.preferences.volume,
        kind ?? current.soundKind,
      )) &&
      !window.AudioContext
    )
      setSoundUnavailable(true);
  }, []);
  useEffect(() => {
    const query = matchMedia("(prefers-reduced-motion: reduce)");
    const update = () => setSystemReduced(query.matches);
    query.addEventListener("change", update);
    return () => query.removeEventListener("change", update);
  }, []);
  useEffect(() => {
    const root = document.documentElement;
    root.dataset.motion =
      preferences.motion && !systemReduced ? "full" : "reduced";
    root.dataset.cursor = preferences.cursor;
    root.style.setProperty(
      "--inspector-tool-cursor",
      toolCursor(preferences.cursor),
    );
    root.style.setProperty(
      "--inspector-tool-pressed",
      toolCursor(preferences.cursor, true),
    );
    for (const [name, value] of Object.entries(system?.motion ?? pixelMotion))
      root.style.setProperty(
        `--inspector-motion-${name}`,
        system ? `var(--ds-motion-${name})` : `${value}ms`,
      );
    for (const [name, value] of Object.entries(system?.shape ?? pixelShape))
      if (typeof value === "number")
        root.style.setProperty(
          `--inspector-shape-${name}`,
          system ? `var(--ds-shape-${name})` : `${value}px`,
        );
    saveFeedback(preferences, storage);
  }, [preferences, systemReduced, system]);
  useEffect(() => {
    if (style !== "pixel" || !preferences.sound) {
      audio.current?.dispose();
      audio.current = null;
    }
  }, [style, preferences.sound]);
  useEffect(() => {
    if (style !== "pixel" || !window.IntersectionObserver) return;
    const observed = new Set<Element>();
    const observer = new IntersectionObserver((entries) => {
      for (const entry of entries)
        entry.target.setAttribute(
          "data-feedback-visible",
          String(entry.isIntersecting),
        );
    });
    const visit = (node: Node, add: boolean) => {
      if (!(node instanceof Element)) return;
      const elements = [
        ...(node.matches("[data-inspector-motion]") ? [node] : []),
        ...node.querySelectorAll("[data-inspector-motion]"),
      ];
      for (const element of elements) {
        if (add) {
          observed.add(element);
          observer.observe(element);
        } else {
          observed.delete(element);
          observer.unobserve(element);
          element.removeAttribute("data-feedback-visible");
        }
      }
    };
    visit(document.body, true);
    const mutations = new MutationObserver((records) => {
      for (const record of records) {
        record.removedNodes.forEach((node) => visit(node, false));
        record.addedNodes.forEach((node) => visit(node, true));
      }
    });
    mutations.observe(document.body, { childList: true, subtree: true });
    return () => {
      mutations.disconnect();
      observer.disconnect();
      for (const element of observed)
        element.removeAttribute("data-feedback-visible");
    };
  }, [style]);
  useEffect(() => {
    const root = document.documentElement;
    let downAt = 0,
      pointerId: number | null = null,
      held = false,
      pressedControl: Element | null = null;
    const reset = () => {
      delete root.dataset.toolPressed;
      pressedControl?.removeAttribute("data-feedback-pressed");
      pressedControl = null;
      pointerId = null;
    };
    const down = (event: PointerEvent) => {
      if (
        event.button !== 0 ||
        latest.current.style !== "pixel" ||
        !feedbackTarget(event)
      )
        return;
      downAt = performance.now();
      held = false;
      pointerId = event.pointerId;
      pressedControl = feedbackTarget(event);
      pressedControl?.setAttribute("data-feedback-pressed", "true");
      if (
        event.pointerType === "mouse" &&
        latest.current.preferences.motion &&
        !matchMedia("(prefers-reduced-motion: reduce)").matches
      )
        root.dataset.toolPressed = "true";
    };
    const up = (event: PointerEvent) => {
      if (event.pointerId === pointerId) {
        held = performance.now() - downAt > 450;
        reset();
      }
    };
    const move = (event: PointerEvent) => {
      if (event.pointerId !== pointerId || !pressedControl) return;
      const hit = document.elementFromPoint(event.clientX, event.clientY);
      if (!hit || !pressedControl.contains(hit)) {
        held = true;
        reset();
      }
    };
    const click = (event: MouseEvent) => {
      if (
        !event.isTrusted ||
        event.altKey ||
        event.button !== 0 ||
        (event.detail > 0 && held) ||
        !feedbackTarget(event)
      )
        return;
      // RN Web consumes propagation. Observe capture, then allow its handler to cancel a long press
      // before starting sound in the same user gesture. Never intercept or invoke the action itself.
      queueMicrotask(() => {
        if (!event.defaultPrevented) void previewSound();
      });
    };
    const keyDown = (event: KeyboardEvent) => {
      if (
        !event.isTrusted ||
        event.repeat ||
        latest.current.style !== "pixel" ||
        !["Enter", " "].includes(event.key) ||
        !feedbackTarget(event)
      )
        return;
      reset();
      pressedControl = feedbackTarget(event);
      pressedControl?.setAttribute("data-feedback-pressed", "true");
    };
    const keyUp = (event: KeyboardEvent) => {
      if (!event.isTrusted || !["Enter", " "].includes(event.key)) return;
      const control = feedbackTarget(event),
        accepted =
          control &&
          control === pressedControl &&
          document.activeElement === control;
      // Native buttons/links produce a click; RN Web switch/tab divs accept the key directly.
      if (accepted && !control.matches("button, a[href], summary"))
        queueMicrotask(() => {
          if (!event.defaultPrevented) void previewSound();
        });
      reset();
    };
    const visible = () => {
      root.dataset.feedbackActive = document.hidden ? "false" : "true";
      if (document.hidden) {
        reset();
        audio.current?.dispose();
        audio.current = null;
      }
    };
    const blur = () => {
      held = false;
      reset();
    };
    visible();
    document.addEventListener("pointerdown", down, true);
    document.addEventListener("pointerup", up, true);
    document.addEventListener("pointermove", move, {
      capture: true,
      passive: true,
    });
    document.addEventListener("pointercancel", blur, true);
    document.addEventListener("click", click, true);
    document.addEventListener("keydown", keyDown, true);
    document.addEventListener("keyup", keyUp, true);
    document.addEventListener("visibilitychange", visible);
    window.addEventListener("blur", blur);
    return () => {
      document.removeEventListener("pointerdown", down, true);
      document.removeEventListener("pointerup", up, true);
      document.removeEventListener("pointermove", move, true);
      document.removeEventListener("pointercancel", blur, true);
      document.removeEventListener("click", click, true);
      document.removeEventListener("keydown", keyDown, true);
      document.removeEventListener("keyup", keyUp, true);
      document.removeEventListener("visibilitychange", visible);
      window.removeEventListener("blur", blur);
      reset();
      audio.current?.dispose();
      audio.current = null;
    };
  }, [previewSound]);
  return {
    preferences,
    setPreferences,
    systemReduced,
    soundUnavailable,
    previewSound,
    pixel: style === "pixel",
  };
}
export type PixelFeedback = ReturnType<typeof usePixelFeedback>;

/** @aiPurpose Compose FeedbackControls presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function FeedbackControls({
  theme,
  feedback,
}: {
  theme: PluginTheme;
  feedback: PixelFeedback;
}) {
  const {
    preferences,
    setPreferences,
    systemReduced,
    soundUnavailable,
    previewSound,
    pixel,
  } = feedback;
  const patch = (next: Partial<FeedbackPreferences>) =>
    setPreferences((value) => ({ ...value, ...next }));
  return (
    <div className="pixel-feedback-controls">
      {!pixel && <p className="component-preview-status">切换到像素风后生效</p>}
      <div className="pixel-preference" data-feedback="silent">
        <span>界面动效</span>
        <Switch
          theme={theme}
          label="界面动效"
          value={preferences.motion}
          onChange={(motion) => patch({ motion })}
        />
      </div>
      {systemReduced && (
        <p className="component-preview-status">已遵循系统的减少动态设置</p>
      )}
      <div className="pixel-preference" data-feedback="silent">
        <span>敲击音效</span>
        <Switch
          theme={theme}
          label="敲击音效"
          value={preferences.sound}
          onChange={(sound) => patch({ sound })}
        />
      </div>
      <label className="pixel-preference" data-feedback="silent">
        音量
        <input
          aria-label="敲击音量"
          type="range"
          min="0"
          max="100"
          value={Math.round(preferences.volume * 100)}
          onChange={(event) =>
            patch({ volume: Number(event.target.value) / 100 })
          }
        />
      </label>
      <div data-feedback="silent">
        <ChoiceGroup
          theme={theme}
          label="工具光标"
          layout="wrap"
          value={preferences.cursor}
          onChange={(cursor: ToolCursor) => patch({ cursor })}
          options={[
            { value: "system", label: "系统" },
            { value: "hammer", label: "锤子" },
            { value: "shovel", label: "铲子" },
            { value: "chisel", label: "凿子" },
          ]}
        />
      </div>
      <div className="component-preview-row" data-feedback="silent">
        <Button
          theme={theme}
          label="试听木质敲击"
          disabled={!pixel || !preferences.sound || preferences.volume === 0}
          onPress={() => void previewSound("wood")}
        />
        <Button
          theme={theme}
          label="试听金属轻敲"
          disabled={!pixel || !preferences.sound || preferences.volume === 0}
          onPress={() => void previewSound("metal")}
        />
      </div>
      {soundUnavailable && (
        <p role="status" className="component-preview-status">
          当前浏览器不支持声音
        </p>
      )}
      <div data-feedback="silent">
        <Button
          theme={theme}
          label="恢复交互默认值"
          variant="ghost"
          onPress={() => setPreferences({ ...feedbackDefaults })}
        />
      </div>
    </div>
  );
}

/** @aiPurpose Compose FeedbackSettings presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function FeedbackSettings({
  theme,
  feedback,
}: {
  theme: PluginTheme;
  feedback: PixelFeedback;
}) {
  const [open, setOpen] = useState(false);
  return (
    <>
      <IconButton
        theme={theme}
        icon="Sparkles"
        label="像素交互设置"
        tooltip={false}
        onPress={() => setOpen(true)}
      />
      <Sheet title="像素交互" open={open} onClose={() => setOpen(false)}>
        <FeedbackControls theme={theme} feedback={feedback} />
      </Sheet>
    </>
  );
}
