/** Short local mechanical feedback. A click never implies that a business operation succeeded. */
import { pixelMotion } from "../../themes/skin.native";
export type TapSound = "wood" | "metal" | "select";
const notes = { wood: [260, 120], metal: [880, 440], select: [520, 360] } as const;

export function createTapSound() {
  let context: AudioContext | undefined, last = -Infinity, disposed = false;
  return {
    async play(volume: number, kind: TapSound = "wood"): Promise<boolean> {
      if (disposed || volume <= 0 || !window.AudioContext || performance.now() - last < pixelMotion.soundGap) return false;
      last = performance.now();
      try {
        context ??= new AudioContext();
        if (context.state === "suspended") await context.resume();
        if (disposed || context.state !== "running") return false;
        const time = context.currentTime, tone = context.createOscillator(), gain = context.createGain(), filter = context.createBiquadFilter();
        tone.type = kind === "metal" ? "triangle" : "square";
        tone.frequency.setValueAtTime(notes[kind][0], time);
        tone.frequency.exponentialRampToValueAtTime(notes[kind][1], time + 0.035);
        filter.type = "lowpass";filter.frequency.value = kind === "metal" ? 2200 : 1100;
        gain.gain.setValueAtTime(Math.max(0, Math.min(1, volume)) * 0.12, time);
        gain.gain.exponentialRampToValueAtTime(0.001, time + 0.04);
        tone.connect(filter);filter.connect(gain);gain.connect(context.destination);
        tone.onended = () => { tone.disconnect();filter.disconnect();gain.disconnect(); };
        tone.start(time);tone.stop(time + 0.045);
        return true;
      } catch { return false; }
    },
    dispose() { disposed = true;if (context && context.state !== "closed") void context.close().catch(() => {}); },
  };
}
