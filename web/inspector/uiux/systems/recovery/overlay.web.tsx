/** @aiPurpose Own viewport hints, modal focus, touch intent and background suspension. */
import {
  createContext,
  useContext,
  useEffect,
  useId,
  useLayoutEffect,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { createPortal } from "react-dom";
type Hint = { owner: string; text: string; anchor: HTMLElement };
interface OverlayContext {
  show(hint: Hint): void;
  hide(owner?: string): void;
  suspend(): () => void;
}
const Context = createContext<OverlayContext | null>(null);

/**
 * @aiPurpose Own viewport tooltip positioning and modal suspension.
 * @aiUsage Mount once around the Web root; hints and dialogs use its shared viewport boundary.
 */
/** @aiPurpose Compose OverlayBoundary presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function OverlayBoundary({ children }: { children: ReactNode }) {
  const [hint, setHint] = useState<Hint | null>(null),
    [point, setPoint] = useState<{ x: number; y: number } | null>(null);
  const node = useRef<HTMLDivElement>(null),
    suspended = useRef(0);
  const api = useRef<OverlayContext>({
    show: (h) => {
      if (!suspended.current || h.anchor.closest('[role="dialog"]')) {
        setPoint(null);
        setHint(h);
      }
    },
    hide: (owner) => setHint((h) => (!owner || h?.owner === owner ? null : h)),
    suspend: () => {
      suspended.current++;
      setHint(null);
      return () => {
        suspended.current = Math.max(0, suspended.current - 1);
      };
    },
  }).current;
  useEffect(() => {
    const dismiss = () => api.hide();
    const key = (e: KeyboardEvent) => {
      if (e.key === "Escape") api.hide();
    };
    window.addEventListener("scroll", dismiss, true);
    window.addEventListener("resize", dismiss);
    document.addEventListener("keydown", key);
    document.addEventListener("visibilitychange", dismiss);
    return () => {
      window.removeEventListener("scroll", dismiss, true);
      window.removeEventListener("resize", dismiss);
      document.removeEventListener("keydown", key);
      document.removeEventListener("visibilitychange", dismiss);
    };
  }, [api]);
  useLayoutEffect(() => {
    if (!hint || !node.current) return;
    const a = hint.anchor.getBoundingClientRect(),
      r = node.current.getBoundingClientRect(),
      s = getComputedStyle(document.documentElement),
      gap = parseFloat(s.getPropertyValue("--hint-gap")),
      m = parseFloat(s.getPropertyValue("--hint-margin"));
    if (!hint.anchor.isConnected || a.bottom <= 0 || a.top >= innerHeight) {
      api.hide(hint.owner);
      return;
    }
    setPoint({
      x: Math.max(
        m,
        Math.min(a.x + a.width / 2 - r.width / 2, innerWidth - r.width - m),
      ),
      y:
        a.bottom + gap + r.height > innerHeight - m
          ? Math.max(m, a.top - r.height - gap)
          : a.bottom + gap,
    });
  }, [hint, api]);
  return (
    <Context.Provider value={api}>
      {children}
      {hint &&
        createPortal(
          <div
            ref={node}
            id="inspector-hint"
            role="tooltip"
            className={"hint " + (!point ? "hint-measuring" : "")}
            style={point ? { left: point.x, top: point.y } : undefined}
          >
            {hint.text}
          </div>,
          document.body,
        )}
    </Context.Provider>
  );
}
/**
 * @aiPurpose Bind one control gesture to the shared hint session.
 * @aiUsage Bind directly to the existing named control; never add button-local floating content.
 */
/** @aiPurpose Provide useHint presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function useHint(text?: string) {
  const context = useContext(Context),
    owner = useId(),
    ref = useRef<HTMLButtonElement>(null),
    timer = useRef<ReturnType<typeof setTimeout> | null>(null),
    consumed = useRef(false),
    suppressed = useRef(false);
  const hide = () => {
    if (timer.current) clearTimeout(timer.current);
    context?.hide(owner);
  };
  useEffect(
    () => () => {
      if (timer.current) clearTimeout(timer.current);
      context?.hide(owner);
    },
    [context, owner],
  );
  const show = () => {
    if (text && ref.current && !suppressed.current)
      context?.show({ owner, text, anchor: ref.current });
  };
  return {
    ref,
    bindings: {
      onPointerEnter: (e: React.PointerEvent) => {
        if (e.pointerType !== "touch") {
          suppressed.current = false;
          timer.current = setTimeout(show, 350);
        }
      },
      onPointerLeave: () => {
        hide();
        suppressed.current = false;
      },
      onFocus: show,
      onBlur: () => {
        hide();
        suppressed.current = false;
      },
      onPointerDown: (e: React.PointerEvent) => {
        hide();
        consumed.current = false;
        if (e.pointerType === "touch") {
          suppressed.current = false;
          timer.current = setTimeout(() => {
            show();
            consumed.current = true;
          }, 500);
        }
      },
      onPointerUp: () => {
        if (timer.current) clearTimeout(timer.current);
      },
      onPointerCancel: () => {
        consumed.current = false;
        hide();
      },
      onClickCapture: (e: React.MouseEvent) => {
        if (consumed.current) {
          e.preventDefault();
          e.stopPropagation();
          consumed.current = false;
        } else {
          hide();
          suppressed.current = true;
        }
      },
    },
  };
}
/**
 * @aiPurpose Keep confirmation content focused and restore its caller on close.
 * @aiUsage Render a local dialog; busy forbids closing until a command receipt is classified.
 */
/** @aiPurpose Compose Modal presentation.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export function Modal({
  title,
  children,
  footer,
  onClose,
  busy = false,
  labelId,
}: {
  title: string;
  children: ReactNode;
  footer?: ReactNode;
  onClose(): void;
  busy?: boolean;
  labelId?: string;
}) {
  const ref = useRef<HTMLElement>(null),
    heading = useRef<HTMLHeadingElement>(null),
    context = useContext(Context),
    id = useId();
  const closeRef = useRef(onClose);
  closeRef.current = onClose;
  const busyRef = useRef(busy);
  busyRef.current = busy;
  useEffect(() => {
    const previous = document.activeElement as HTMLElement | null,
      root = document.getElementById("root");
    const release = context?.suspend();
    if (root) root.inert = true;
    heading.current?.focus();
    const key = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopPropagation();
        if (!busyRef.current) closeRef.current();
      }
      if (e.key === "Tab") {
        const items = [
          ...(ref.current?.querySelectorAll<HTMLElement>(
            'button:not(:disabled),input:not(:disabled),select:not(:disabled),textarea:not(:disabled),[tabindex="0"]',
          ) || []),
        ].filter((x) => x.getClientRects().length);
        const first = items[0],
          last = items.at(-1);
        if (!items.length) {
          e.preventDefault();
          return;
        }
        if (
          e.shiftKey &&
          (document.activeElement === first ||
            document.activeElement === heading.current)
        ) {
          e.preventDefault();
          last?.focus();
        } else if (
          !e.shiftKey &&
          (document.activeElement === last ||
            document.activeElement === heading.current)
        ) {
          e.preventDefault();
          first?.focus();
        }
      }
    };
    document.addEventListener("keydown", key, true);
    return () => {
      document.removeEventListener("keydown", key, true);
      if (root) root.inert = false;
      release?.();
      if (previous?.isConnected) previous.focus();
    };
  }, [context]);
  return createPortal(
    <div
      className="modal-backdrop"
      onPointerDown={(e) => {
        if (e.target === e.currentTarget && !busy) onClose();
      }}
    >
      <section
        className="modal"
        ref={ref}
        role="dialog"
        aria-modal="true"
        aria-labelledby={labelId || id}
      >
        <header className="modal-heading">
          <h2 ref={heading} id={labelId || id} tabIndex={-1}>
            {title}
          </h2>
          <button
            className="icon-button"
            aria-label="关闭"
            disabled={busy}
            onClick={onClose}
          >
            <svg className="icon" viewBox="0 0 16 16" aria-hidden="true">
              <path
                fill="currentColor"
                d="M3 2 8 7l5-5 1 1-5 5 5 5-1 1-5-5-5 5-1-1 5-5-5-5Z"
              />
            </svg>
          </button>
        </header>
        <div className="modal-body">{children}</div>
        {footer && <footer className="modal-footer">{footer}</footer>}
      </section>
    </div>,
    document.body,
  );
}
