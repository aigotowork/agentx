/** @aiPurpose Show the shared Space conversation with deterministic, read-only supply. */
import { useState, useSyncExternalStore } from "react";
import type { SceneProps } from "@longwork/uiux";
import { SpaceTheme, type SpaceStyle } from "./space.web";
import { Conversation } from "../../../src/conversation/Conversation";
import { SpaceController } from "../../../src/offline/controller";
import { createOfflineSupply } from "../../../src/offline/fixture";
import { createStudioSupply } from "../../../src/offline/studio";
import type { SuppliedRecords } from "../../../src/offline/model";

function Reader({ args, empty = false, style }: SceneProps & { empty?: boolean; style?: SpaceStyle }) {
  const [controller] = useState(() => new SpaceController(style ? createStudioSupply() : createOfflineSupply()));
  const state = useSyncExternalStore(controller.subscribe, controller.getSnapshot);
  const [open, setOpen] = useState(true);
  const { supply } = controller;
  const agent = supply.agents.find(item => item.name === (style ? "mora" : "luna") && item.ref.record === "current")!;
  const channel = state.channels[agent.id] ?? "messages";
  const records: SuppliedRecords = args.status === "error" ? { status: "error", reason: "示例读取失败" }
    : args.status === "unavailable" ? { status: "unavailable", reason: "演示未提供记录" }
      : empty ? { status: "available", provenance: "本地演示", items: [] } : supply.records(agent.id, channel);
  const reader = open ? <Conversation agent={agent} records={records} controller={controller} channel={channel} onChannel={value => controller.channel(agent.id, value)} draft={state.drafts[agent.id] ?? ""} onDraft={value => controller.writeDraft(agent.id, value)} onBack={() => setOpen(false)} studio={!!style} backLabel={style ? "返回办公室" : undefined} />
    : <div className="space-preview-tools"><button type="button" onClick={() => setOpen(true)}>打开 {agent.title} 会话</button></div>;
  const surface = <div className="space-preview" data-space-art={style ? "studio" : undefined} data-narrow={args.narrow === true}>{reader}</div>;
  return style ? <SpaceTheme style={style}>{surface}</SpaceTheme> : surface;
}
function Empty(props: SceneProps) { return <Reader {...props} empty />; }
function Folio(props: SceneProps) { return <Reader {...props} style="folio" />; }
function Signal(props: SceneProps) { return <Reader {...props} style="signal" />; }

/** @aiPurpose Bind populated, empty and styled scenes to the same Space reader.
 * @aiUsage Drafts stay editable in the demo without creating messages or native receipts.
 */
export const scenes = { default: Reader, empty: Empty, folio: Folio, signal: Signal };
