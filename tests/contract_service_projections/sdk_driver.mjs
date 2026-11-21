/** Installed-package acceptance only: copy into an external consumer project before invoking. */
import { readFile } from 'node:fs/promises';
import {
  createManagementClient, createWorkbenchClient, createObservationClient,
  parseManagementJson, parseWorkbenchJson, parseObservationJson, stringifyJson,
} from '@agentx/client';

const request = JSON.parse(await readFile(process.argv[2], 'utf8'));
const domain = request.domain ?? 'management';
const factories = { management: createManagementClient, workbench: createWorkbenchClient, observation: createObservationClient };
if (!Object.hasOwn(factories, domain)) throw new Error('Unknown actual public SDK domain');
const api = factories[domain]({ baseUrl: request.baseUrl });
const options = request.options ?? {};
if (request.cancelBeforeWait) options.signal = AbortSignal.abort('acceptance local observation stop');
const parsers = { management: parseManagementJson, workbench: parseWorkbenchJson, observation: parseObservationJson };
// Optional argument envelopes parse native values from actual emitted component wire
// schemas. The harness never copies DTOs or converts native marker integers with Number.
const args = request.arguments
  ? request.arguments.map(arg => arg?.component && typeof arg.wireText === 'string'
    ? parsers[domain](arg.component, arg.wireText) : arg)
  : request.action === 'describe' ? [options] : [request.input, options];
if (request.binaryUploadFile) args[0] = { ...args[0], bytes: new Blob([await readFile(request.binaryUploadFile)]) };
const parts = request.action.split('.');
const method = parts.pop();
let receiver = api;
for (const part of parts) {
  if (!Object.hasOwn(receiver, part)) throw new Error('Unknown actual public SDK group');
  receiver = receiver[part];
}
if (!Object.hasOwn(receiver, method) || typeof receiver[method] !== 'function') throw new Error('Unknown actual public SDK method');
function integerTypes(value, path = '$', output = []) {
  if (typeof value === 'bigint') output.push({ path, type: 'bigint', value: value.toString() });
  else if (value && typeof value === 'object') {
    for (const [key, child] of Object.entries(value)) integerTypes(child, `${path}.${key}`, output);
  }
  return output;
}
try {
  let value = await receiver[method](...args);
  if (value instanceof Blob) value = { bytes: [...new Uint8Array(await value.arrayBuffer())], type: value.type };
  console.log(stringifyJson({ ok: true, value, integerTypes: integerTypes(value) }));
} catch (error) {
  console.log(stringifyJson({ ok: false, error: {
    name: error.name, code: error.code, message: error.message, status: error.status,
    input: error.input, operation: error.operation, identity: error.identity, lastOperation: error.lastOperation,
  } }));
}
