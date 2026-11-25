#!/usr/bin/env node
/** @aiPurpose Compose the complete separate Node HTTP CLI while preserving listener-specific wire, effects and cancellation knowledge. */
import { parseCli, configuration, seconds, UsageError } from './cli-runtime.js';
import type { CliContext } from './cli-runtime.js';
import { runManagement } from './cli-management.js';
import { runWork } from './cli-work.js';
import { runObservation } from './cli-observation.js';
import { runCloudAdmin } from './cli-cloud-admin.js';
import { CloudAdminError, CloudAdminOutcomeUnknown } from './cloud-admin.js';
import { ManagementError, OutcomeUnknown, ReceiptError, ObservationStopped } from './client.js';
import { WorkOutcomeUnknown } from './workbench.js';
import { AdapterError, AdapterDisconnected, MutationOutcomeUnknown, ProtocolError } from './transport.js';
import { stringifyJson, stringifyWorkbenchJson } from './codec.js';
import { wireDocument as workbench } from './generated/workbench/wire-schema.gen.js';
import { wireDocument as observation } from './generated/observation/wire-schema.gen.js';
import { createSchemaCodec } from './schema-codec.js';

const help = `agentx-http 0.1.0 — AgentX HTTP SDK companion (native agentx stays separate)

management show
workspaces list
roles list --workspace PATH
directory inspect PATH | directory send PATH MESSAGE [WRITE]
agents list [--after CURSOR --limit N --current-only --label SELECTOR...]
agents show NAME EXACT
agents launch [--tool TOOL --profile PROFILE --role ROLE --workspace PATH
  --directory SERVER_PATH --title TEXT --prompt TEXT --label KEY=VALUE...] [WRITE]
agents kill|stop-participation|resume|fork|transcript NAME EXACT [WRITE]
agents relations NAME EXACT [--after CURSOR --limit N]
agents messages list NAME EXACT [--after ID --limit N]
agents messages send NAME EXACT MESSAGE [WRITE]
agents labels get NAME EXACT
agents labels patch NAME EXACT [--set KEY=VALUE... --remove KEY... --if-revision REV] [WRITE]
operations show|wait ORIGINAL_Q --generation ORIGINAL_G [--sent] [WAIT]
attachments upload FILE|- [--name NAME --media-type MIME]
attachments download ID --output NEW_FILE|-
mailbox list [--before ID --pending-only --limit N] | mailbox handle ID...
office activity [--after ID --limit N]
inspection show
inspection configure --if-revision REV (--enabled|--disabled)
  (--recovery-enabled|--recovery-disabled) --error-duration-seconds N
inspection analyses create --observation ID... [--request-id UUID]
inspection analyses cancel JOB_ID | inspection continue --episode FILE|-
workbench show
work list|candidates [--after CURSOR --limit N]
work show ID | work command ORIGINAL_COMMAND_ID --generation ORIGINAL_G
work thread ID [--after EVENT_ID --limit N]
work create --brief FILE|- --implementer-ref FILE --checker-ref FILE [WORKWRITE]
work brief ID --brief FILE|- [REVISION] [WORKWRITE]
work bind ID --role implementer|checker --participant-ref FILE [REVISION] [WORKWRITE]
work dispatch ID --role implementer|checker [REVISION] [WORKWRITE]
work note ID --role implementer|checker... MESSAGE_TEXT [REVISION] [WORKWRITE]
work answer ID --question ID MESSAGE_TEXT [--amendment FILE] [REVISION] [WORKWRITE]
work check-request ID --result ID --reason TEXT [REVISION] [WORKWRITE]
work accept ID --result ID --check ID [--brief-revision REV] [REVISION] [WORKWRITE]
work changes ID --result ID [--check ID] --reason TEXT [REVISION] [WORKWRITE]
observe scope|snapshot
observe requests list [--selection FILE --before ID --limit N]
observe requests show REF_FILE|-
observe connections|peers|hosts [--before ID --limit N]
observe reports list [--before ID --limit N] | observe reports show REF_FILE|-
observe config head [--expected-epoch FILE] | observe config show REF_FILE|-
observe config compare LEFT_REF_FILE RIGHT_REF_FILE [--field TAGGED_JSON]...
observe events describe | observe events read --window FILE|- [--limit N]
admin overview
admin namespaces list [--after CURSOR --limit N] | admin namespaces show ID
admin namespaces hosts ID [--generation G] [--after CURSOR --limit N]
admin namespaces agents ID [--generation G] [--host HOST --workspace-id ID --after CURSOR --limit N]
admin namespaces workspaces ID [--generation G] [--host HOST --after CURSOR --limit N]
  For inventory --after, pass --generation from the original page.
admin hosts capabilities NAMESPACE --host HOST
admin operations add-workspace NAMESPACE CONTROL --name NAME --parent ROOT_ALIAS --directory PATH [--create-directory]
admin operations launch NAMESPACE CONTROL --workspace-id ID [--role ROLE --name NAME --title TITLE --task TEXT --directory PATH --create-directory]
admin operations stop NAMESPACE CONTROL --name NAME --incarnation INCARNATION
admin operations show NAMESPACE --request-id ID
admin namespaces create ID [ADMINWRITE]
admin namespaces rotate-token ID --expected-version N [ADMINWRITE]
admin pending replay FILE | admin pending reveal FILE

Global options (any position): --management-url URL, --workbench-url URL,
  --observation-url URL (independent; respective AGENTX_API_*_URL env),
  --config FILE (AGENTX_API_CONFIG; default XDG_CONFIG_HOME/agentx-http/config.json),
  --timeout-seconds N (default15), --format json, --help, --version.
EXACT: --incarnation I | --stopped-event E (exactly one; no name fallback).
MESSAGE: --text TEXT | --file FILE|-; --intent request|inform|ack,
  --reply-to ID --thread NAME --attachment ID... . MESSAGE_TEXT uses text/file only.
WRITE: --generation G (default describe once), --request-id UUID (new intent default),
  --pending FILE (default .agentx-http/requests/ID.json; exclusive fsync before dispatch),
  --wait [WAIT]. --input FULL_REQUEST_FILE|- replaces target/body/identity flags.
WORKWRITE: --generation G --command-id UUID --pending FILE; alternatively
  --input FULL_APPLY_CALL_FILE|- replaces ID/body/identity flags.
REVISION: --if-revision REV (default read exact Work once), --work-ref REF_FILE.
WAIT: --poll-seconds N (default0.25), --wait-timeout-seconds N (default30).
Admin: --admin-url URL or AGENTX_ADMIN_URL; AGENTX_ADMIN_TOKEN is required
  for network calls. HTTPS is required except on loopback.
ADMINWRITE: --request-id ID --pending FILE --new-token-file FILE|- (optional).
CONTROL: --generation G --host HOST --installation-id INSTALLATION [--request-id ID --pending FILE].
  Omit --role for default Codex. Queued/running means accepted, not completed.
  Use admin operations show to check the receipt; pending replay checks that receipt before resending.
  Without --new-token-file a token is generated. Retrieve it only by explicitly
  running admin pending reveal. Use admin pending replay to resend original input.
Observation: --expected-daemon REF_FILE (default null); never auto-rebase/start daemon.
Management JSON files/stdout preserve ordinary numbers losslessly. Workbench and
  Observation JSON files/stdout use native {"$integer":"decimal"} wire markers.
Only one file operand may use stdin. File outputs never overwrite existing files.
stdout: JSON results (binary only for download --output -); stderr: diagnostics.
Exits: 0 success/accepted, 1 explicit failure, 2 usage/protocol, 3 unknown result,
  124 deadline, 130 local read/wait interrupt. Stopping observation never cancels work.
No automatic mutation retry, actor impersonation or installed/live mutation.
See package README for original-identity recovery and whole-system generation.
`;
const workbenchCodec = createSchemaCodec(workbench, true);
const observationCodec = createSchemaCodec(observation, true);
function terminalExit(value: unknown): number {
  if (!value || typeof value !== 'object') return 0;
  if ('status' in value) {
    if (value.status === 'Failed') return 1;
    if (value.status === 'Unknown') return 3;
  }
  if ('Rejected' in value) return 1;
  if ('LocalFailure' in value || 'Err' in value) return 1;
  if ('Inspection' in value) return terminalExit(value.Inspection);
  if ('response' in value) return terminalExit(value.response);
  if ('DaemonChanged' in value) return 1;
  // InspectionResult is a closed result value whose variant contains a native Result.
  for (const field of ['Runtime', 'Configuration', 'Events']) if (field in value) return terminalExit(Reflect.get(value, field));
  return 0;
}
async function main(): Promise<void> {
  let parsed: ReturnType<typeof parseCli>;
  try { parsed = parseCli(); } catch (error) { throw new UsageError(error instanceof Error ? error.message : String(error)); }
  const { values, positionals } = parsed;
  if (values.version) { console.log('0.1.0'); return; }
  if (values.help || positionals.length === 0) { console.log(help); return; }
  const config = await configuration(values);
  if ((values.format ?? process.env.AGENTX_API_FORMAT ?? config.format ?? 'json') !== 'json') throw new UsageError('Only --format json is supported');
  const timeoutMs = seconds(values['timeout-seconds'] ?? process.env.AGENTX_API_TIMEOUT_SECONDS ?? config.timeout_seconds, 15);
  const interrupted = new AbortController();
  const interrupt = () => interrupted.abort('Canceled');
  process.once('SIGINT', interrupt);
  const boundedFetch: typeof fetch = (request, init) => fetch(request, { ...init, signal: AbortSignal.any([
    ...(request instanceof Request ? [request.signal] : []), ...(init?.signal ? [init.signal] : []), interrupted.signal, AbortSignal.timeout(timeoutMs),
  ]) });
  const ctx: CliContext = {
    values, positionals, config, signal: interrupted.signal, fetch: boundedFetch,
    waitOptions: { signal: interrupted.signal, timeoutMs: seconds(values['wait-timeout-seconds'] ?? process.env.AGENTX_API_WAIT_TIMEOUT_SECONDS ?? config.wait_timeout_seconds, 30), pollIntervalMs: seconds(values['poll-seconds'] ?? process.env.AGENTX_API_POLL_SECONDS ?? config.poll_seconds, 0.25) },
    url(domain) {
      const selected = values[`${domain}-url`] ?? process.env[`AGENTX_API_${domain.toUpperCase()}_URL`] ?? config[`${domain}_url`];
      if (typeof selected !== 'string' || !selected) throw new UsageError(`--${domain}-url or corresponding AGENTX_API URL is required`);
      const url = new URL(selected);
      if (!['http:', 'https:'].includes(url.protocol)) throw new UsageError('Listener URL must use http/https');
      return selected;
    },
  };
  try {
    const group = positionals[0];
    const output = group === 'work' || group === 'workbench' ? await runWork(ctx)
      : group === 'observe' ? await runObservation(ctx)
      : group === 'admin' ? await runCloudAdmin(ctx) : await runManagement(ctx);
    if (output) {
      console.log(output.domain === 'workbench' ? workbenchCodec.stringify(output.schema, output.value) : output.domain === 'observation' ? observationCodec.stringify(output.schema, output.value) : stringifyJson(output.value));
      process.exitCode = output.domain === 'cloud-admin' && output.value && typeof output.value === 'object' && 'status' in output.value
        ? output.value.status === 'failed' ? 1 : output.value.status === 'uncertain' ? 3 : 0
        : terminalExit(output.value);
    }
  } catch (error) {
    // Preserve typed original identities/receipts before handling an unclassified interrupted read.
    if (interrupted.signal.aborted && !(error instanceof OutcomeUnknown) && !(error instanceof MutationOutcomeUnknown) && !(error instanceof WorkOutcomeUnknown) && !(error instanceof ObservationStopped) && !(error instanceof ReceiptError) && !(error instanceof ManagementError) && !(error instanceof AdapterError) && !(error instanceof CloudAdminOutcomeUnknown)) {
      console.error(stringifyJson({ code: 'Canceled', message: 'Local read interrupted; server work is unchanged' })); process.exitCode = 130; return;
    }
    throw error;
  } finally { process.removeListener('SIGINT', interrupt); }
}
main().catch((error: unknown) => {
  let code = 'ClientError', exit = 2;
  const detail: Record<string, unknown> = { message: error instanceof Error ? error.message : String(error) };
  if (error instanceof ManagementError || error instanceof AdapterError) { code = error.code; exit = 1; detail.status = error.status; }
  if (error instanceof OutcomeUnknown || error instanceof WorkOutcomeUnknown || error instanceof MutationOutcomeUnknown) {
    code = error.code; exit = 3; detail.input = error instanceof WorkOutcomeUnknown ? JSON.parse(stringifyWorkbenchJson('ApplyCall', error.input)) : error.input; if (error instanceof MutationOutcomeUnknown) detail.operation = error.operation;
  }
  if (error instanceof ReceiptError) { code = error.code; exit = code === 'OutcomeUnknown' ? 3 : code === 'Failed' ? 1 : 2; detail.operation = error.operation; }
  if (error instanceof ObservationStopped) { code = error.code; exit = code === 'Canceled' ? 130 : 124; detail.identity = error.identity; detail.lastOperation = error.lastOperation; }
  if (error instanceof AdapterDisconnected) { code = 'AdapterDisconnected'; exit = error.cause instanceof Error && error.cause.name === 'TimeoutError' ? 124 : 2; }
  if (error instanceof CloudAdminError) { code = error.code; exit = code === 'DeadlineExceeded' ? 124 : error.status && error.status >= 400 ? 1 : 2; detail.status = error.status; }
  if (error instanceof CloudAdminOutcomeUnknown) { code = error.code; exit = 3; detail.operation = error.operation; detail.namespace_id = error.namespaceId; detail.request_id = error.requestId; }
  if (error instanceof ProtocolError) { code = error.code; detail.schema = error.schema; detail.rawText = error.rawText; }
  if (error instanceof UsageError) code = 'InvalidInput';
  if (error instanceof Error && error.name === 'TimeoutError') { code = 'DeadlineExceeded'; exit = 124; }
  console.error(stringifyJson({ code, ...detail })); process.exitCode = exit;
});
