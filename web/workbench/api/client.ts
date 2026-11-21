/** Page lifetime and cancellation over the generated public Workbench SDK. */
import { createWorkbenchClient, AdapterError, AdapterDisconnected } from '@agentx/client';
import type { WorkbenchClient } from '@agentx/client';
import type { ApplyCall, Change, Descriptor, ReadCall, Response as NativeResponse } from './native';
export type * from './native';
export { AdapterError, AdapterDisconnected } from '@agentx/client';
export { WorkOutcomeUnknown as OutcomeUnknown } from '@agentx/client';

/** Own one explicit listener connection; close cancels reads, never submitted work. */
export class WorkbenchApi {
  private closed = false;
  private readonly pending = new Set<AbortController>();
  private constructor(readonly descriptor: Descriptor, private readonly client: WorkbenchClient) {}
  static async connect(baseUrl = new URLSearchParams(location.search).get('workbenchUrl') ?? location.origin): Promise<WorkbenchApi> {
    const client = createWorkbenchClient({ baseUrl });
    return new WorkbenchApi(await client.describe(), client);
  }
  async read(call: ReadCall, signal?: AbortSignal): Promise<NativeResponse> {
    if (this.closed) throw new AdapterDisconnected('Closed');
    if (this.pending.size >= 8) throw new AdapterError(503, 'Busy');
    const controller = new AbortController();
    const abort = () => controller.abort(signal?.reason);
    if (signal?.aborted) abort(); else signal?.addEventListener('abort', abort, { once: true });
    this.pending.add(controller);
    try { return await this.client.read(call, { signal: controller.signal }); }
    finally { this.pending.delete(controller); signal?.removeEventListener('abort', abort); }
  }
  async apply(call: ApplyCall): Promise<NativeResponse> {
    if (this.closed) throw new AdapterDisconnected('Closed');
    return this.client.apply(call);
  }
  lookup(commandId: string, signal?: AbortSignal): Promise<NativeResponse> {
    const generation = this.descriptor.generation;
    if (!generation) throw new AdapterDisconnected('Generation unavailable');
    return this.read({ generation, read: { Command: { command_id: commandId } } }, signal);
  }
  close(): void { this.closed = true; for (const pending of this.pending) pending.abort(); this.pending.clear(); }
}
/**
 * @aiPurpose Allocate a stable command identity on credential-free numeric-LAN HTTP.
 * @aiUsage Generate once per explicit intent; retain that value for lookup or exact retry.
 */
export function newCommandId(): string {
  const bytes = crypto.getRandomValues(new Uint8Array(16));
  bytes[6] = (bytes[6]! & 0x0f) | 0x40;
  bytes[8] = (bytes[8]! & 0x3f) | 0x80;
  const hex = Array.from(bytes, byte => byte.toString(16).padStart(2, "0")).join("");
  return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`;
}
export type CommandReceipt = Change;
