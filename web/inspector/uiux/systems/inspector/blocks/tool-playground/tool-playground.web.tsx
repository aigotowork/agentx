/** @aiPurpose Exercise shared tool feedback with local material state; it never sends product commands. */
import {
  useEffect,
  useRef,
  useState,
  type CSSProperties,
  type KeyboardEvent,
  type MouseEvent,
} from "react";
import { pixelMotion } from "../../foundations/foundation.tokens";
import type { PluginTheme } from "../../../../../host/contracts";
import { Button, ChoiceGroup } from "../../components/kit/kit.native";
import type { PixelFeedback } from "../../motion/feedback.web";
import { toolImage, type ToolArtName } from "../../assets/tools/tools.assets";
import "./tool-playground.styles.css";
const names = { hammer: "敲定锤", shovel: "探查铲", chisel: "细修凿" } as const;
type Impact = { id: number; x: number; y: number; tool: ToolArtName };
type PointStyle = CSSProperties & {
  "--impact-x": string;
  "--impact-y": string;
};
/** @aiPurpose Review original tool artwork and local material feedback. @aiUsage Supply host theme and explicit feedback preferences; never use it to signal business success. */
/** @aiPurpose Compose ToolPlayground presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function ToolPlayground({
  theme,
  feedback,
}: {
  theme: PluginTheme;
  feedback: PixelFeedback;
}) {
  const [tool, setTool] = useState<ToolArtName>("hammer");
  const [removed, setRemoved] = useState<ReadonlySet<number>>(() => new Set());
  const [impacts, setImpacts] = useState<Impact[]>([]);
  const [taps, setTaps] = useState(0);
  const sequence = useRef(0);
  const pad = useRef<HTMLDivElement>(null);
  const moving =
    feedback.pixel && feedback.preferences.motion && !feedback.systemReduced;
  useEffect(() => {
    if (!moving) setImpacts([]);
  }, [moving]);
  // Hidden boards can cancel CSS animationend; bound the lifetime of decorative nodes anyway.
  useEffect(() => {
    if (!impacts.length) return;
    const timer = setTimeout(() => setImpacts([]), pixelMotion.strike + pixelMotion.debris);
    return () => clearTimeout(timer);
  }, [impacts]);
  function hit(index?: number) {
    const cell =
      index ??
      Array.from({ length: 18 }, (_, value) => value).find(
        (value) => !removed.has(value),
      );
    if (cell === undefined || removed.has(cell)) return;
    setRemoved((current) => new Set([...current, cell]));
    setTaps((current) => current + 1);
    const target = pad.current?.querySelector(`[data-material="${cell}"]`);
    if (moving && target && pad.current) {
      const a = target.getBoundingClientRect(),
        b = pad.current.getBoundingClientRect();
      const impact = {
        id: ++sequence.current,
        x: a.x + a.width / 2 - b.x,
        y: a.y + a.height / 2 - b.y,
        tool,
      };
      setImpacts((current) => [...current.slice(-7), impact]);
    }
    void feedback.previewSound();
  }
  function pointer(event: MouseEvent<HTMLDivElement>) {
    if (event.button !== 0) return;
    const element = (event.target as Element).closest<HTMLElement>(
      "[data-material]",
    );
    hit(element ? Number(element.dataset.material) : undefined);
  }
  function keyboard(event: KeyboardEvent<HTMLDivElement>) {
    if (!["Enter", " "].includes(event.key)) return;
    event.preventDefault();
    if (!event.repeat) hit();
  }
  return (
    <div className="ds-tool-playground" data-feedback="silent">
      <ChoiceGroup
        theme={theme}
        label="试敲工具"
        layout="wrap"
        value={tool}
        onChange={setTool}
        options={Object.entries(names).map(([value, label]) => ({
          value: value as ToolArtName,
          label,
        }))}
      />
      <div
        className="ds-tool-pad"
        ref={pad}
        data-tool={tool}
        role="button"
        tabIndex={0}
        aria-label="工具试敲台，点击或按空格敲落本地方块"
        onClick={pointer}
        onKeyDown={keyboard}
      >
        <div className="ds-material" aria-hidden="true">
          {Array.from({ length: 18 }, (_, index) => (
            <span
              key={index}
              data-material={index}
              className="ds-material-cell"
              data-removed={removed.has(index)}
            />
          ))}
        </div>
        <span className="ds-tool-hint">斜握 → 锤头向下 → 方块飞散</span>
        {impacts.map((impact) => (
          <span
            key={impact.id}
            className="ds-tool-impact"
            aria-hidden="true"
            style={
              {
                "--impact-x": `${impact.x}px`,
                "--impact-y": `${impact.y}px`,
              } as PointStyle
            }
            onAnimationEnd={(event) => {
              if (event.target === event.currentTarget)
                setImpacts((current) =>
                  current.filter((value) => value.id !== impact.id),
                );
            }}
          >
            <img
              className="ds-striking-tool"
              src={toolImage(impact.tool)}
              alt=""
            />
            {[0, 1, 2, 3, 4].map((value) => (
              <i key={value} className="ds-tool-debris" />
            ))}
          </span>
        ))}
      </div>
      <div className="ds-tool-footer">
        <output aria-live="polite">
          {names[tool]} · {taps} 次 · 已敲落 {removed.size} / 18
        </output>
        <Button
          theme={theme}
          label="补回方块"
          size="sm"
          onPress={() => {
            setRemoved(new Set());
            setImpacts([]);
          }}
        />
      </div>
    </div>
  );
}
