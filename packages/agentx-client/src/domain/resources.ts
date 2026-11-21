/** Independent resources that share the selected scope but do not belong to an Agent or Workspace. */
import type { DirectoryEntry, HumanMailbox, SubmitDirectoryMessage } from '../generated/management/types.gen.js';
import type { ManagementClient } from '../management.js';
import { freezeInput } from '../transport.js';
import type { CallOptions } from '../transport.js';
import { DomainReferenceError, readCurrentReference, requireGeneration, requireScope } from './address.js';
import type { Attachments, DirectoryRoutes, HumanInbox, InboxPage, OfficeActivityCursor, OfficeActivityPage, PageRequest, ScopeAddress } from './contracts.js';
import { checkedReceipt, commandIdentity, directorySendResult, operationAddress, retainInput } from './operation.js';

export function bindAttachments(client: ManagementClient, scope: ScopeAddress): Attachments {
  const attachments: Attachments = {
    async upload({ name, mediaType, bytes }, options) {
      const { id, ...metadata } = await client.attachments.upload({ generation: scope.generation, name, media_type: mediaType, bytes }, options);
      if (!id) throw new DomainReferenceError('InconsistentResponse', 'Attachment registration did not return an ID');
      return freezeInput({ address: { scope, id }, ...metadata });
    },
    async download(address, options) {
      requireScope(address.scope, scope);
      if (typeof address.id !== 'string' || !address.id) throw new DomainReferenceError('InvalidReference', 'Expected a retained attachment ID');
      return client.attachments.download({ generation: scope.generation, id: address.id }, options);
    },
  };
  return Object.freeze(attachments);
}

function inboxPage(scope: ScopeAddress, pending: boolean, result: HumanMailbox): InboxPage {
  requireGeneration(result.generation, scope);
  for (const item of result.messages) if (item.sender != null) readCurrentReference(item.sender);
  return freezeInput({ scope, messages: result.messages, unhandledCount: result.unhandled_count,
    next: result.next_before == null ? null : { kind: 'human-inbox', scope, pending, position: result.next_before } });
}

export function bindInbox(client: ManagementClient, scope: ScopeAddress): HumanInbox {
  const inbox: HumanInbox = {
    async list(page = {}, options) {
      const pending = page.pending ?? false;
      if (page.before) {
        requireScope(page.before.scope, scope);
        if (page.before.kind !== 'human-inbox' || page.before.pending !== pending) throw new DomainReferenceError('ReferenceMismatch', 'Cursor belongs to a different mailbox page selection');
      }
      const result = await client.humanMailbox.list({ generation: scope.generation, before: page.before?.position, limit: page.limit, pending }, options);
      return inboxPage(scope, pending, result);
    },
    async handle(ids, options) {
      return inboxPage(scope, false, await client.humanMailbox.handle({ generation: scope.generation, ids: [...ids] }, options));
    },
  };
  return Object.freeze(inbox);
}

function checkDirectory(directory: string) {
  if (typeof directory !== 'string' || !directory || directory.includes('\0')) throw new TypeError('Supply a host directory');
  return directory;
}

function checkTextOnly(message: { attachments?: readonly string[] }) {
  if (message.attachments !== undefined && (!Array.isArray(message.attachments) || message.attachments.length !== 0)) {
    throw new TypeError('Directory send supports text only');
  }
}

function inspectDirectory(scope: ScopeAddress, entry: DirectoryEntry) {
  requireGeneration(entry.generation, scope);
  if (entry.endpoint.participant != null) readCurrentReference(entry.endpoint.participant);
  return freezeInput(entry);
}

export function bindDirectory(client: ManagementClient, scope: ScopeAddress): DirectoryRoutes {
  const directory: DirectoryRoutes = {
    async inspect(directory, options) {
      return inspectDirectory(scope, await client.directory.inspect({ generation: scope.generation, directory: checkDirectory(directory) }, options));
    },
    prepareSend(directory, message, id) {
      const operation = operationAddress(scope, id);
      checkTextOnly(message);
      const input = retainInput('SubmitDirectoryMessage', { ...commandIdentity(operation, scope), directory: checkDirectory(directory), message: { ...message, attachments: message.attachments && [...message.attachments] } });
      return freezeInput({ kind: 'directory-send', operation, directory: input.directory, message: input.message });
    },
    async send(prepared, options) {
      if (prepared.kind !== 'directory-send') throw new TypeError('Expected a prepared directory send');
      const identity = commandIdentity(prepared.operation, scope);
      checkTextOnly(prepared.message);
      const input: SubmitDirectoryMessage = { ...identity, directory: checkDirectory(prepared.directory), message: { ...prepared.message, attachments: prepared.message.attachments && [...prepared.message.attachments] } };
      const receipt = await client.directory.send(input, options);
      return checkedReceipt('directory-send', operationAddress(scope, identity.request_id), receipt, directorySendResult(scope));
    },
  };
  return Object.freeze(directory);
}

export async function readOfficeActivity(client: ManagementClient, scope: ScopeAddress, page: PageRequest<OfficeActivityCursor> = {}, options?: CallOptions): Promise<OfficeActivityPage> {
  if (page.after) {
    requireScope(page.after.scope, scope);
    if (page.after.kind !== 'office-activity') throw new DomainReferenceError('InvalidReference', 'Expected an office activity cursor');
  }
  const { next_after, ...result } = await client.officeActivity.list({ generation: scope.generation, after: page.after?.position, limit: page.limit }, options);
  requireGeneration(result.generation, scope);
  for (const ref of result.created) readCurrentReference(ref);
  for (const event of result.messages) {
    if (event.sender != null) readCurrentReference(event.sender);
    for (const ref of event.recipients) readCurrentReference(ref);
  }
  for (const item of result.human_messages) if (item.sender != null) readCurrentReference(item.sender);
  return freezeInput({ ...result, checkpoint: { kind: 'office-activity', scope, position: next_after } });
}
