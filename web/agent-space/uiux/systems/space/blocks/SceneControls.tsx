/** @aiPurpose Expose camera, animation and roster actions without owning world geometry or time. */
import { Button } from "../components/Controls";
interface Props { paused: boolean; directory: boolean; onZoom(factor: number): void; onPanorama(): void; onPause(): void; onDirectory(): void }
/** @aiPurpose Keep scene controls in one accessible compact toolbar.
 * @aiUsage Route callbacks to the existing SceneSpace camera/state owner; pausing affects decorative motion only.
 */
export function SceneControls({ paused, directory, onZoom, onPanorama, onPause, onDirectory }: Props) {
  return <nav className="office-camera-controls" aria-label="办公室镜头"><Button aria-label="缩小大厅" onClick={() => onZoom(.8)}>−</Button><Button aria-label="放大大厅" onClick={() => onZoom(1.25)}>＋</Button><Button onClick={onPanorama}>全景</Button><Button aria-pressed={paused} onClick={onPause}>{paused ? "播放动作" : "暂停动作"}</Button><Button aria-expanded={directory} onClick={onDirectory}>名册</Button></nav>;
}
