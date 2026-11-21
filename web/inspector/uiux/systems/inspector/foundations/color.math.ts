/** @aiPurpose Compute color contrast consistently for semantic theme resolution and palette consumers. */
function luminance(color: string): number | null {
  const short = /^#([a-f\d])([a-f\d])([a-f\d])$/i.exec(color);
  const hex = short
    ? short
        .slice(1)
        .map((part) => part + part)
        .join("")
    : /^#([a-f\d]{6})(?:[a-f\d]{2})?$/i.exec(color)?.[1];
  if (!hex) return null;
  const [r, g, b] = [0, 2, 4].map((offset) => {
    const channel = parseInt(hex.slice(offset, offset + 2), 16) / 255;
    return channel <= 0.04045
      ? channel / 12.92
      : ((channel + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * r! + 0.7152 * g! + 0.0722 * b!;
}
/** WCAG contrast ratio; unknown color formats count as readable. */
export function contrast(a: string, b: string): number {
  const x = luminance(a),
    y = luminance(b);
  return x === null || y === null
    ? 21
    : (Math.max(x, y) + 0.05) / (Math.min(x, y) + 0.05);
}
