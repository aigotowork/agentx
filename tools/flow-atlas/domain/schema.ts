/** @aiPurpose Check a bounded inert JSON Schema subset without invoking native parse/refinement hooks. */
import type { Diagnostic, FieldPath, Json, ValueSchema } from './contracts.ts';

type Shape = Readonly<Record<string, Json>>;
export type Report = (code: string, path: readonly (string | number)[], message: string, severity?: Diagnostic['severity']) => void;
const structural = new Set(['$schema', '$id', '$defs', '$ref', 'type', 'properties', 'required', 'additionalProperties', 'items', 'prefixItems', 'minItems', 'maxItems', 'minimum', 'maximum', 'exclusiveMinimum', 'exclusiveMaximum', 'multipleOf', 'minLength', 'maxLength', 'pattern', 'enum', 'const', 'anyOf', 'oneOf', 'allOf', 'not', 'title', 'description', 'examples', 'default', 'deprecated', 'readOnly', 'writeOnly']);
const record = (value: unknown): value is Shape => !!value && typeof value === 'object' && !Array.isArray(value);
export const same = (a: unknown, b: unknown): boolean => {
  if (a === b) return true;
  if (Array.isArray(a) && Array.isArray(b)) return a.length === b.length && a.every((value, i) => same(value, b[i]));
  if (record(a) && record(b)) { const keys = Object.keys(a); return keys.length === Object.keys(b).length && keys.every(key => Object.hasOwn(b, key) && same(a[key], b[key])); }
  return false;
};
export function read(value: unknown, path: FieldPath): unknown {
  let cursor = value;
  for (const key of path) {
    if (cursor === null || typeof cursor !== 'object' || !Object.hasOwn(cursor, key)) return undefined;
    cursor = (cursor as Record<string | number, unknown>)[key];
  }
  return cursor;
}
export function pathKey(path: FieldPath): string { return JSON.stringify(path); }
export function prefix(parent: FieldPath, child: FieldPath): boolean { return parent.length <= child.length && parent.every((key, i) => key === child[i]); }
export function resolve(shape: Shape, root: Shape, seen = new Set<string>()): Shape | undefined {
  if (shape.$ref === undefined) return shape;
  if (typeof shape.$ref !== 'string' || !shape.$ref.startsWith('#/') || seen.has(shape.$ref)) return undefined;
  const next = read(root, shape.$ref.slice(2).split('/').map(key => key.replace(/~1/g, '/').replace(/~0/g, '~')));
  if (!record(next)) return undefined;
  return resolve(next, root, new Set([...seen, shape.$ref]));
}
export function schemaAt(schema: ValueSchema | undefined, path: FieldPath): Shape | undefined {
  if (!schema) return undefined;
  const root = schema.jsonSchema;
  let current: Shape | undefined = root;
  for (const key of path) {
    if (!current) return undefined;
    current = resolve(current, root);
    if (!current) return undefined;
    const property = typeof key === 'number' ? (Array.isArray(current.prefixItems) ? current.prefixItems[key] : current.items) : record(current.properties) ? current.properties[key] : undefined;
    if (!record(property)) return undefined;
    current = property;
  }
  return current && resolve(current, root);
}
export function schemaCoverage(schema: ValueSchema, report: Report): void {
  const root = schema.jsonSchema;
  const seen = new Set<unknown>();
  function visit(shape: unknown, path: FieldPath): void {
    if (typeof shape === 'boolean') return;
    if (!record(shape) || seen.has(shape)) return;
    seen.add(shape);
    for (const key of Object.keys(shape)) if (!structural.has(key)) report('UNSUPPORTED_SCHEMA_KEYWORD', ['schemas', schema.id, ...path, key], `Keyword ${key} is exported but not checked.`, 'warning');
    if (shape.$ref && !resolve(shape, root)) report('UNSUPPORTED_SCHEMA_REFERENCE', ['schemas', schema.id, ...path], 'Only finite local JSON Schema references are checked.', 'warning');
    if (shape.not !== undefined && !(record(shape.not) && Object.keys(shape.not).length === 0)) report('UNSUPPORTED_SCHEMA_KEYWORD', ['schemas', schema.id, ...path, 'not'], 'Only the never-schema form of not is checked.', 'warning');
    for (const key of ['properties', '$defs'] as const) if (record(shape[key])) for (const [name, child] of Object.entries(shape[key])) visit(child, [...path, key, name]);
    for (const key of ['anyOf', 'oneOf', 'allOf', 'prefixItems'] as const) if (Array.isArray(shape[key])) shape[key].forEach((child, i) => visit(child, [...path, key, i]));
    if (shape.items) visit(shape.items, [...path, 'items']);
    if (record(shape.additionalProperties)) visit(shape.additionalProperties, [...path, 'additionalProperties']);
  }
  visit(root, []);
}
export function valueErrors(value: unknown, schema: ValueSchema | undefined): readonly string[] {
  if (!schema) return ['Schema was not converted.'];
  const root = schema.jsonSchema;
  function check(item: unknown, raw: Shape, path: string, depth: number): string[] {
    if (depth > 64) return [`${path}: schema depth exceeds supported bound.`];
    const shape = resolve(raw, root);
    if (!shape) return []; // Coverage separately marks this as unsupported, never as checked proof.
    const errors: string[] = [];
    if (record(shape.not) && Object.keys(shape.not).length === 0) return [`${path}: value is forbidden.`];
    if (Array.isArray(shape.anyOf) && !shape.anyOf.some(branch => record(branch) && check(item, branch, path, depth + 1).length === 0)) errors.push(`${path}: no anyOf branch matches.`);
    if (Array.isArray(shape.oneOf) && shape.oneOf.filter(branch => record(branch) && check(item, branch, path, depth + 1).length === 0).length !== 1) errors.push(`${path}: expected exactly one matching branch.`);
    if (Array.isArray(shape.allOf)) for (const branch of shape.allOf) if (record(branch)) errors.push(...check(item, branch, path, depth + 1));
    if (Object.hasOwn(shape, 'const') && !same(item, shape.const)) errors.push(`${path}: constant differs.`);
    if (Array.isArray(shape.enum) && !shape.enum.some(entry => same(item, entry))) errors.push(`${path}: value is outside the enum.`);
    const actual = item === null ? 'null' : Array.isArray(item) ? 'array' : typeof item;
    const types = typeof shape.type === 'string' ? [shape.type] : Array.isArray(shape.type) ? shape.type : [];
    if (types.length && !types.includes(actual) && !(actual === 'number' && Number.isInteger(item) && types.includes('integer'))) errors.push(`${path}: expected ${types.join('|')}, received ${actual}.`);
    if (typeof item === 'number') {
      for (const [key, bad] of [ ['minimum', (n: number) => item < n], ['maximum', (n: number) => item > n], ['exclusiveMinimum', (n: number) => item <= n], ['exclusiveMaximum', (n: number) => item >= n] ] as const) if (typeof shape[key] === 'number' && bad(shape[key])) errors.push(`${path}: violates ${key}.`);
      if (typeof shape.multipleOf === 'number' && Math.abs(item / shape.multipleOf - Math.round(item / shape.multipleOf)) > 1e-9) errors.push(`${path}: violates multipleOf.`);
    }
    if (typeof item === 'string') {
      if (typeof shape.minLength === 'number' && [...item].length < shape.minLength) errors.push(`${path}: too short.`);
      if (typeof shape.maxLength === 'number' && [...item].length > shape.maxLength) errors.push(`${path}: too long.`);
      if (typeof shape.pattern === 'string') { try { if (!new RegExp(shape.pattern, 'u').test(item)) errors.push(`${path}: pattern mismatch.`); } catch { errors.push(`${path}: unsupported pattern.`); } }
    }
    if (Array.isArray(item)) {
      if (typeof shape.minItems === 'number' && item.length < shape.minItems) errors.push(`${path}: too few items.`);
      if (typeof shape.maxItems === 'number' && item.length > shape.maxItems) errors.push(`${path}: too many items.`);
      item.forEach((child, i) => { const element = Array.isArray(shape.prefixItems) && i < shape.prefixItems.length ? shape.prefixItems[i] : shape.items; if (record(element)) errors.push(...check(child, element, `${path}[${i}]`, depth + 1)); else if (element === false) errors.push(`${path}[${i}]: extra item.`); });
    } else if (record(item)) {
      const properties = record(shape.properties) ? shape.properties : {};
      if (Array.isArray(shape.required)) for (const key of shape.required) if (typeof key === 'string' && !Object.hasOwn(item, key)) errors.push(`${path}.${key}: required field missing.`);
      for (const [key, child] of Object.entries(item)) {
        const field = properties[key];
        if (record(field)) errors.push(...check(child, field, `${path}.${key}`, depth + 1));
        else if (shape.additionalProperties === false) errors.push(`${path}.${key}: unknown field.`);
        else if (record(shape.additionalProperties)) errors.push(...check(child, shape.additionalProperties, `${path}.${key}`, depth + 1));
      }
    }
    return errors;
  }
  return check(value, root, '$', 0);
}
export function requiredPaths(schema: ValueSchema | undefined): readonly FieldPath[] {
  if (!schema) return [];
  function walk(raw: Shape, path: FieldPath, depth: number): FieldPath[] {
    const shape = resolve(raw, schema!.jsonSchema);
    if (!shape || depth > 32) return [path];
    if (shape.type !== 'object' || !Array.isArray(shape.required) || !record(shape.properties)) return [path];
    return shape.required.flatMap(key => typeof key === 'string' && record((shape.properties as Shape)[key]) ? walk((shape.properties as Shape)[key] as Shape, [...path, key], depth + 1) : []);
  }
  return walk(schema.jsonSchema, [], 0);
}
/** Conservative compatibility: unknown/complex schemas explicitly remain unproved. */
export function compatible(source: Shape, target: Shape, depth = 0): 'yes' | 'no' | 'unknown' {
  if (same(source, target)) return 'yes';
  if (depth > 32 || source.$ref || target.$ref || ['anyOf', 'oneOf', 'allOf', 'not'].some(key => source[key] || target[key])) return 'unknown';
  const sourceType = source.type; const targetType = target.type;
  if (typeof sourceType !== 'string' || typeof targetType !== 'string') return 'unknown';
  if (sourceType !== targetType && !(sourceType === 'integer' && targetType === 'number')) return 'no';
  if (Object.hasOwn(target, 'const') && !same(source.const, target.const)) return 'no';
  if (Array.isArray(target.enum)) {
    const candidates = Array.isArray(source.enum) ? source.enum : Object.hasOwn(source, 'const') ? [source.const] : undefined;
    if (!candidates || !candidates.every(value => (target.enum as readonly Json[]).some(other => same(value, other)))) return 'no';
  }
  for (const key of ['minimum', 'exclusiveMinimum', 'minLength', 'minItems'] as const) if (typeof target[key] === 'number' && (typeof source[key] !== 'number' || source[key] < target[key])) return 'no';
  for (const key of ['maximum', 'exclusiveMaximum', 'maxLength', 'maxItems'] as const) if (typeof target[key] === 'number' && (typeof source[key] !== 'number' || source[key] > target[key])) return 'no';
  if (target.pattern && !same(source.pattern, target.pattern) || target.multipleOf && !same(source.multipleOf, target.multipleOf)) return 'unknown';
  if (sourceType === 'object') {
    if (!record(target.properties) || !record(source.properties)) return 'unknown';
    const targetProperties = target.properties;
    const sourceProperties = source.properties;
    const targetRequired = Array.isArray(target.required) ? target.required : [];
    const sourceRequired = Array.isArray(source.required) ? source.required : [];
    let unknown = false;
    for (const key of targetRequired) {
      if (typeof key !== 'string' || !sourceRequired.includes(key)) return 'no';
      const a = sourceProperties[key]; const b = targetProperties[key];
      if (!record(a) || !record(b)) return 'unknown';
      const result = compatible(a, b, depth + 1);
      if (result === 'no') return 'no';
      unknown ||= result === 'unknown';
    }
    if (target.additionalProperties === false && Object.keys(sourceProperties).some(key => !Object.hasOwn(targetProperties, key))) return 'no';
    return unknown ? 'unknown' : 'yes';
  }
  if (sourceType === 'array') return record(source.items) && record(target.items) ? compatible(source.items, target.items, depth + 1) : 'unknown';
  return 'yes';
}
