/** @aiPurpose Exercise canonical reading-window disclosure, draft continuity and return. */
import { useState } from "react";
import { Button } from "./space.web";
import { ExpandedObject } from "../../../src/scene/ExpandedObject";
function Window() {
  const [visible, setVisible] = useState(false), [expanded, setExpanded] = useState(false), [draft, setDraft] = useState('关闭窗口后仍保留');
  return <div className="space-preview"><Button onClick={() => setVisible(true)}>打开对象阅读窗</Button><ExpandedObject title="UIUX 对象阅读窗" kind="conversation" point={{ x: 100, y: 100 }} viewport={{ width: 360, height: 680 }} visible={visible} expanded={expanded} onExpand={() => setExpanded(value => !value)} onClose={() => setVisible(false)}><label>保留的草稿<textarea value={draft} onChange={event => setDraft(event.target.value)} /></label></ExpandedObject></div>;
}
/** @aiPurpose Bind the actual object-window consumer scene.
 * @aiUsage Escape, close and reopen share the product overlay owner and retain draft identity.
 */
export const scenes = { default: Window };
