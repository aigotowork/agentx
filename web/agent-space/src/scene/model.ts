/** @aiPurpose Responsive room/seat projection adapted from shared/hall/model.ts. See public/notices/migration-inventory.json. */
import { populationSeats, type HallLayoutMemory, type PopulationSnapshot } from "./population";
import { membershipKey, type AgentResource } from "../offline/model";
import type { HallNavigation } from "./navigation";
import type { Point } from "./geometry";
import type { PropId } from "./art/pixel/props";
import { studioDeskAt, studioFixtureArt, studioFootprint, studioImage, studioStoolAt } from "./studio-art";
export const isCabinetResource = (resource: AgentResource) => resource.ref.record === "stopped";
export const HALL_STATE_LABEL = { running: "工作中", idle: "等待消息", ready: "就绪", closed: "已停止", failed: "执行失败", unknown: "状态待核对" } as const;
export interface HallActor { key: string; resource: AgentResource; home: Point; rest: Point; parentKey?: string; navigation?: HallNavigation }
export interface HallOfficeRect extends Point { width: number; height: number }
export interface HallOffice {
  work: HallOfficeRect;
  lounge: HallOfficeRect;
  door: Point;
  fixtures: readonly { id: string; prop: PropId; at: Point }[];
}
export interface HallRoom { key: string; title: string; origin: Point; width: number; height: number; actors: HallActor[]; cabinet: { at: Point; resources: AgentResource[] }; office?: HallOffice }
export interface HallScene { art?: "studio"; human: Point; width: number; height: number; rooms: HallRoom[]; actors: HallActor[]; cabinetCount: number }

/** World dimensions yield readable office furniture and names at the host's .75 near view. */
export const HALL_OFFICE_LAYOUT = {
  zoom: .75, multipleRoomsAt: 900, sideLoungeAt: 760,
  edge: 12, wideEdge: 24, roomGap: 32, firstRoomY: 128,
  inset: 32, header: 64, columnPitch: 120, rowPitch: 120, maximumColumns: 6,
  loungeWidth: 320, loungeGap: 16, loungeHeader: 56, restPitch: 112,
  cabinetFoot: 48, cabinetInset: 80, cabinetBottom: 48, closedHeight: 192,
} as const;

/** Keep the original responsive office projection with fixture-local identities and readable rest-seat pitch. */
export function projectHall(resources: readonly AgentResource[], workspaceNames: ReadonlyMap<string, string>, paneWidth: number, memory: HallLayoutMemory = new Map(), snapshot: PopulationSnapshot = { revision: 0, complete: true }, scope = "demo", art?: HallScene["art"]): HallScene {
  const unique = [...new Map(resources.map(resource => [resource.id, resource])).values()];
  const ordered = [...unique].sort((a, b) => a.createdAt.localeCompare(b.createdAt) || a.id.localeCompare(b.id));
  const groups = new Map<string, AgentResource[]>([...workspaceNames.keys()].map(key => [key, []]));
  for (const resource of ordered) { const key = membershipKey(resource); groups.set(key, [...(groups.get(key) ?? []), resource]); }
  const layout = HALL_OFFICE_LAYOUT;
  const edge = paneWidth >= layout.multipleRoomsAt ? layout.wideEdge : layout.edge;
  const width = Math.max(192, (paneWidth - edge * 2) / layout.zoom);
  const rooms = officeRooms(groups, workspaceNames, width, memory, snapshot, scope, art === "studio");
  return { art, human: { x: 80, y: art === "studio" ? 72 : 84 }, width, height: Math.max(192, ...rooms.map(room => room.origin.y + room.height + layout.firstRoomY)), rooms, actors: rooms.flatMap(room => room.actors), cabinetCount: unique.filter(isCabinetResource).length };
}

