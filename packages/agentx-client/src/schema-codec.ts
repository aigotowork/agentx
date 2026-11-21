/** @aiPurpose Validate each actual emitted listener schema and map its exact native integer representation. */
import { parse, stringify, isLosslessNumber } from 'lossless-json';


export type Schema = {
  $ref?: string; type?: string | readonly string[]; format?: string; enum?: readonly unknown[];
  const?: unknown; nullable?: boolean; properties?: Readonly<Record<string, Schema>>;
  required?: readonly string[]; additionalProperties?: boolean | Schema;
  items?: Schema | boolean; prefixItems?: readonly Schema[]; minItems?: number; maxItems?: number; oneOf?: readonly Schema[]; anyOf?: readonly Schema[]; allOf?: readonly Schema[];
  minimum?: number; maximum?: number;
  [extension: `x-${string}`]: unknown;
};
type Document = { components: { schemas: Readonly<Record<string, Schema>> } };
/** @aiPurpose Derive SDK values from emitted integer-marker wire DTOs. @aiUsage Apply to generated Workbench/Observation types; do not author another business DTO. */
export type Native<T> = T extends { $integer: string } ? bigint : T extends readonly unknown[] ? { [K in keyof T]: Native<T[K]> } : T extends object ? { [K in keyof T]: Native<T[K]> } : T;

