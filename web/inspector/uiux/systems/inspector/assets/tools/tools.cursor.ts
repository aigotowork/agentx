/** @aiPurpose Select the same original SVG tools for browser cursors and the component playground. */
import type { ToolCursor } from "../../motion/feedback.preferences";
import { toolArt } from "./tools.assets";
/** @aiPurpose Express pointer feedback without altering hit testing. @aiUsage Use only for explicit fine-pointer preferences. */
export function toolCursor(tool: ToolCursor, pressed = false): string {
  if (tool === "system") return "auto";
  const source = toolArt[tool];
  const svg = pressed
    ? source
        .replace(
          /(<svg[^>]*>)/,
          '$1<g transform="translate(0 2) rotate(-12 20 20)">',
        )
        .replace("</svg>", "</g></svg>")
    : source;
  return `url("data:image/svg+xml,${encodeURIComponent(svg)}") 8 10, pointer`;
}