/** The backdrop and the moving people consume one world projection; furniture never grants an action. */
function officeRooms(groups: ReadonlyMap<string, AgentResource[]>, names: ReadonlyMap<string, string>, worldWidth: number, memory: HallLayoutMemory, snapshot: PopulationSnapshot, scope: string, studio: boolean): HallRoom[] {
  const layout = HALL_OFFICE_LAYOUT;
  let shelfX = 0, shelfY = studio ? 112 : layout.firstRoomY, shelfHeight = 0;
  return [...groups].sort(([a], [b]) => a.localeCompare(b)).map(([key, entries]) => {
    const active = entries.filter(resource => !isCabinetResource(resource)), cabinet = entries.filter(isCabinetResource);
    const population = populationSeats(memory, JSON.stringify([scope, key]), active.map(resource => resource.id), snapshot);
    const capacity = population.capacity;
    const preferredColumns = capacity <= 2 ? 1 : Math.min(4, capacity / 2);
    const preferredWidth = layout.inset * 2 + preferredColumns * (layout.columnPitch + layout.restPitch) + layout.loungeGap;
    const side = capacity > 0 && preferredWidth <= worldWidth;
    const columns = side ? preferredColumns : Math.max(1, Math.min(preferredColumns, Math.floor((worldWidth - layout.inset * 2) / layout.columnPitch)));
    const rows = Math.ceil(capacity / columns);
    const workWidth = columns * layout.columnPitch;
    const loungeWidth = side ? columns * layout.restPitch : workWidth;
    const roomWidth = capacity ? layout.inset * 2 + workWidth + (side ? layout.loungeGap + loungeWidth : 0) : 192;
    const clusterColumns = Math.max(1, Math.floor(loungeWidth / 160));
    const clusterRows = Math.ceil(Math.ceil(capacity / 4) / clusterColumns);
    const workHeight = rows * layout.rowPitch, loungeHeight = capacity ? Math.max(layout.loungeHeader + rows * layout.restPitch, 230 + (clusterRows - 1) * 190) : 0;
    const height = capacity ? layout.header + (side ? Math.max(workHeight, loungeHeight) : workHeight + layout.loungeGap + loungeHeight) + layout.cabinetFoot : 160;
    if (shelfX && shelfX + roomWidth > worldWidth) { shelfX = 0; shelfY += shelfHeight + layout.roomGap; shelfHeight = 0; }
    const origin = { x: shelfX, y: shelfY };
    shelfX += roomWidth + layout.roomGap; shelfHeight = Math.max(shelfHeight, height);
    const work = { x: origin.x + layout.inset, y: origin.y + layout.header, width: capacity ? workWidth : roomWidth - layout.inset * 2, height: workHeight };
    const lounge = { x: side ? work.x + workWidth + layout.loungeGap : work.x, y: side ? work.y : work.y + workHeight + layout.loungeGap, width: capacity ? loungeWidth : 0, height: loungeHeight };
    const actors = active.map((resource): HallActor => {
      const slot = population.slots.get(resource.id) ?? 0, column = slot % columns, row = Math.floor(slot / columns);
      return { key: resource.id, resource,
        home: { x: work.x + work.width / 2 + (column - (columns - 1) / 2) * layout.columnPitch, y: work.y + layout.rowPitch / 2 + row * layout.rowPitch },
        rest: loungePoint(lounge, slot, clusterColumns),
      };
    });
    const fixtures: HallOffice["fixtures"] = [
      ...(capacity ? [
        { id: "coffee", prop: "coffee" as const, at: { x: lounge.x + lounge.width - (studio ? 31 : 16), y: lounge.y + (studio ? 58 : 24) } },
        ...(columns >= 3 ? [{ id: "sofa", prop: "sofa" as const, at: { x: lounge.x + (studio ? 64 : 32), y: lounge.y + (studio ? 57 : 24) } }] : []),
      ] : [{ id: "bookshelf", prop: "bookshelf" as const, at: { x: origin.x + 64, y: origin.y + (studio ? 116 : 96) } }]),
      ...(studio ? [{ id: "plant", prop: "plant" as const, at: { x: origin.x + roomWidth - 28, y: origin.y + 69 } }] : []),
    ];
    const door = { x: origin.x + 64, y: origin.y + height };
    const cabinetAt = { x: origin.x + roomWidth - 48, y: origin.y + height - 24 };
    const bounds = { x: origin.x + 16, y: origin.y + 40, width: roomWidth - 32, height: height - 52 };
    const solidProps = studio ? [
      ...fixtures.flatMap(fixture => {
        const art = studioFixtureArt(fixture.prop, fixture.at);
        return art ? [studioFootprint(art, 26)] : [];
      }),
      ...(cabinet.length ? [studioFootprint(studioImage("cabinet", cabinetAt), 32)] : []),
    ] : [];
    for (const actor of actors) actor.navigation = { bounds, door, obstacles: studio ? [
      ...actors.flatMap(person => [
        studioFootprint(studioImage("desk", studioDeskAt(person.home)), 29),
        studioFootprint(studioImage("stool", studioStoolAt(person.home)), 12),
      ]),
      ...solidProps,
    ] : [
      ...actors.map(person => ({ x: person.home.x - 44, y: person.home.y - 74, width: 88, height: person.key === actor.key ? 66 : 98 })),
      ...fixtures.map(fixture => ({ x: fixture.at.x - (fixture.prop === "sofa" ? 56 : 28), y: fixture.at.y - 64, width: fixture.prop === "sofa" ? 112 : 56, height: 76 })),
    ] };
    return { key, title: `${names.get(key) ?? key} · ${active.length}${snapshot.complete ? " 成员" : "+ 已加载"}`, origin, width: roomWidth, height, actors,
      cabinet: { at: cabinetAt, resources: cabinet },
      office: { work, lounge, door, fixtures },
    };
  });
}

/** Four standing places make an open conversation group; spare capacity never adds chairs. */
function loungePoint(lounge: HallOfficeRect, slot: number, columns: number): Point {
  const group = Math.floor(slot / 4), column = group % columns, row = Math.floor(group / columns);
  const offsets = [{ x: -32, y: 0 }, { x: 34, y: 16 }, { x: -24, y: 82 }, { x: 42, y: 100 }];
  const offset = offsets[slot % 4]!;
  const mirror = group % 2 ? -1 : 1;
  return { x: lounge.x + 56 + column * 160 + offset.x * mirror, y: lounge.y + 90 + row * 190 + offset.y };
}
