import { useCallback, useEffect, useRef, useState } from "react";
import { Button } from "../../uiux/systems/space/components/Controls";
import "./feedback.css";

/**
 * @aiPurpose Name the explicit conversation window transitions that can play a cue.
 * @aiUsage Classify an actual state change before calling the workspace feedback hook.
 */
export type WorkspaceFeedbackEvent = "open" | "minimize" | "restore" | "close";

const SOUND_KEY = "agentx.space.window-sounds";
const CUE_PITCH: Record<WorkspaceFeedbackEvent, readonly [number, number]> = {
  open: [320, 460],
  minimize: [375, 250],
  restore: [280, 390],
  close: [290, 210],
};

function readSound() {
  try { return typeof window !== "undefined" && window.localStorage.getItem(SOUND_KEY) === "on"; }
  catch { return false; }
}

function startCue(context: AudioContext, [from, to]: readonly [number, number]) {
  const source = context.createOscillator();
  const level = context.createGain();
  const now = context.currentTime;
  const quiet = 0.0001;
  source.frequency.setValueAtTime(from, now);
  source.frequency.exponentialRampToValueAtTime(to, now + 0.042);
  level.gain.setValueAtTime(quiet, now);
  level.gain.exponentialRampToValueAtTime(0.055, now + 0.007);
  level.gain.exponentialRampToValueAtTime(quiet, now + 0.058);
  source.connect(level).connect(context.destination);
  source.onended = () => { source.disconnect(); level.disconnect(); };
  source.start(now);
  source.stop(now + 0.06);
  return source;
}

/**
 * @aiPurpose Own the page's opt-in window sounds; silence is the default.
 * @aiUsage Mount once in Space and call play directly from a deliberate window action that changes state.
 * @aiBehavior Toggle never plays; unavailable storage falls back to session state, and unavailable audio or gesture activation stays silent.
 */
export function useWorkspaceFeedback() {
  const [enabled, setEnabled] = useState(readSound);
  const preference = useRef(enabled);
  const audio = useRef<{ context?: AudioContext; voice?: OscillatorNode; revision: number }>({ revision: 0 });

  const toggle = useCallback(() => {
    const next = !preference.current;
    preference.current = next;
    setEnabled(next);
    try { window.localStorage.setItem(SOUND_KEY, next ? "on" : "off"); }
    catch { /* The session preference still works in restricted storage contexts. */ }
    if (!next) {
      audio.current.revision++;
      audio.current.voice?.stop();
      audio.current.voice = undefined;
    }
  }, []);

  const play = useCallback((event: WorkspaceFeedbackEvent) => {
    if (!preference.current || !window.navigator.userActivation?.isActive || !window.AudioContext) return;
    const state = audio.current;
    const revision = ++state.revision;
    try {
      const context = state.context ??= new AudioContext();
      const playLatest = () => {
        if (!preference.current || state.revision !== revision || context.state !== "running"
          || document.visibilityState !== "visible") return;
        state.voice?.stop();
        const voice = startCue(context, CUE_PITCH[event]);
        state.voice = voice;
        voice.addEventListener("ended", () => {
          if (state.voice === voice) state.voice = undefined;
        }, { once: true });
      };
      if (context.state === "running") playLatest();
      else void context.resume().then(playLatest).catch(() => {});
    } catch { /* Workspace state remains the confirmation when Web Audio is unavailable. */ }
  }, []);

  useEffect(() => {
    const state = audio.current;
    return () => {
      state.revision++;
      const context = state.context;
      state.voice = undefined;
      state.context = undefined;
      if (context) void context.close().catch(() => {});
    };
  }, []);

  return { enabled, toggle, play };
}

/**
 * @aiPurpose Offer an explicit, quiet-by-default preference for deliberate workspace actions.
 * @aiUsage Place once in the workspace controls and pass enabled and toggle from useWorkspaceFeedback.
 * @aiBehavior The stable button name and pressed state are readable by assistive technology; the icon shows mute.
 */
export function WorkspaceSoundToggle({ enabled, onToggle }: { enabled: boolean; onToggle(): void }) {
  const statePath = enabled ? "M13 7a4 4 0 0 1 0 6m2-8a7 7 0 0 1 0 10" : "m13 8 4 4m0-4-4 4";
  return (
    <Button className="space-window-sound" aria-label="窗口音效" aria-pressed={enabled} onClick={onToggle}>
      <svg viewBox="0 0 20 20" aria-hidden="true" focusable="false">
        <path d="M3 8h3l4-3v10l-4-3H3z" />
        <path d={statePath} />
      </svg>
      音效
    </Button>
  );
}
