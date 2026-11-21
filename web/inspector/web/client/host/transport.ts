/** Explicit local mock status; no network, credentials or implicit live fallback. */
import { hostStatus } from '../../../host/provider';
export type HostStatus = typeof hostStatus;
export async function request<T>(path:string):Promise<T> {
  if (path !== '/api/inspector/status') throw new Error(`Unsupported local host query: ${path}`);
  return hostStatus as T;
}
