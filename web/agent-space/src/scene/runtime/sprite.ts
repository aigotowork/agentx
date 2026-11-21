/** Exact image-box placement and sprite selection shared by drawing and on-demand diagnostics. */
import { character, CHARACTER_FRAMES, POSE_FRAME_MS, frameAt, loopOf, lookFor, type Bitmap } from "../art/art";
import type { HallFrame, SceneBehavior } from "../motion";
export function spriteBounds(bitmap: Bitmap, x: number, y: number, scale = 2) {
  return { x: Math.round(x - bitmap.anchor.x * scale), y: Math.round(y - bitmap.anchor.y * scale), width: bitmap.width * scale, height: bitmap.height * scale };
}
/** The renderer and diagnostics share the exact sprite-frame calculation. */
export function actorAnimation(frame: HallFrame, elapsed: number, still: boolean) {
  return {pose:frame.pose,facing:frame.facing,carry:frame.carry??null,frame:still?0:frameAt(loopOf(CHARACTER_FRAMES[frame.pose],POSE_FRAME_MS[frame.pose]),elapsed),frameCount:CHARACTER_FRAMES[frame.pose],frameMs:POSE_FRAME_MS[frame.pose],paused:still};
}
export function actorSprite(key: string, frame: HallFrame, elapsed: number, still: boolean): Bitmap {
  const animation = actorAnimation(frame,elapsed,still).frame;
  return character({ role: "agent", look: lookFor(key), pose: frame.pose, facing: frame.facing, carry: frame.carry, frame: animation });
}
export function attentionGlyph(behavior: SceneBehavior) {
  return behavior === "blocked" ? "question" : behavior === "request-error" ? "alert" : behavior === "error" ? "fail" : behavior === "sleeping" ? "zzz" : behavior === "unknown" ? "question" : behavior === "working" || behavior === "launching" ? "thinking" : "readyCup";
}
