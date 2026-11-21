/** @aiPurpose Operate canonical controls, passive facts and a controlled scene toolbar. */
import { useState } from "react";
import type { SceneProps } from "@longwork/uiux";
import { Button, FactButton, Feedback, SceneControls } from "./space.web";
function Controls({ emit }: SceneProps) {
  const [count, setCount] = useState(0);
  return <div className="space-preview-tools"><Button variant="primary" onClick={() => { setCount(value => value + 1); emit('press'); }}>主动作</Button><Button disabled>不可用</Button><FactButton facts={<><strong>精确实例事实</strong><span>焦点与触摸入口复用产品事实提示</span></>} onClick={() => setCount(value => value + 1)}>本地事实 ⓘ</FactButton><Feedback>本地动作 {count} 次</Feedback></div>;
}
function Toolbar() {
  const [paused, setPaused] = useState(false), [directory, setDirectory] = useState(false), [zoom, setZoom] = useState(1);
  return <div className="space-preview" data-short="true"><Feedback>UIUX 本地镜头 {zoom.toFixed(2)} · {paused ? '暂停' : '播放'} · {directory ? '名册打开' : '名册关闭'}</Feedback><SceneControls paused={paused} directory={directory} onZoom={factor => setZoom(value => value * factor)} onPanorama={() => setZoom(1)} onPause={() => setPaused(value => !value)} onDirectory={() => setDirectory(value => !value)} /></div>;
}
/** @aiPurpose Expose matching control and toolbar scenes.
 * @aiUsage All callbacks affect only visible local state.
 */
export const scenes = { default: Controls, toolbar: Toolbar };
