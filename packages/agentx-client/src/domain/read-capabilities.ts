/** Per-operation guards for optional Management reads. */
import { ManagementError } from '../client.js';
import type { Descriptor } from '../generated/management/types.gen.js';
import { DomainReferenceError } from './address.js';

const features = {
  'workspace-members': { flag: 'supports_workspace_members', label: 'Workspace member pages' },
  'message-history': { flag: 'supports_message_history', label: 'Agent message history' },
} as const;

export type ReadCapability = keyof typeof features;
export type ReadCapabilities = Pick<Descriptor, typeof features[ReadCapability]['flag']>;

/** An optional read is unavailable; the original Scope and other handles remain usable. */
export class DomainCapabilityError extends DomainReferenceError {
  constructor(readonly capability: ReadCapability) {
    super('UnsupportedCapability', `Management does not support ${features[capability].label}`);
    this.name = 'DomainCapabilityError';
  }
}

export async function readWhenSupported<T>(descriptor: ReadCapabilities, capability: ReadCapability, read: () => Promise<T>): Promise<T> {
  if (descriptor[features[capability].flag] !== true) throw new DomainCapabilityError(capability);
  try {
    return await read();
  } catch (error) {
    // The server may change independently of this pinned descriptor.
    if (error instanceof ManagementError && error.code === 'Unsupported') throw new DomainCapabilityError(capability);
    throw error;
  }
}
