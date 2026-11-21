/** @aiPurpose Register the product's canonical light Space resources in the installed UIUX workbench. */
import type { ReactNode } from "react";
import type { SystemAdapter } from "@longwork/uiux";
import { SpaceInteractions, SpaceTheme } from "./systems/space/space.web";
import "./systems/space/styles/index.css";
import "../src/scene/studio.css";
function Provider({ children }: { children: ReactNode }) { return <SpaceTheme><SpaceInteractions>{children}</SpaceInteractions></SpaceTheme>; }
function Controls() { return <span>Space · 浅色</span>; }
function TokenEditor() { return <span>变量源：styles/foundation.css</span>; }
/** @aiPurpose Register the complete Space board and its canonical resources.
 * @aiUsage The current supported theme is the product light theme; scenes never query native services.
 */
export const systems: SystemAdapter[] = [{ id: "space", title: "AgentX Space", Provider, Controls, TokenEditor, composition: { resource: "space/whole-space", scene: "default" }, canvas: [{ resource: "space/whole-space", scene: "default", wide: true }, { resource: "space/conversation-composer", scene: "default" }, { resource: "space/scene-controls", scene: "default" }] }];
