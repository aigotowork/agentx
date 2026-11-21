/** @aiPurpose Bind Work CLI flows to finite native operator commands with exact revisions and retained command identities. */
import { createWorkbenchClient } from './workbench.js';
import type { WorkbenchTypes as W } from './workbench.js';
import { parseWorkbenchJson, stringifyWorkbenchJson } from './codec.js';
import { UsageError, required, integer, inputText, restrict, retain, newId } from './cli-runtime.js';
import type { CliContext, CliResult } from './cli-runtime.js';

/** @aiPurpose Execute complete Work read/operator workflows on the selected Workbench listener. @aiUsage Structured operands use actual marker JSON; no AgentReport or actor claim is accepted. */
export async function runWork(ctx: CliContext): Promise<CliResult> {
  const v = ctx.values, [group, action, id] = ctx.positionals;
  const api = createWorkbenchClient({ baseUrl: ctx.url('workbench'), fetch: ctx.fetch });
  const opts = { signal: ctx.signal };
  const result = (value: W['Response']): CliResult => ({ value, schema: 'Response', domain: 'workbench' });
  const file = async <K extends keyof W>(name: K, path: string | undefined) => parseWorkbenchJson(name, await inputText(required(path, `--${name} FILE|-`)));
  let descriptor: W['Descriptor'] | undefined;
  async function describe(): Promise<W['Descriptor']> { return descriptor ??= await api.describe(opts); }
  async function generation(): Promise<string> { return v.generation ?? required((await describe()).generation ?? undefined, 'available Workbench generation'); }
  async function workRef(): Promise<W['WorkRef']> {
    if (v['work-ref']) {
      const reference = await file('WorkRef', v['work-ref']);
      if (id && reference.id !== id || v.generation && reference.generation !== v.generation) throw new UsageError('Saved WorkRef conflicts with explicit ID/generation');
      return reference;
    }
    const selected = await describe();
    return { scope: selected.scope, project: selected.project, generation: await generation(), id: required(id, 'WORK_ID') };
  }
  const page = (): W['Page'] => ({ after: v.after ?? null, limit: v.limit ? integer(v.limit, '--limit') : 32n });
  function role(value: string | undefined): W['Role'] {
    if (value === 'implementer' || value === 'Implementer') return 'Implementer';
    if (value === 'checker' || value === 'Checker') return 'Checker';
    throw new UsageError('--role must be implementer or checker');
  }
  if (group === 'workbench' && action === 'show') { restrict(ctx, [], 2); return { value: await api.describe(opts), schema: 'Descriptor', domain: 'workbench' }; }
  if (action === 'list' || action === 'candidates') {
    restrict(ctx, ['generation', 'after', 'limit'], 2);
    return result(action === 'list' ? await api.works.list(await generation(), page(), opts) : await api.works.candidates(await generation(), page(), opts));
  }
  if (action === 'show') { restrict(ctx, ['generation', 'work-ref'], 3); const reference = await workRef(); return result(await api.works.get(reference.generation, reference, opts)); }
  if (action === 'command') {
    restrict(ctx, ['generation'], 3);
    return result(await api.works.command(required(v.generation, '--generation (original command)'), required(id, 'COMMAND_ID'), opts));
  }
  if (action === 'thread') {
    restrict(ctx, ['generation', 'work-ref', 'after', 'limit'], 3);
    const work = await workRef();
    return result(await api.works.thread(work.generation, { work, page: { limit: v.limit ? integer(v.limit, '--limit') : 32n, after: v.after ? integer(v.after, '--after') : null } }, opts));
  }
  const tags = { create: 'CreateWork', brief: 'UpdateBrief', bind: 'BindParticipant', dispatch: 'DispatchAssignment', note: 'SendNote', answer: 'AnswerQuestion', 'check-request': 'RequestCheck', accept: 'AcceptResult', changes: 'RequestChanges' } as const;
  if (!action || !Object.hasOwn(tags, action)) throw new UsageError('Unknown Work command; use --help');
  const allowed = ['input', 'generation', 'command-id', 'pending', 'work-ref', 'if-revision'];
  const local: Record<string, string[]> = {
    create: ['brief', 'implementer-ref', 'checker-ref'], brief: ['brief'], bind: ['role', 'participant-ref'], dispatch: ['role'], note: ['role', 'text', 'file'],
    answer: ['question', 'text', 'file', 'amendment'], 'check-request': ['result', 'reason'], accept: ['result', 'check', 'brief-revision'], changes: ['result', 'check', 'reason'],
  };
  restrict(ctx, [...allowed, ...(local[action] ?? [])], v.input || action === 'create' ? 2 : 3);
  let input: W['ApplyCall'];
  if (v.input) {
    if (Object.keys(v).some(key => [...allowed.filter(key => key !== 'input' && key !== 'pending'), ...(local[action] ?? [])].includes(key))) throw new UsageError('--input cannot combine with Work body/identity flags');
    input = await file('ApplyCall', v.input);
    const expected = Object.entries(tags).find(([name]) => name === action)?.[1];
    if (!expected || !(expected in input.command)) throw new UsageError('Saved ApplyCall command does not match selected CLI action');
  } else {
    const work = action === 'create' ? null : await workRef();
    let detail: W['WorkDetail'] | undefined;
    if (work && (!v['if-revision'] || action === 'accept' && !v['brief-revision'])) {
      const read = await api.works.get(work.generation, work, opts);
      if (!('Read' in read) || !('Detail' in read.Read)) return result(read);
      detail = read.Read.Detail;
    }
    const expected_revision = work ? v['if-revision'] !== undefined ? integer(v['if-revision'], '--if-revision') : detail?.work.revision : null;
    if (work && expected_revision === undefined) throw new UsageError('Cannot establish original expected revision');
    let command: W['OperatorCommand'];
    const text = async () => {
      if ((v.text === undefined) === (v.file === undefined)) throw new UsageError('Supply exactly one --text or --file');
      return v.text ?? inputText(required(v.file, '--file'));
    };
    switch (action) {
      case 'create': command = { CreateWork: { brief: await file('BriefInput', v.brief), implementer: await file('ParticipantRef', v['implementer-ref']), checker: await file('ParticipantRef', v['checker-ref']) } }; break;
      case 'brief': command = { UpdateBrief: await file('BriefInput', v.brief) }; break;
      case 'bind':
        if (v.role?.length !== 1) throw new UsageError('Bind takes exactly one --role');
        command = { BindParticipant: { role: role(v.role[0]), participant: await file('ParticipantRef', v['participant-ref']) } }; break;
      case 'dispatch':
        if (v.role?.length !== 1) throw new UsageError('Dispatch takes exactly one --role');
        command = { DispatchAssignment: { role: role(v.role[0]) } }; break;
      case 'note':
        if (!v.role?.length) throw new UsageError('Supply recipient --role');
        command = { SendNote: { roles: v.role.map(role), text: await text() } }; break;
      case 'answer': command = { AnswerQuestion: { question_id: required(v.question, '--question'), text: await text(), amendment: v.amendment ? await file('BriefInput', v.amendment) : null } }; break;
      case 'check-request': command = { RequestCheck: { result_id: required(v.result, '--result'), reason: required(v.reason, '--reason') } }; break;
      case 'accept': {
        const result_id = required(v.result, '--result');
        const brief_revision = v['brief-revision'] !== undefined ? integer(v['brief-revision'], '--brief-revision') : detail?.work.results.find(result => result.id === result_id)?.brief_revision;
        if (brief_revision === undefined) throw new UsageError('Selected result basis is unavailable; supply --brief-revision');
        command = { AcceptResult: { result_id, check_id: required(v.check, '--check'), brief_revision } }; break;
      }
      case 'changes': command = { RequestChanges: { result_id: required(v.result, '--result'), check_id: v.check ?? null, reason: required(v.reason, '--reason') } }; break;
      default: throw new UsageError('Unsupported Work action');
    }
    input = { generation: work?.generation ?? await generation(), command_id: v['command-id'] ?? newId(), work, expected_revision: expected_revision ?? null, command };
  }
  const body = stringifyWorkbenchJson('ApplyCall', input);
  await retain(ctx, input.command_id, body, `work.${action}`, { generation: input.generation, command_id: input.command_id });
  return result(await api.apply(parseWorkbenchJson('ApplyCall', body), opts));
}
