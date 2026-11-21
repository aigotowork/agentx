/** @aiPurpose Own Web icon, modal, settings and toast presentation independently of host transport. */
import {useCallback,useEffect,useId,useState,type ReactNode} from 'react';
import {createPortal} from 'react-dom';
import * as Lucide from 'lucide-react';
import {ScrollView} from 'react-native';
import type {PluginTheme} from '../../../../host/contracts';
import {useVisualStyle} from './web/appearance.context';
import {pixelIconPaths} from '../assets/icons/icons.paths';
import '../assets/icons/icons.motion.css';
let currentTheme:PluginTheme;
/** @aiPurpose Provide setHostTheme presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function setHostTheme(theme:PluginTheme){currentTheme=theme;}
/** @aiPurpose Compose Icon presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function Icon({
  name,
  size = 16,
  color = "currentColor",
}: {
  name: string;
  size?: number;
  color?: string;
}) {
  const style = useVisualStyle();
  const mosaicId = useId().replace(/\W/g, "");
  const pixel = style === "pixel" ? pixelIconPaths[name] : undefined;
  if (pixel) return <svg className="inspector-pixel-icon" data-icon={name} width={size} height={size} viewBox="0 0 16 16" fill={color} aria-hidden="true" focusable="false"><path d={pixel} /><defs>{[[0,0,8,8],[8,4,8,6],[4,10,8,6]].map(([x,y,width,height],index) => <clipPath key={index} id={`${mosaicId}-${index}`}><rect x={x} y={y} width={width} height={height} /></clipPath>)}</defs>{[0,1,2].map(index => <g key={index} className={`inspector-mosaic-tile inspector-mosaic-${index}`} clipPath={`url(#${mosaicId}-${index})`}><path d={pixel} /></g>)}</svg>;
  const Glyph = (Lucide as unknown as Record<string, Lucide.LucideIcon>)[name];
  return Glyph ? (
    <Glyph size={size} color={color} strokeWidth={1.75} aria-hidden="true" />
  ) : null;
}
/** @aiPurpose Compose Modal presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function Modal({
  title,
  open,
  onOpenChange,
  children,
}: {
  title: string;
  open: boolean;
  onOpenChange(open: boolean): void;
  children: ReactNode;
}) {
  useEffect(() => {
    if (!open) return;
    const key = (event: KeyboardEvent) => {
      if (event.key === "Escape") onOpenChange(false);
    };
    document.addEventListener("keydown", key);
    return () => document.removeEventListener("keydown", key);
  }, [open, onOpenChange]);
  if (!open) return null;
  return createPortal(
    <div
      className="host-modal-backdrop"
      onClick={(event) => {
        if (event.target === event.currentTarget) onOpenChange(false);
      }}
    >
      <div
        className="host-modal"
        role="dialog"
        aria-modal="true"
        aria-label={title}
      >
        <div className="host-modal-heading">
          <span>{title}</span>
          <button
            className="host-icon"
            aria-label="关闭"
            onClick={() => onOpenChange(false)}
          >
            <Icon name="X" color={currentTheme.colors.foregroundMuted} />
          </button>
        </div>
        {children}
      </div>
    </div>,
    document.body,
  );
}
Modal.Content = function Content({
  children,
  contentContainerStyle,
}: {
  children: ReactNode;
  contentContainerStyle?: object;
  scrollable?: boolean;
}) {
  return (
    <ScrollView
      style={{ flexShrink: 1 }}
      contentContainerStyle={[{ padding: 24, gap: 16 }, contentContainerStyle]}
    >
      {children}
    </ScrollView>
  );
};
let sequence = 0;
let toasts: Array<{ id: number; message: string }> = [];
const toastListeners = new Set<() => void>();
function toast(message: string, _options?: {variant?:string;[key:string]:unknown}) {
  message = `模拟 · ${message}`;
  const id = ++sequence;
  toasts = [...toasts.slice(-2), { id, message }];
  for (const listener of toastListeners) listener();
  setTimeout(() => {
    toasts = toasts.filter((item) => item.id !== id);
    for (const listener of toastListeners) listener();
  }, 6000);
}
const toastApi = { show: toast, error: toast };
/** @aiPurpose Provide useToast presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function useToast() {
  return toastApi;
}
/** @aiPurpose Compose ToastHost presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function ToastHost() {
  const [, setRevision] = useState(0);
  useEffect(() => {
    const listener = () => setRevision((value) => value + 1);
    toastListeners.add(listener);
    return () => {
      toastListeners.delete(listener);
    };
  }, []);
  return (
    <div className="host-toasts" role="status" aria-live="polite">
      {toasts.map((item) => (
        <div key={item.id}>{item.message}</div>
      ))}
    </div>
  );
}
/** @aiPurpose Compose SettingsSection presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function SettingsSection({
  title,
  children,
}: {
  title: string;
  children: ReactNode;
}) {
  return (
    <section className="host-settings">
      <h3>{title}</h3>
      {children}
    </section>
  );
}
/** @aiPurpose Compose SettingsCard presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function SettingsCard({ children }: { children: ReactNode }) {
  return <div className="host-settings-card">{children}</div>;
}
/** @aiPurpose Compose SettingsRow presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function SettingsRow({
  label,
  hint,
  error,
  children,
}: {
  label: string;
  hint?: string;
  error?: string | null;
  children?: ReactNode;
}) {
  return (
    <div className="host-settings-row">
      <div>{label}</div>
      {hint && <small>{hint}</small>}
      {children}
      {error && <div role="alert">{error}</div>}
    </div>
  );
}
/** @aiPurpose Compose SettingsSelect presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function SettingsSelect({
  label,
  hint,
  value,
  options,
  onValueChange,
  disabled,
  error,
}: {
  label: string;
  hint?: string;
  value: string;
  options: Array<{ value: string; label: string }>;
  onValueChange(value: string): void;
  disabled?: boolean;
  error?: string | null;
}) {
  return (
    <SettingsRow label={label} hint={hint} error={error}>
      <select
        aria-label={label}
        value={value}
        onChange={(event) => onValueChange(event.target.value)}
        disabled={disabled}
      >
        {options.map((option) => (
          <option key={option.value} value={option.value}>
            {option.label}
          </option>
        ))}
      </select>
    </SettingsRow>
  );
}
/** @aiPurpose Compose SettingsSwitch presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function SettingsSwitch({
  label,
  hint,
  value,
  onValueChange,
  disabled,
  error,
}: {
  label: string;
  hint?: string;
  value: boolean;
  onValueChange(value: boolean): void;
  disabled?: boolean;
  error?: string | null;
}) {
  return (
    <SettingsRow label={label} hint={hint} error={error}>
      <input
        type="checkbox"
        role="switch"
        aria-label={label}
        checked={value}
        disabled={disabled}
        onChange={(event) => onValueChange(event.target.checked)}
      />
    </SettingsRow>
  );
}
/** @aiPurpose Compose SettingsAction presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function SettingsAction({
  label,
  actionLabel,
  onPress,
}: {
  label: string;
  actionLabel: string;
  onPress(): void;
}) {
  return (
    <SettingsRow label={label}>
      <button onClick={onPress}>{actionLabel}</button>
    </SettingsRow>
  );
}
/** @aiPurpose Compose Loading presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function Loading() {
  return <div role="progressbar" aria-label="Loading" />;
}
