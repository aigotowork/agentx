/** @aiPurpose Adapt pinned native Zod schemas without coupling generic authoring modules to Zod. */
import * as z from 'zod';
import type { SchemaAdapter } from '../contracts.ts';
import { portableCopy } from '../portable.ts';

// Zod's default/catch/lazy conversion can call author callbacks. This deliberately small
// version-pinned preflight refuses them before toJSONSchema, including in nested shapes.
function preflight(root: z.ZodType): void {
  const seen = new Set<object>();
  const supported = new Set(['string', 'number', 'boolean', 'null', 'enum', 'literal', 'object', 'array', 'union', 'intersection', 'tuple', 'record', 'optional', 'nullable', 'nonoptional', 'readonly', 'never', 'any', 'unknown']);
  function visit(value: unknown): void {
    if (!value || typeof value !== 'object' || seen.has(value)) return;
    seen.add(value);
    if (value instanceof z.ZodType) {
      const definition = value._zod.def;
      if (!supported.has(definition.type)) throw new Error(`Unsupported semantic conversion: ${definition.type}.`);
      if (definition.checks?.some(check => check._zod.def.check === 'custom')) throw new Error('Custom refinements cannot be represented by JSON Schema.');
      for (const [key, descriptor] of Object.entries(Object.getOwnPropertyDescriptors(definition))) {
        if (!('value' in descriptor)) {
          // Zod 4.6.7 wraps every object shape in a memoizing getter and exposes its raw
          // shape on that getter. Inspect raw descriptors before the built-in converter reads it.
          const raw = key === 'shape' && definition.type === 'object' && descriptor.get
            ? Object.getOwnPropertyDescriptor(descriptor.get, 'raw') : undefined;
          if (raw && 'value' in raw && raw.value && typeof raw.value === 'object') { visit(raw.value); continue; }
          throw new Error(`Schema accessor is unsupported: ${key}.`);
        }
        if (key !== 'checks') visit(descriptor.value);
      }
    } else if (Array.isArray(value) || Object.getPrototypeOf(value) === Object.prototype || Object.getPrototypeOf(value) === null) {
      for (const [key, descriptor] of Object.entries(Object.getOwnPropertyDescriptors(value))) {
        if (!('value' in descriptor)) throw new Error(`Schema shape accessor is unsupported: ${key}.`);
        visit(descriptor.value);
      }
    }
  }
  visit(root);
}

/** @aiPurpose Export imported Zod 4.6.7 schemas as inert JSON Schema with explicit unsupported diagnostics.
 * @aiUsage Pass this adapter to schema; conversion does not parse input or execute methods/refinement predicates.
 * @aiBehavior Unsupported transformations, BigInt, Date and custom nodes fail instead of becoming permissive schemas.
 */
export const zodAdapter: SchemaAdapter = Object.freeze({
  convert(schema: unknown, context: { readonly id: string; readonly label?: string; readonly io: 'input' | 'output' }) {
    try {
      if (!(schema instanceof z.ZodType)) throw new Error('Expected a schema from the declared Zod 4.6.7 peer.');
      preflight(schema);
      const jsonSchema = z.toJSONSchema(schema, {
        target: 'draft-2020-12', io: context.io, unrepresentable: 'throw', cycles: 'ref', reused: 'ref',
        override({ zodSchema }) {
          // Input conversion can otherwise hide a pipe/transform. Reject it in both directions.
          if (['transform', 'pipe', 'custom'].includes(zodSchema._zod.def.type)) throw new Error(`Unsupported semantic conversion: ${zodSchema._zod.def.type}.`);
          if (zodSchema._zod.def.checks?.some(check => check._zod.def.check === 'custom')) throw new Error('Custom refinements cannot be represented by JSON Schema.');
        },
      });
      // Zod 4.6.7 finalize() first produces JSON, then attaches a nonenumerable,
      // readonly ~standard data property containing validator/converter functions.
      // Exclude only that library-owned root descriptor, without reading its value
      // or relaxing portableCopy for author metadata or nested JSON Schema fields.
      const descriptors: Record<string, PropertyDescriptor> = Object.getOwnPropertyDescriptors(jsonSchema);
      const standard = descriptors['~standard'];
      if (standard && 'value' in standard && !standard.enumerable && !standard.writable && !standard.configurable) delete descriptors['~standard'];
      const portableSchema = Object.create(Object.getPrototypeOf(jsonSchema), descriptors) as unknown;
      return {
        ok: true as const,
        value: { id: context.id, label: context.label ?? context.id, jsonSchema: portableCopy(portableSchema) as Readonly<Record<string, import('../contracts.ts').Json>> },
        diagnostics: [],
      };
    } catch (error) {
      return {
        ok: false as const,
        diagnostics: [{ severity: 'error' as const, code: 'UNSUPPORTED_SCHEMA', path: [] as readonly string[], message: error instanceof Error ? error.message : String(error) }],
      };
    }
  },
});
