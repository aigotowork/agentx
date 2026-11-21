/** @aiPurpose Keep Web host icons and portals subscribed to the current presentation skin. */
import { createContext, useContext } from "react";
import type { VisualStyle } from "../../themes/skin.native";

/** @aiPurpose Compose VisualStyleContext presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export const VisualStyleContext = createContext<VisualStyle>("original");
/** @aiPurpose Subscribe host widgets to the active visual skin. @aiUsage Read inside React components, including portal children. */
/** @aiPurpose Provide useVisualStyle presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export const useVisualStyle = () => useContext(VisualStyleContext);