/** @aiPurpose Bind structural validation and exact numeric conversion to one actual emitted schema closure. @aiUsage Only wrappers for the matching listener may use the resulting codec. */
export function createSchemaCodec(document: Document, markerMode = false) {
  const schemas = document.components.schemas;

  function record(value: unknown): value is Record<string, unknown> {
    return typeof value === 'object' && value !== null && !Array.isArray(value) && !isLosslessNumber(value);
  }
  function reference(schema: Schema): Schema {
    if (!schema.$ref) return schema;
    const name = schema.$ref.replace('#/components/schemas/', '');
    const result = schemas[name];
    if (!result || schema.$ref !== `#/components/schemas/${name}`) throw new TypeError(`Unsupported schema reference: ${schema.$ref}`);
    return result;
  }
  function convert(value: unknown, source: Schema, path: string, encode = false): unknown {
    const schema = reference(source);
    const types = typeof schema.type === 'string' ? [schema.type] : schema.type;
    if (value === null && (schema.nullable || types?.includes('null'))) return null;
    const alternatives = schema.oneOf ?? schema.anyOf;
    if (alternatives) {
      const matches: unknown[] = [];
      for (const branch of alternatives) {
        try {
          const converted = convert(value, branch, path, encode);
          if (schema.anyOf) return converted;
          matches.push(converted);
        } catch { /* A closed union branch must validate the actual value. */ }
      }
      if (matches.length === 1) return matches[0];
      throw new TypeError(`${path}: expected exactly one emitted union branch, got ${matches.length}`);
    }
    if (schema.allOf) {
      let result = value;
      for (const branch of schema.allOf) result = convert(result, branch, path, encode);
      return result;
    }
    if (markerMode && schema.properties?.$integer) {
      const metadata = schema['x-agentx-native-integer'];
      const format = typeof metadata === 'string' ? metadata : metadata && typeof metadata === 'object' && 'format' in metadata && typeof metadata.format === 'string' ? metadata.format : undefined;
      let integer: bigint;
      if (encode) {
        if (typeof value !== 'bigint') throw new TypeError(`${path}: native integer operand must be bigint`);
        integer = value;
      } else {
        if (!record(value) || Object.keys(value).length !== 1 || typeof value.$integer !== 'string' || !/^(0|-[1-9][0-9]*|[1-9][0-9]*)$/.test(value.$integer)) throw new TypeError(`${path}: invalid integer marker`);
        integer = BigInt(value.$integer);
      }
      const ranges: Record<string, readonly [bigint, bigint]> = {
        'u-int8': [0n, 255n], uint8: [0n, 255n], int8: [-128n, 127n],
        'u-int16': [0n, 65535n], uint16: [0n, 65535n], int16: [-32768n, 32767n],
        'u-int32': [0n, 4294967295n], uint32: [0n, 4294967295n], int32: [-2147483648n, 2147483647n],
        'u-int64': [0n, 18446744073709551615n], uint64: [0n, 18446744073709551615n], int64: [-9223372036854775808n, 9223372036854775807n],
      };
      const range = format ? ranges[format] : undefined;
      const min = range?.[0] ?? -9223372036854775808n, max = range?.[1] ?? 18446744073709551615n;
      if (integer < min || integer > max) throw new RangeError(`${path}: integer outside native range`);
      return encode ? { $integer: integer.toString() } : integer;
    }
    if (schema.enum && !schema.enum.includes(value)) throw new TypeError(`${path}: invalid enum value`);
    if ('const' in schema && schema.const !== value) throw new TypeError(`${path}: invalid constant`);
    if (markerMode && (isLosslessNumber(value) || typeof value === 'number')) throw new TypeError(`${path}: bare numeric value on integer-marker surface`);
    if (types?.includes('integer')) {
      let token: string;
      if (isLosslessNumber(value)) token = value.value;
      else if (typeof value === 'bigint') token = value.toString();
      else if (typeof value === 'number' && Number.isSafeInteger(value)) token = String(value);
      else throw new TypeError(`${path}: expected exact integer, unsafe numbers and digit strings refused`);
      if (!/^-?(0|[1-9]\d*)$/.test(token)) throw new TypeError(`${path}: expected integral JSON token`);
      const integer = BigInt(token);
      const bounds: Record<string, readonly [bigint, bigint]> = {
        uint64: [0n, 18446744073709551615n], int64: [-9223372036854775808n, 9223372036854775807n],
        uint32: [0n, 4294967295n], int32: [-2147483648n, 2147483647n],
      };
      const range = bounds[schema.format?.replace('u-int', 'uint') ?? ''];
      if (range && (integer < range[0] || integer > range[1])) throw new RangeError(`${path}: outside ${schema.format}`);
      if (schema.minimum !== undefined && integer < BigInt(schema.minimum)) throw new RangeError(`${path}: below minimum`);
      if (schema.maximum !== undefined && integer > BigInt(schema.maximum)) throw new RangeError(`${path}: above maximum`);
      if (schema.format === 'uint64' || schema.format === 'u-int64' || schema.format === 'int64') return integer;
      const number = Number(integer);
      if (!Number.isSafeInteger(number)) throw new RangeError(`${path}: integer lacks a lossless field policy`);
      return number;
    }
    if (types?.includes('number')) {
      const number = isLosslessNumber(value) ? Number(value.value) : value;
      if (typeof number !== 'number' || !Number.isFinite(number)) throw new TypeError(`${path}: expected finite number`);
      return number;
    }
    if (types?.includes('object') || schema.properties) {
      if (!record(value)) throw new TypeError(`${path}: expected object`);
      for (const key of schema.required ?? []) if (!(key in value)) throw new TypeError(`${path}.${key}: required`);
      const result: Record<string, unknown> = {};
      for (const [key, item] of Object.entries(value)) {
        if (item === undefined) continue;
        const field = schema.properties?.[key];
        if (field) Object.defineProperty(result, key, { value: convert(item, field, `${path}.${key}`, encode), enumerable: true, writable: true, configurable: true });
        else if (schema.additionalProperties === false) throw new TypeError(`${path}.${key}: unknown field`);
        else Object.defineProperty(result, key, { value: convert(item, typeof schema.additionalProperties === 'object' ? schema.additionalProperties : {}, `${path}.${key}`, encode), enumerable: true, writable: true, configurable: true });
      }
      return result;
    }
    if (types?.includes('array')) {
      if (!Array.isArray(value)) throw new TypeError(`${path}: expected array`);
      if (schema.minItems !== undefined && value.length < schema.minItems || schema.maxItems !== undefined && value.length > schema.maxItems) throw new TypeError(`${path}: invalid array length`);
      return value.map((item, index) => {
        const prefix = schema.prefixItems?.[index];
        if (!prefix && schema.items === false) throw new TypeError(`${path}: extra tuple element`);
        return convert(item, prefix ?? (typeof schema.items === 'object' ? schema.items : {}), `${path}[${index}]`, encode);
      });
    }
    if (types?.includes('string') && typeof value !== 'string') throw new TypeError(`${path}: expected string`);
    if (types?.includes('boolean') && typeof value !== 'boolean') throw new TypeError(`${path}: expected boolean`);
    if (types?.length === 1 && types[0] === 'null' && value !== null) throw new TypeError(`${path}: expected null`);
    // An untyped extensibility field must never smuggle an unsafe integer through Number.
    if (isLosslessNumber(value)) {
      const number = Number(value.value);
      if (!Number.isFinite(number) || (Number.isInteger(number) && !Number.isSafeInteger(number))) throw new RangeError(`${path}: numeric token lacks emitted field policy`);
      return number;
    }
    if (Array.isArray(value)) return value.map((item, i) => convert(item, {}, `${path}[${i}]`, encode));
    if (record(value)) return Object.fromEntries(Object.entries(value).map(([key, item]) => [key, convert(item, {}, `${path}.${key}`, encode)]));
    if (typeof value === 'bigint') throw new TypeError(`${path}: bigint lacks emitted integer field policy`);
    return value;
  }

  return {
    parse(schemaName: string, text: string): unknown {
      const schema = schemas[schemaName];
      if (!schema) throw new TypeError(`Unknown emitted schema ${schemaName}`);
      return convert(parse(text, undefined, { onDuplicateKey: ({ key }) => { throw new TypeError(`Duplicate JSON key ${key}`); } }), schema, schemaName);
    },
    stringify(schemaName: string, value: unknown): string {
      const schema = schemas[schemaName];
      if (!schema) throw new TypeError(`Unknown emitted schema ${schemaName}`);
      const text = stringify(convert(value, schema, schemaName, true));
      if (text === undefined) throw new TypeError('Expected serializable JSON value');
      return text;
    },
  };
}

/** @aiPurpose Print SDK values as lossless ordinary JSON. @aiUsage CLI and persistence output; schema validation belongs to the listener codec. */
export function stringifyJson(value: unknown): string {
  const text = stringify(value);
  if (text === undefined) throw new TypeError('Expected serializable JSON value');
  return text;
}

/** @aiPurpose Read lossless ordinary JSON envelopes from caller storage. @aiUsage Restore typed domain values through the matching schema codec; unsafe integer tokens remain bigint. */
export function parseJson(text: string): unknown {
  return parse(text, undefined, {
    parseNumber: token => {
      const value = Number(token);
      if (!Number.isFinite(value)) throw new RangeError('JSON numeric token is not finite');
      return /^-?(0|[1-9][0-9]*)$/.test(token) && !Number.isSafeInteger(value) ? BigInt(token) : value;
    },
    onDuplicateKey: ({ key }) => { throw new TypeError(`Duplicate JSON key ${key}`); },
  });
}
