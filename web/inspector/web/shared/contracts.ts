/** @aiPurpose Share browser/HTTP presentation contracts while keeping recovery facts and commands owned by the plugin. */
export type { RecoveryList } from '../../plugin/shared/recovery';
export type { Incident, Attempt, Policy, RecoveryCycle, Target } from '../../plugin/shared/domain/recovery';
import type { Attempt, Policy, RecoveryCycle, Target } from '../../plugin/shared/domain/recovery';

export interface WebSession {
  csrfToken: string;
  hostName: string;
  pluginId: 'agentx-inspector';
  serverTime: string;
}
export interface ContinueInput { operationId: string; targets: Target[] }
export interface ContinueReply { attempts: Attempt[] }
export interface ManageInput { cycleId: string; action: 'pause' | 'resume' | 'stop' }
export interface ManageReply { cycle: RecoveryCycle }
export interface ConfigureReply { policy: Policy }
export interface WebError { error: { code: string; message: string; uncertain?: boolean } }
