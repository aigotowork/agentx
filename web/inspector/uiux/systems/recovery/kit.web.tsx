/** @aiPurpose Provide shared semantic Web primitives without recovery state or effects. */
import {
  useId,
  useRef,
  type ButtonHTMLAttributes,
  type ReactNode,
} from "react";
import { Icon, type IconName } from "./icons.web";
import { useHint } from "./overlay.web";
export { Icon, type IconName } from "./icons.web";
export { OverlayBoundary, Modal } from "./overlay.web";
export type Tone =
  | "neutral"
  | "accent"
  | "warning"
  | "danger"
  | "unknown"
  | "info";
interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  icon?: IconName;
  variant?: "primary" | "secondary" | "quiet" | "danger";
  busy?: boolean;
  children: ReactNode;
}
/**
 * @aiPurpose Render a labeled action with shared variants and busy state.
 * @aiUsage Use the canonical Button primitive; domain data and write effects remain with its consumer.
 */
/** @aiPurpose Compose Button presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function Button({
  icon,
  variant = "secondary",
  busy = false,
  children,
  className = "",
  disabled,
  ...props
}: ButtonProps) {
  return (
    <button
      type="button"
      className={`button ${variant} ${className}`}
      disabled={disabled || busy}
      aria-busy={busy || undefined}
      {...props}
    >
      {icon && <Icon name={icon} />}
      <span>{children}</span>
      {busy && <span className="busy-pixel" aria-hidden="true" />}
    </button>
  );
}
/**
 * @aiPurpose Render a named compact action with shared pointer, keyboard and touch hints.
 * @aiUsage Use the canonical IconButton primitive; domain data and write effects remain with its consumer.
 */
/** @aiPurpose Compose IconButton presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function IconButton({
  label,
  icon,
  active,
  ...props
}: Omit<ButtonHTMLAttributes<HTMLButtonElement>, "children"> & {
  label: string;
  icon: IconName;
  active?: boolean;
}) {
  const h = useHint(label);
  return (
    <button
      type="button"
      {...props}
      {...h.bindings}
      ref={h.ref}
      className={
        "icon-button " + (active ? "active " : "") + (props.className || "")
      }
      aria-label={label}
      aria-pressed={active}
    >
      <Icon name={icon} />
    </button>
  );
}
/**
 * @aiPurpose Pair status text with a semantic tone and optional state glyph.
 * @aiUsage Use the canonical Badge primitive; domain data and write effects remain with its consumer.
 */
/** @aiPurpose Compose Badge presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function Badge({
  children,
  tone = "neutral",
  icon,
}: {
  children: ReactNode;
  tone?: Tone;
  icon?: IconName;
}) {
  return (
    <span className={"badge " + tone}>
      {icon ? <Icon name={icon} /> : <span className="state-dot" />}
      {children}
    </span>
  );
}
/**
 * @aiPurpose Render a controlled tab selection with keyboard navigation.
 * @aiUsage Use the canonical Tabs primitive; domain data and write effects remain with its consumer.
 */
/** @aiPurpose Compose Tabs presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function Tabs<T extends string>({
  label,
  items,
  value,
  onChange,
}: {
  label: string;
  items: readonly { id: T; label: string; count?: number }[];
  value: T;
  onChange(value: T): void;
}) {
  const group = useId(),
    refs = useRef(new Map<T, HTMLButtonElement>());
  return (
    <div className="tabs" role="tablist" aria-label={label}>
      {items.map((item, index) => (
        <button
          key={item.id}
          id={group + item.id}
          role="tab"
          className={"tab " + (value === item.id ? "selected" : "")}
          aria-selected={value === item.id}
          tabIndex={value === item.id ? 0 : -1}
          ref={(e) => {
            if (e) refs.current.set(item.id, e);
          }}
          onClick={() => onChange(item.id)}
          onKeyDown={(e) => {
            if (!["ArrowLeft", "ArrowRight", "Home", "End"].includes(e.key))
              return;
            e.preventDefault();
            const i =
              e.key === "Home"
                ? 0
                : e.key === "End"
                  ? items.length - 1
                  : (index + (e.key === "ArrowRight" ? 1 : items.length - 1)) %
                    items.length;
            onChange(items[i].id);
            refs.current.get(items[i].id)?.focus();
          }}
        >
          {item.label}
          {item.count !== undefined && (
            <span className="tab-count">{item.count}</span>
          )}
        </button>
      ))}
    </div>
  );
}
/**
 * @aiPurpose Associate one fact label with its value in a definition list.
 * @aiUsage Use the canonical Field primitive; domain data and write effects remain with its consumer.
 */
/** @aiPurpose Compose Field presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function Field({
  label,
  children,
}: {
  label: string;
  children: ReactNode;
}) {
  return (
    <div className="fact">
      <dt>{label}</dt>
      <dd>{children}</dd>
    </div>
  );
}
/**
 * @aiPurpose Present the current collection state and an optional next action.
 * @aiUsage Use the canonical Empty primitive; domain data and write effects remain with its consumer.
 */
/** @aiPurpose Compose Empty presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function Empty({
  icon = "queue",
  title,
  action,
}: {
  icon?: IconName;
  title: string;
  action?: ReactNode;
}) {
  return (
    <div className="empty">
      <Icon name={icon} />
      <p>{title}</p>
      {action}
    </div>
  );
}
/**
 * @aiPurpose Expose a selectable technical identity and copy action.
 * @aiUsage Use the canonical CopyValue primitive; domain data and write effects remain with its consumer.
 */
/** @aiPurpose Compose CopyValue presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function CopyValue({
  value,
  label = "复制 ID",
}: {
  value: string;
  label?: string;
}) {
  const ref = useRef<HTMLSpanElement>(null);
  return (
    <span className="copy-value">
      <code ref={ref}>{value}</code>
      <IconButton
        icon="copy"
        label={label}
        onClick={async () => {
          try {
            if (navigator.clipboard?.writeText)
              await navigator.clipboard.writeText(value);
            else {
              const range = document.createRange();
              range.selectNodeContents(ref.current!);
              const select = window.getSelection();
              select?.removeAllRanges();
              select?.addRange(range);
              document.execCommand("copy");
            }
          } catch {
            const range = document.createRange();
            if (ref.current) {
              range.selectNodeContents(ref.current);
              window.getSelection()?.addRange(range);
            }
          }
        }}
      />
    </span>
  );
}
