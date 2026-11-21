/** @aiPurpose Share copy actions and explicit result feedback for messages and code blocks. */
import { useRef, useState } from "react";
import { copyText } from "./clipboard";

/**
 * @aiPurpose Copy the complete supplied text without changing the composer or message expansion.
 * @aiUsage Use for message or code actions with an accessible label; failures remain visible beside the action.
 */
export function CopyButton({ text, label = "复制" }: { text: string; label?: string }) {
  const [result, setResult] = useState<{ text: string; status: "pending" | "copied" | "failed" } | null>(null);
  const inFlight = useRef(false);
  const status = result?.text === text ? result.status : "idle";
  async function copy() {
    if (inFlight.current) return;
    inFlight.current = true; setResult({ text, status: "pending" });
    try { await copyText(text); setResult({ text, status: "copied" }); }
    catch { setResult({ text, status: "failed" }); }
    finally { inFlight.current = false; }
  }
  return <span className="copy-action" data-copy-state={status}><button type="button" className="text-button" aria-live="polite" disabled={status === "pending"} onClick={() => void copy()}>{status === "copied" ? "已复制" : status === "pending" ? "复制中…" : label}</button><small className="copy-feedback" role="status">{status === "failed" ? "复制失败，请选中文本后手动复制。" : ""}</small></span>;
}
