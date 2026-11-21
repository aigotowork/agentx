/** Retain a declared real-UI checkpoint; callers own scenario/effect decisions. */
import { mkdir, writeFile } from "node:fs/promises";
import { join } from "node:path";

/**
 * Capture actual DOM/ARIA/focus/geometry and viewport pixels with native state.
 * This reads no React internals; the explicit gap is retained in the sidecar.
 */
export async function checkpoint(page, directory, id, meaning, nativeState = null) {
  const target = join(directory, id);
  await mkdir(target, { recursive: false });
  const started = new Date().toISOString();
  const semantic = await page.locator("body").ariaSnapshot();
  const dom = await page.evaluate(() => {
    const bounds = element => {
      const box = element.getBoundingClientRect();
      return { x: box.x, y: box.y, width: box.width, height: box.height };
    };
    const visible = element => {
      const box = element.getBoundingClientRect();
      const style = getComputedStyle(element);
      return box.width > 0 && box.height > 0 && style.visibility !== "hidden" && style.display !== "none";
    };
    return {
      url: location.href, viewport: { width: innerWidth, height: innerHeight, dpr: devicePixelRatio },
      secureContext: isSecureContext, randomUUID: typeof crypto.randomUUID,
      focus: document.activeElement?.outerHTML ?? null,
      regions: [...document.querySelectorAll("main,nav,header,section,dialog,[role=tabpanel],[role=status],[role=alert]")]
        .filter(visible).map(element => ({ tag: element.tagName, role: element.getAttribute("role"),
          class: element.className, text: element.innerText, bounds: bounds(element) })),
      controls: [...document.querySelectorAll("button,input,textarea,select,summary,[role=tab]")]
        .filter(visible).map(element => ({ tag: element.tagName, role: element.getAttribute("role"),
          name: element.getAttribute("aria-label") ?? element.innerText ?? "", disabled: element.disabled,
          ariaDisabled: element.getAttribute("aria-disabled"), bounds: bounds(element) })),
      selectedRows: [...document.querySelectorAll(".work-row.selected")].map(element => element.innerText),
      pendingReference: sessionStorage.getItem("agentx-workbench-pending-v1"),
    };
  });
  await writeFile(join(target, "semantic.yml"), semantic);
  await writeFile(join(target, "dom.json"), `${JSON.stringify(dom, null, 2)}\n`);
  await page.screenshot({ path: join(target, "viewport.png"), fullPage: false });
  const record = { id, meaning, started, finished: new Date().toISOString(), url: page.url(),
    viewport: page.viewportSize(), nativeState,
    artifacts: { semantic: "semantic.yml", stateAndGeometry: "dom.json", render: "viewport.png" },
    limits: "Sequential captures from one actual page; not atomic. No React/private hook state or physical touchscreen/assistive-technology observation. User-intent image assessment is recorded separately." };
  await writeFile(join(target, "checkpoint.json"), `${JSON.stringify(record, (_key, value) => typeof value === "bigint" ? { $integer: value.toString() } : value, 2)}\n`);
  return record;
}
