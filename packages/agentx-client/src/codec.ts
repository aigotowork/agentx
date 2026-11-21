/** @aiPurpose Decode and encode each independently emitted listener contract without rounding native integers. */
import type { SchemaTypes as ManagementSchemas } from './generated/management/schema-types.gen.js';
import type { SchemaTypes as WorkbenchSchemas } from './generated/workbench/schema-types.gen.js';
import type { SchemaTypes as ObservationSchemas } from './generated/observation/schema-types.gen.js';
import { wireDocument as management } from './generated/management/wire-schema.gen.js';
import { wireDocument as workbench } from './generated/workbench/wire-schema.gen.js';
import { wireDocument as observation } from './generated/observation/wire-schema.gen.js';
import { createSchemaCodec } from './schema-codec.js';
import type { Native } from './schema-codec.js';
export { stringifyJson, parseJson } from './schema-codec.js';
export type { Native } from './schema-codec.js';
const managementCodec = createSchemaCodec(management);
const workbenchCodec = createSchemaCodec(workbench, true);
const observationCodec = createSchemaCodec(observation, true);

/** @aiPurpose Parse management ordinary JSON before Number rounding. @aiUsage Select an actual emitted management component. */
export function parseManagementJson<K extends keyof ManagementSchemas>(name: K, text: string): ManagementSchemas[K] {
  return managementCodec.parse(name, text) as ManagementSchemas[K];
}
/** @aiPurpose Encode management data with field-specific native integer bounds. @aiUsage Bigint is allowed only at emitted long-integer fields. */
export function stringifyManagementJson<K extends keyof ManagementSchemas>(name: K, value: ManagementSchemas[K]): string {
  return managementCodec.stringify(name, value);
}
/** @aiPurpose Parse Workbench markers into native SDK bigint values. @aiUsage Select an actual emitted Workbench component. */
export function parseWorkbenchJson<K extends keyof WorkbenchSchemas>(name: K, text: string): Native<WorkbenchSchemas[K]> {
  return workbenchCodec.parse(name, text) as Native<WorkbenchSchemas[K]>;
}
/** @aiPurpose Encode Workbench native SDK bigint operands with exact integer markers. @aiUsage Never pass plain Number or preencoded marker operands. */
export function stringifyWorkbenchJson<K extends keyof WorkbenchSchemas>(name: K, value: Native<WorkbenchSchemas[K]>): string {
  return workbenchCodec.stringify(name, value);
}
/** @aiPurpose Parse passive Observation markers without losing error/evidence branches. @aiUsage Select an actual emitted Observation component. */
export function parseObservationJson<K extends keyof ObservationSchemas>(name: K, text: string): Native<ObservationSchemas[K]> {
  return observationCodec.parse(name, text) as Native<ObservationSchemas[K]>;
}
/** @aiPurpose Encode passive Observation operands using integer markers. @aiUsage Every native integer operand must be bigint. */
export function stringifyObservationJson<K extends keyof ObservationSchemas>(name: K, value: Native<ObservationSchemas[K]>): string {
  return observationCodec.stringify(name, value);
}

/** @aiPurpose Supply generated Workbench request DTOs through the validated native-to-wire mapping. @aiUsage Internal transport boundary; bigint values become actual marker objects before generated JSON serialization. */
export function workbenchWireValue<K extends keyof WorkbenchSchemas>(name: K, value: Native<WorkbenchSchemas[K]>): WorkbenchSchemas[K] {
  // The codec validates the native value and emits only exact marker strings, never lossy JSON numeric operands.
  return JSON.parse(workbenchCodec.stringify(name, value)) as WorkbenchSchemas[K];
}
/** @aiPurpose Supply generated Observation request DTOs through the validated native-to-wire mapping. @aiUsage Internal transport boundary; use only for passive read operands. */
export function observationWireValue<K extends keyof ObservationSchemas>(name: K, value: Native<ObservationSchemas[K]>): ObservationSchemas[K] {
  return JSON.parse(observationCodec.stringify(name, value)) as ObservationSchemas[K];
}
