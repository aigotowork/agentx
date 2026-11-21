/** @aiPurpose Operate the actual Space composition using isolated local fixture state and the existing Canvas runtime. */
import { useRef, useState, useSyncExternalStore } from "react";
import type { SceneProps } from "@longwork/uiux";
import { OfficeLayout, SpaceLayout, ChatLayout, Button, SpaceTheme } from "./space.web";
import { SPACE_WORLDS, type SpaceWorld } from "./components/presentation";
import { createOfflineSupply } from "../../../src/offline/fixture";
import { createStudioSupply } from "../../../src/offline/studio";
import { SpaceController } from "../../../src/offline/controller";
import { Conversation } from "../../../src/conversation/Conversation";
import { OfficeNavigation, OfficeContext } from "../../../src/OfficeNavigation";
import { Directory } from "../../../src/Directory";
import { SceneSpace, type SceneMemory } from "../../../src/scene/SceneSpace";
import type { HallLayoutMemory } from "../../../src/scene/population";
import { officeMembers } from "../../../src/offline/projection";

function localController(world?: SpaceWorld) {
  if (world) {
    const controller = new SpaceController(createStudioSupply());
    controller.office('/studio/catalog');
    return controller;
  }
  const controller = new SpaceController(createOfflineSupply());
  controller.office('/projects/atlas/web');
  controller.select(controller.supply.agents[0].id);
  return controller;
}
/** @aiPurpose Show the complete office/reader relationship from real public resources and product adapters.
 * @aiUsage This scene uses explicit demo supply; camera and drafts reset only through its deliberate reset action.
 */
export function SpaceComposition({ args, world }: SceneProps & { world?: SpaceWorld }) {
  const studio = world !== undefined;
  const [controller, setController] = useState(() => localController(world));
  const [resetRevision, setResetRevision] = useState(0);
  const state = useSyncExternalStore(controller.subscribe, controller.getSnapshot), supply = controller.supply;
  const memory = useRef<SceneMemory | null>(null), layoutMemory = useRef<HallLayoutMemory>(new Map());
  const scoped = officeMembers(supply, state.workspaceId), selected = supply.agents.find(agent => agent.id === state.selectedId);
  const channel = selected ? state.channels[selected.id] ?? 'messages' : 'messages';
  const records = selected && supply.records(selected.id, channel);
  const reader = selected && records ? <Conversation studio={studio} backLabel={studio ? '返回办公室' : undefined} controller={controller} agent={selected} records={records} channel={channel} onChannel={value => controller.channel(selected.id, value)} draft={state.drafts[selected.id] ?? ''} onDraft={text => controller.writeDraft(selected.id, text)} onBack={controller.close} /> : <div className="reader-empty">选择一位 Agent</div>;
  const directory = <Directory studio={studio} agents={scoped} selectedId={state.selectedId} drafts={state.drafts} onSelect={controller.select} onReset={controller.clearFilters} />;
  const names = new Map(supply.workspaces.map(office => [office.id, office.title]));
  const scene = <SceneSpace studio={studio} studioSample={studio} key={resetRevision} memory={memory} layoutMemory={layoutMemory.current} identityScope={`${supply.scope.id}:${supply.scope.generation}`} agents={scoped} names={names} visibleKeys={new Set(scoped.map(agent => agent.id))} selectedId={state.selectedId} reading={state.reading} reader={reader} directory={directory} scope={state.workspaceId} onSelect={controller.select} onClose={controller.close} onCabinet={controller.historyDirectory} panel={state.panel} onPanel={controller.panel} onReset={controller.clearFilters} />;
  const content = state.presentation === 'scene' ? scene : <ChatLayout directory={directory} reader={reader} reading={state.reading} />;
  const header = <><strong>Agent 空间</strong><nav className="presentation-switch" aria-label="呈现方式"><Button aria-pressed={state.presentation === 'scene'} onClick={() => controller.presentation('scene')}>场景</Button><Button aria-pressed={state.presentation === 'chat'} onClick={() => controller.presentation('chat')}>聊天</Button></nav><Button onClick={() => { memory.current = null; layoutMemory.current = new Map(); setController(localController(world)); setResetRevision(value => value + 1); }}>重置演示</Button></>;
  return <div className="space-preview" data-narrow={args.narrow === true} data-short={args.short === true}><OfficeLayout studio={studio} studioSample={studio} studioReader={studio && state.reading ? state.presentation : undefined} header={header} context={<OfficeContext studio={studio} controller={controller} matchedCount={scoped.length} />}><SpaceLayout navigation={<OfficeNavigation studio={studio} controller={controller} />}>{content}</SpaceLayout></OfficeLayout></div>;
}
function worldScene(world: SpaceWorld) {
  return function OfficeWorld(props: SceneProps) {
    return <SpaceTheme style={world}><SpaceComposition {...props} world={world} /></SpaceTheme>;
  };
}
const worldScenes = Object.fromEntries(SPACE_WORLDS.map(({ value }) => [value, worldScene(value)]));
/** @aiPurpose Bind declared board scenes to the same complete Space composition.
 * @aiUsage The catalogue varies only the style. Scene controls resize and retain local work.
 */
export const scenes = { default: SpaceComposition, ...worldScenes };
