/** Page-local capacities and exact-member seats. Native collection completeness owns shrink admission. */
export interface PopulationSnapshot { revision: number; complete: boolean }
interface RoomPopulation { capacity: number; slots: Map<string, number>; revision: number; lowCount: number; lowStreak: number }
export type HallLayoutMemory = Map<string, RoomPopulation>;
export interface PopulationSeats { capacity: number; slots: ReadonlyMap<string, number> }
export function populationCapacity(count: number): number {
  return [0, 2, 4, 6, 8, 12, 16].find(capacity => capacity >= count) ?? Math.ceil(count / 4) * 4;
}
/** Preserve member seats across status, reorder and filters; shrink requires two complete snapshots. */
export function populationSeats(memory: HallLayoutMemory, key: string, members: readonly string[], snapshot: PopulationSnapshot): PopulationSeats {
  let saved = memory.get(key);
  if (!saved) { saved = { capacity: populationCapacity(members.length), slots: new Map(), revision: -1, lowCount: -1, lowStreak: 0 }; memory.set(key, saved); }
  const current = new Set(members);
  if (snapshot.complete) for (const member of saved.slots.keys()) if (!current.has(member)) saved.slots.delete(member);
  if (saved.revision !== snapshot.revision) {
    saved.revision = snapshot.revision;
    const low = snapshot.complete && members.length <= saved.capacity / 2 && populationCapacity(members.length) < saved.capacity;
    saved.lowStreak = low ? saved.lowCount === members.length ? saved.lowStreak + 1 : 1 : 0;
    saved.lowCount = low ? members.length : -1;
    if (saved.lowStreak >= 2) {
      saved.capacity = populationCapacity(members.length);
      const ordered = [...saved.slots].sort((a, b) => a[1] - b[1]);
      saved.slots = new Map(ordered.map(([id], index) => [id, index])); saved.lowStreak = 0;
    }
  }
  const used = new Set(saved.slots.values());
  for (const id of members) if (!saved.slots.has(id)) {
    let slot = 0; while (used.has(slot)) slot++; saved.slots.set(id, slot); used.add(slot);
  }
  const needed = Math.max(members.length, 0, ...[...saved.slots.values()].map(slot => slot + 1));
  if (needed > saved.capacity) saved.capacity = populationCapacity(needed);
  return saved;
}
