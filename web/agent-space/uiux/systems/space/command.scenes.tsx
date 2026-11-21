import { useState } from "react";
import { Button, CommandSurface, SpaceTheme, type SpaceStyle } from "./space.web";

function Candidate({ style }: { style: SpaceStyle }) {
  const [opener, setOpener] = useState<HTMLElement | null>(null), [confirmed, setConfirmed] = useState(false);
  return <SpaceTheme style={style}><div className="space-preview space-command-preview">
    <div className="space-preview-tools"><Button onClick={event => { setOpener(event.currentTarget); setConfirmed(false); }}>终止进程</Button>{confirmed && <output>已确认 · 仅演示</output>}</div>
    {opener && <CommandSurface title="终止进程" opener={opener} danger submitLabel="确认终止" onClose={() => setOpener(null)} onSubmit={(event, close) => { event.preventDefault(); setConfirmed(true); close(); }}>
      <div className="command-identity"><strong>@mora</strong><small className="command-exact-ref">mora · 101</small><p>终止此实例的进程。历史记录会保留。</p></div>
    </CommandSurface>}
  </div></SpaceTheme>;
}
function Folio() { return <Candidate style="folio" />; }
function Signal() { return <Candidate style="signal" />; }

/** @aiPurpose Compare the actual command surface in the two Space candidates.
 * @aiUsage Operate cancel, keyboard return or local confirmation; no native command is available.
 */
export const scenes = { folio: Folio, signal: Signal };
