/** @aiPurpose Reject lossy JSON conversion, custom serialization and executable object members. */
import type { Json } from './contracts.ts';

/** @aiPurpose Copy inert JSON while refusing values JSON.stringify would silently alter or execute.
 * @aiUsage Use at artifact boundaries; native schemas and handlers must never be passed here.
 */
export function portableCopy(value: unknown): Json {
  const active = new Set<object>();
  function copy(item: unknown, path: string): Json {
    if (item === null || typeof item === 'string' || typeof item === 'boolean') return item;
    if (typeof item === 'number' && Number.isFinite(item)) return item;
    if (typeof item !== 'object' || item === null) throw new Error(`Nonportable value at ${path}`);
    if (active.has(item)) throw new Error(`Cyclic value at ${path}`);
    const proto = Object.getPrototypeOf(item);
    if (!Array.isArray(item) && proto !== Object.prototype && proto !== null) throw new Error(`Nonplain object at ${path}`);
    if (Object.getOwnPropertySymbols(item).length) throw new Error(`Symbol member at ${path}`);
    active.add(item);
    try {
      if (Array.isArray(item)) {
        const result: Json[] = [];
        for (let i = 0; i < item.length; i++) {
          const descriptor = Object.getOwnPropertyDescriptor(item, String(i));
          if (!descriptor || !('value' in descriptor)) throw new Error(`Sparse/accessor array at ${path}[${i}]`);
          result.push(copy(descriptor.value, `${path}[${i}]`));
        }
        if (Object.keys(item).some(key => !/^(0|[1-9][0-9]*)$/.test(key))) throw new Error(`Extra array member at ${path}`);
        return result;
      }
      const result: Record<string, Json> = Object.create(null);
      for (const key of Object.getOwnPropertyNames(item)) {
        const descriptor = Object.getOwnPropertyDescriptor(item, key)!;
        if (!descriptor.enumerable || !('value' in descriptor)) throw new Error(`Hidden/accessor member at ${path}.${key}`);
        result[key] = copy(descriptor.value, `${path}.${key}`);
      }
      return result;
    } finally { active.delete(item); }
  }
  return copy(value, '$');
}
