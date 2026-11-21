import type { SpaceStyle } from "./presentation";
import { SPACE_STYLE_GROUPS, parseSpaceStyle } from "./presentation";

/** @aiPurpose Offer the full set of appearances without wrapping the toolbar or reader.
 * @aiUsage Pass the selected style and update it through onChange. The browser handles touch and keyboard selection.
 */
export function SpaceStyleSwitch({ value, onChange, compact = false }: {
  value: SpaceStyle;
  onChange(value: SpaceStyle): void;
  compact?: boolean;
}) {
  function selectStyle(value: string) {
    const style = parseSpaceStyle(value);
    if (style) onChange(style);
  }
  return <label className="space-style-switch" data-compact={compact || undefined}>
    <span>风格</span>
    <select aria-label="界面风格" value={value} onChange={event => selectStyle(event.currentTarget.value)}>
      {SPACE_STYLE_GROUPS.map(group => <optgroup key={group.id} label={group.label}>
        {group.styles.map(style => <option key={style.value} value={style.value}>{style.label}</option>)}
      </optgroup>)}
    </select>
  </label>;
}
