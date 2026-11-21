/** @aiPurpose Adapt saved Canvas tweak CSS to the same semantic resolver used by React controls. */
import { useLayoutEffect, useState } from "react";
import type { SystemOverrides } from "../../themes/theme.resolve";
const bindings = {
  press: "--press-ms",
  pressDepth: "--press-depth",
  rhythm: "--running-ms",
  chart: "--reveal-ms",
} as const;
/** @aiPurpose Keep native Canvas tweaks effective without creating a second component theme. @aiUsage Local variable-panel edits override these defaults. */
export function useCanvasOverrides(): SystemOverrides {
  const [values, setValues] = useState<SystemOverrides>({});
  useLayoutEffect(() => {
    function sync() {
      const css = getComputedStyle(document.documentElement),
        next: SystemOverrides = {};
      for (const [role, variable] of Object.entries(bindings)) {
        const number = Number.parseFloat(css.getPropertyValue(variable));
        if (Number.isFinite(number))
          next[role as keyof typeof bindings] = number;
      }
      setValues((current) =>
        JSON.stringify(current) === JSON.stringify(next) ? current : next,
      );
    }
    sync();
    const observer = new MutationObserver(sync);
    observer.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ["style"],
    });
    observer.observe(document.head, {
      subtree: true,
      childList: true,
      characterData: true,
    });
    document.addEventListener("load", sync, true);
    return () => {
      observer.disconnect();
      document.removeEventListener("load", sync, true);
    };
  }, []);
  return values;
}
