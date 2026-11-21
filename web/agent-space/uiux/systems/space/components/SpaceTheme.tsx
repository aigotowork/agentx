/** @aiPurpose Apply the supported Space theme without recoloring its host workbench or changing child identity. */
import { createContext, type ReactNode } from 'react';
import { spaceGameWorld, spaceWorld, type SpaceStyle, type SpaceWorld } from "./presentation";

/** @aiPurpose Share the active artwork identity with the scene and its portraits.
 * @aiUsage Read inside SpaceTheme; an undefined value selects the baseline Studio set.
 */
export const SpaceWorldContext = createContext<SpaceWorld | undefined>(undefined);
/** @aiPurpose Scope canonical foundation tokens and presentation styles to a complete Space consumer.
 * @aiUsage Wrap the consumer once and load styles/index.css. An optional candidate changes tokens without remounting children.
 */
export function SpaceTheme({ children, style = "classic" }: { children: ReactNode; style?: SpaceStyle }) {
  const world = spaceWorld(style);
  const skin = style === "classic" || style === "studio" ? undefined : style;
  return <SpaceWorldContext.Provider value={world}><div className="space-system" data-space-style={skin} data-space-world={world} data-space-game={spaceGameWorld(world)}><div className="space-theme-content">{children}</div></div></SpaceWorldContext.Provider>;
}
