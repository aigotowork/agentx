/** Finite offline provider. Every value/effect is illustrative and belongs to this page. */
import type { z } from 'zod';
import type { AgentXMockHost, Operation } from './contracts';
import { invokeFixture, mockAgentX } from '../plugin/mock/operations.js';
import { recoveryFixture } from '../plugin/mock/recovery-fixture';
import type { ContinueInput, ManageInput, Policy } from '../web/shared/contracts';

export interface InspectorProvider {
  readonly mode: 'mock';
  readonly generation: string;
  invoke<I extends z.ZodType,O extends z.ZodType>(operation:Operation<I,O>,input:z.input<I>):Promise<z.output<O>>;
  reset(): void;
}
/** This incarnation metadata is mock identity only; the native AgentRef adapter is deliberately deferred. */
export const mockIdentity = Object.freeze({dataset:'agentx-inspector-demo',generation:'demo-page-v1',source:'simulation' as const});
export const provider:InspectorProvider = {
  mode:'mock', generation: mockIdentity.generation,
  async invoke(operation,input) {return invokeFixture(operation,operation.input.parse(input));},
  reset() {location.assign(new URL(location.href));},
};
export const agentx = mockAgentX as unknown as AgentXMockHost;
export const hostStatus = {serverId:'fixture-host',mode:'mock' as const,readOnly:false,source:'simulation' as const};
export async function recoveryRequest(path:string,body?:unknown):Promise<unknown> {
  if (path === '/api/session') return {csrfToken:'local-simulation',hostName:'AgentX · 本地模拟',pluginId:'agentx-inspector',serverTime:new Date().toISOString()};
  if (path === '/api/recovery') return recoveryFixture('recovery.list',{});
  if (path === '/api/recovery/continue') return recoveryFixture('recovery.continue',body as ContinueInput);
  if (path === '/api/recovery/manage') return recoveryFixture('recovery.manage',body as ManageInput);
  if (path === '/api/recovery/configure') return recoveryFixture('recovery.configure',body as Policy);
  throw new Error(`Unsupported local recovery operation: ${path}`);
}
