/** @aiPurpose Own live theme overrides while preserving all story and form identities. */
import {
  createContext,
  useContext,
  useLayoutEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import {
  resolveDesignSystem,
  type SystemAppearance,
  type SystemOverrides,
  resolveInspectorTokens,
} from "../../themes/theme.resolve";
import type {TokenValues} from '../../../../framework/framework.public';
import { VisualStyleContext } from "./appearance.context";
import { usePixelFeedback } from "../../motion/feedback.web";
import { setHostTheme } from "../host.web";
import { OverlayBoundary } from "../../components/overlay/overlay.native";
import { useCanvasOverrides } from "./canvas.preferences";
function useSystem() {
  const [appearance, setAppearance] = useState<SystemAppearance>({
    style: "pixel",
    theme: "light",
  });
  const [overrides, setOverrides] = useState<SystemOverrides>({});
  const [tokens,setTokens]=useState<TokenValues>({});
  const canvas = useCanvasOverrides();
  const resolved = useMemo(
    () => resolveDesignSystem(appearance, { ...canvas, ...(Object.keys(tokens).length?resolveInspectorTokens(tokens,appearance.style+'.'+appearance.theme):{}), ...overrides }),
    [appearance, overrides, canvas,tokens],
  );
  const feedback = usePixelFeedback(appearance.style, resolved);
  useLayoutEffect(() => {
    const root = document.documentElement;
    root.style.colorScheme = appearance.theme;
    root.dataset.experience = "inspector";
    root.dataset.style = appearance.style;
    root.dataset.theme = appearance.theme;
    root.dataset.systemTokens = "true";
    for (const [name, value] of Object.entries(resolved.variables))
      root.style.setProperty(name, value);
    setHostTheme(resolved.theme);
  }, [appearance, resolved]);
  return {
    appearance,
    setAppearance,
    overrides,
    setOverrides,
    tokens,setTokens,
    resolved,
    theme: resolved.theme,
    feedback,
  };
}
const Context = createContext<ReturnType<typeof useSystem> | null>(null);
/** @aiPurpose Own the live design-system context. @aiUsage Mount once around all stories without a style key. */
/** @aiPurpose Compose SystemProvider presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function SystemProvider({ children }: { children: ReactNode }) {
  const value = useSystem();
  return (
    <Context.Provider value={value}>
      <VisualStyleContext.Provider value={value.appearance.style}>
        <OverlayBoundary theme={value.theme} scopeKey="design-studio">
          {children}
        </OverlayBoundary>
      </VisualStyleContext.Provider>
    </Context.Provider>
  );
}
/** @aiPurpose Access the shared theme and local editing controls. @aiUsage Use only inside SystemProvider. */
/** @aiPurpose Provide useDesignSystem presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function useDesignSystem() {
  const value = useContext(Context);
  if (!value) throw new Error("DesignSystemProvider is required");
  return value;
}
