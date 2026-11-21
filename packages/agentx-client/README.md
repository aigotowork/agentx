# AgentX HTTP client

`@agentx/client` 提供浏览器可导入的管理 SDK，以及独立的 Node 命令 `agentx-http`。完整 SDK/CLI 批次覆盖三个独立 listener；各阶段实际证据见 full Task。原生 `agentx` 命令不变。需要 Node 22+ 运行 CLI，SDK 使用标准 Fetch API。

```js
import { createManagementClient, OutcomeUnknown, stringifyManagementJson } from '@agentx/client';
const api = createManagementClient({ baseUrl: 'http://127.0.0.1:PORT' });
const descriptor = await api.describe();
const page = await api.listAgents({ generation: descriptor.generation, limit: 50 });
const selected = page.agents.find(view => view.can_send);
if (!selected) throw new Error('没有可发送的 Agent');
const target = await api.getAgent({ generation: descriptor.generation, agent: selected.agent });
const input = {
  generation: descriptor.generation,
  request_id: crypto.randomUUID(),
  agent: target.agent,
  message: { text: 'hello', attachments: [], intent: 'inform', reply_to: null, thread: null },
};
// 在 dispatch 前保存完整 input；浏览器可使用自己的持久存储。
localStorage.setItem(input.request_id, stringifyManagementJson('SubmitMessage', input));
try {
  const receipt = await api.send(input);
  // Accepted 表示已接纳；Sent 表示原生消息已接纳，并不表示 Agent 已阅读或回答。
  const sent = await api.waitForSent(input, { timeoutMs: 30000 });
  console.log(receipt.request_id, sent.message_id);
} catch (error) {
  if (error instanceof OutcomeUnknown) {
    const original = await api.getOperation(error.input);
    console.log(original); // 不生成新 ID 自动重发。
  } else throw error;
}
```

`getOperation({ generation, request_id })` 和 `waitForOperation(identity, options)` 返回原回执。`waitForSent` 组合回执查询并检查原身份、终态和 `Sent` 标签；其他成功结果会报 `ReceiptError`，保留 `.operation`。`Failed` 和 `Unknown` 分开表达；`ManagementError.code` 是稳定服务错误码。`NotFound` 不证明消息没有执行。SDK 不自动重试提交，也不在 stale 错误后更新 generation 或按名称替换目标。

等待选项：`signal`、`timeoutMs`（默认 30000）、`pollIntervalMs`（默认 250）。`ObservationStopped` 保留 `.identity`、`.lastOperation` 和 `Canceled`/`DeadlineExceeded`；取消等待不会取消服务端工作。单次调用也接受 `{ signal }`，SDK 的单次 HTTP 默认不设超时。发送期间中断或丢失响应会成为 `OutcomeUnknown`，其 `.input` 是派发前校验、复制并冻结的完整请求；调用方仍应在调用前持久保存请求。

普通管理 JSON 的 `u-int64`/`int64` 在 SDK 中为 `bigint`；`u-int32`/`int32` 为 `number`，浮点为 `number`。generation、incarnation、message_id、request_id 和 revision 是原样保留的字符串。`parseManagementJson(componentName, text)` 和 `stringifyManagementJson(componentName, value)` 按实际导出 schema 校验结构和整数范围，bigint 编码为未加引号的 JSON 整数；不把所有数字字符串转成数字。保存 SDK 结果可用 `stringifyJson`，普通 `JSON.stringify` 不支持 bigint。原生约束、权限、跨字段关系仍由服务端执行。

## Three-listener SDK and CLI

The complete batch uses the actual native-emitted contract chain. Whole-package generation/build,11 focused consumer cases, and the independent retained-workspace Linux integration all passed at the documented public-package boundary. The final tarball and 57-file reproducible regeneration are recorded in the [full consumer Task](../../.controller/contract-service-projections/tasks/full-consumers.task.md) and [full acceptance evidence](../../.controller/contract-service-projections/tasks/full-acceptance.task.md). Earlier pilot artifacts remain bounded historical evidence. No running service or installed binary is replaced by this package.

The public composition is `createAgentXClient({ managementUrl, workbenchUrl, observationUrl, fetch? })`, with independently configured clients; omitted domains are undefined. It performs no network or daemon acquisition at construction. Each can also be created separately using `createManagementClient`, `createWorkbenchClient` or `createObservationClient`.

Management groups are `workspaces.list/members`, `roles.list`, `directory.inspect/send`, `agents.list/get/launch/kill/stopParticipation/resume/fork/transcript/relations`, `agents.messages.list/send`, `agents.labels.get/patch`, `attachments.upload/download`, `humanMailbox.list/handle`, `officeActivity.list` and `inspection.get/configure/analyze/cancelAnalysis/requestContinuation`. Original `describe/listAgents/getAgent/send/getOperation/waitForOperation/waitForSent` stay available. The kill/stop HTTP alias has one clear SDK method `kill`. Inspector shared inputs are ordinary JSON with exact single external tags on wire; its Snapshot, AnalysisJob and RecoveryAttempt never become a management Operation. `queued` continuation is mailbox admission, not model execution.

Workbench exposes `describe`, closed `read/apply`, and named `works.list/get/candidates/command/thread/create/updateBrief/bind/dispatch/note/answer/requestCheck/accept/requestChanges`. Each named read/mutation preserves the full native `Response` union; HTTP admission errors are `AdapterError`, while a native `Rejected` remains a value. `WorkOutcomeUnknown.input` retains a detached full original ApplyCall with generation/command_id/work/expected_revision/command; native Unknown also remains unknown. `works.command(originalGeneration, originalCommandId)` observes original durable changes. No automatic rebase, replacement revision, new-ID replay or AgentReport authority is added. `participantFromAgent` only converts a current exact decimal incarnation to native `{canonical_name,incarnation_bits}` and refuses historical stopped refs; actual native membership/authority checks still apply.

Observation exposes `scope`, closed `read`, `runtime.snapshot`, `runtime.requests.list/get`, `runtime.connections/peers/hosts`, `runtime.clientReports.list/get`, `configuration.head/get/compare`, and `events.describe/read`. `expectedDaemon` defaults to null and is never refreshed implicitly. Methods preserve complete ReadResult including LocalFailure, native errors, DaemonChanged, incomparable configuration, stale epochs, event gaps and coverage. `close` stops this client's reads; no underlying daemon is started, stopped or rebased. A local one-EventsRead guard and bounded in-flight reads follow the established browser contract; there is no automatic poll/retry loop.

All Workbench/Observation SDK integers are `bigint`, derived with `Native<T>` from actual emitted marker DTOs. `parseWorkbenchJson/stringifyWorkbenchJson` and `parseObservationJson/stringifyObservationJson` operate on exact emitted component names, enforce native width/sign metadata and reject bare wire numbers, malformed markers or duplicate keys. CLI structured files and JSON stdout for these two listeners use actual `{"$integer":"decimal"}` wire objects. This lets file operands round-trip without losing nested IDs/revisions/timestamps; their ordinary opaque strings stay strings. Management CLI files and stdout keep ordinary lossless JSON numbers. Cross-listener codecs are not interchangeable.

CLI includes the complete resource tree from `--help`: management/workspaces/roles/directory/agents/operations/attachments/mailbox/office/inspection, Workbench operator workflows under `work`, and passive diagnostics under `observe`. The final bin is `agentx-http`; `agentx-api` in the panorama is historical working naming. `mailbox list --pending-only` distinguishes filtering from `--pending FILE` write recovery storage. Complex `--field` values are native tagged JSON rather than a second invented field-selector grammar. Work `--input` is a complete original marker-coded ApplyCall. Full Work methods retain exact revision; omitted `--if-revision` reads the selected detail once, and accept's omitted `--brief-revision` is taken from the specifically selected result basis. Revision conflict never triggers an automatic reread/reapply.

Independent URL configuration uses CLI > ENV > config: `--management-url`/`AGENTX_API_MANAGEMENT_URL`/`management_url`, and matching `workbench` and `observation` variants. Config also supports `format`, `timeout_seconds`, `poll_seconds` and `wait_timeout_seconds` with corresponding AGENTX_API ENV. Generation now takes one actual-spec directory, not a handwritten spec or separate DTO file:

```sh
CANDIDATE __export-management-openapi > ACTUAL_SPEC_DIR/management.json
CANDIDATE __export-workbench-openapi > ACTUAL_SPEC_DIR/workbench.json
CANDIDATE __export-observation-openapi > ACTUAL_SPEC_DIR/observation.json
npm run generate --prefix packages/agentx-client -- /ABS/ACTUAL_SPEC_DIR
```

One whole consumer build/check and one concentrated retained-workspace integration follow the complete generated batch. Existing public tarball evidence is not repeated per method or ordinary handoff. No registry publication or installed/live adoption is included.

## Cloud deployment and scopes

云端管理和消息通信使用不同的地址和 Bearer。管理端使用部署根地址，例如 `https://agentx.example`；通信端使用管理回执的 `scope_url`，也可直接传入完整的 `/scopes/ID/api/v1` 地址。两个客户端均要求 HTTPS，只有 loopback 允许 HTTP，并拒绝重定向。

```ts
import { createCloudAdminClient, createCloudScopeClient } from '@agentx/client';

const admin = createCloudAdminClient({ baseUrl: deploymentUrl, token: adminToken });
const overview = await admin.overview();
const scope = await admin.getScope('office');
const communication = createCloudScopeClient({ baseUrl: scope.scope_url, token: scopeToken });
const descriptor = await communication.describe();
```

`CloudAdminTypes` 和 `CloudScopeTypes` 按原生 OpenAPI 组件名导出。两者的 64 位时间戳在 SDK 中为 `bigint`；标识和消息 ID 仍为字符串。管理方法包括 `overview`、`listScopes`、`getScope`、`listHosts`、`listAgents`、`putScope` 和 `rotateScopeToken`。通信方法包括 `describe`、`listHosts`、`listAgents`、`renewHost`、`registerAgent`、`retireAgent`、`send`、`getOperation`、`claimInbox`、`confirmInbox` 和 `recoverInbox`。各调用可传 `{ signal }`。查看[通信契约](../../.controller/cloud-scope/communication-wire.md)了解写入的代次要求、原始消息查询及投递确认。

先保留完整的写入参数，再提交请求。管理 SDK 的 `CloudAdminOutcomeUnknown` 含 Scope、操作、请求 ID，不含令牌；由调用方决定是否向同一部署显式重放原请求。通信 SDK 的 `CloudScopeOutcomeUnknown` 要用原请求或 claim 身份来核对。客户端不会自动重试、变更代次或释放 claim。

```sh
# Set AGENTX_ADMIN_URL and AGENTX_ADMIN_TOKEN before remote commands.
agentx-http admin overview
agentx-http admin scopes list --limit 20
agentx-http admin scopes show office
agentx-http admin scopes hosts office
agentx-http admin scopes agents office --host host-a
agentx-http admin scopes create office --pending ./office-create.json
agentx-http admin scopes rotate-token office --expected-version 1 --pending ./office-rotation.json
# After an unknown result, explicitly replay the retained request against the same deployment:
agentx-http admin pending replay ./office-rotation.json
# Explicitly print the proposed token as JSON; this does not check whether it is active:
agentx-http admin pending reveal ./office-rotation.json
```

都可以省略 `--pending`；CLI 默认用 `$XDG_STATE_HOME/agentx-http/cloud-admin`（未设置则用 `~/.local/state`），派发前保存带原 URL、部署 ID 和请求参数的私有文件。重放前会检查当前部署 ID。在 POSIX 上文件模式为 0600，重放和显示拒绝读取其他用户可访问的文件。普通回执和错误不显示令牌。默认生成令牌，也可用 `--new-token-file FILE|-` 从文件或 stdin 提供。远程命令的管理 URL 优先取 `--admin-url`，其次为 `AGENTX_ADMIN_URL`，然后才取配置的 `admin_url`；管理令牌只取 `AGENTX_ADMIN_TOKEN`。

启用云端管理的主机可通过 `getHostCapabilities(scope, host)` 查看安装标识、目录更新时间、可用工作区和角色。调用 `submitControlOperation(scope, original)` 提交一项启动或停止；`getControlOperation(scope, requestId)` 读取回执，`recoverControlOperation(scope, original)` 还会核对原始代次、主机安装与完整命令。通信端的同名消息回执是另一资源。查看[云端控制契约](../../.controller/cloud-daemon/wire.md)了解状态和代次。

```sh
agentx-http admin hosts capabilities office --host host-a
# Use values returned by capabilities for G and I. Omit --role for default Codex.
agentx-http admin operations launch office --generation G --host host-a --installation-id I \
  --workspace studio --name nova --title 'Review notes' --task 'Summarize the latest notes.'
agentx-http admin operations show office --request-id Q
# Use the exact name and incarnation from the intended Agent row or launch receipt.
agentx-http admin operations stop office --generation G --host host-a --installation-id I \
  --name nova --incarnation INCARNATION
```

`queued` 和 `running` 表示主机尚未提供最终结果。`failed` 的 CLI 退出码为 1，`uncertain` 为 3；后者可能随原主机报告更新。提交响应丢失后可用 `admin operations show` 查询原请求。`admin pending replay FILE` 会先核对部署和原回执，仅在未找到回执时重发完全相同的请求。启动任务文本也保存在私有 pending 文件中；文件不含管理员令牌。

使用已构建的原生候选程序重新导出两个云端契约和对应 SDK；不涉及部署或服务启动：

```sh
npm run generate --prefix packages/agentx-client -- --cloud /ABS/AGENTX_CANDIDATE
```

选择性更新时可在命令尾部加 `admin` 或 `scope`。云端生成不会覆盖本地三个 listener 的投影。

## Scoped domain objects

`selectAgentXScope({ baseUrl, expectedScope })` 用可信的原生作用域路径选择 Management listener，并固定数据库代次。`scope.verify()` 可以主动确认 listener 仍是同一作用域、同一代次；原有句柄不会随检查切换。用 `scope.workspaces()` 查找工作区，`workspace.roles()` 读当前有效岗位定义；Agent 快照记录启动时实际采用的岗位。`workspace.members({ limit, after })` 是兼容的全局分页投影：一页可能没有成员，但 `coverage.next` 仍可继续。`scope.workspace('global')` 只表示明确采纳的全局工作区；旧记录和未知归属各自保留。

若 `scope.supportsWorkspaceMembers` 为真，`workspace.memberPage({ limit, after, currentOnly })` 则由原生先筛成员，再分页。翻页请沿用同一个 `currentOnly` 和该方法返回的 `coverage.next`；不能混用两种游标。旧 listener 缺少能力声明时，由应用明确选择兼容投影。

若 `scope.descriptor.supports_message_history === true`，用 `agent.messageHistory()` 直接读取最近 20 条，按事件顺序显示。使用返回的 `nextBefore` 作为 `agent.messageHistory({ before })` 的参数可翻阅更早消息；`null` 表示已到头。只用最新页的 `liveAfter` 调用 `agent.messages({ after })` 继续收取新消息。旧页的 `liveAfter` 固定为 `null`。两类游标都绑定原 listener、代次和精确 Agent，不能互换。能力字段缺失时可继续用 `agent.messages()` 顺序读取。

```ts
import { selectAgentXScope, OutcomeUnknown } from '@agentx/client';

const scope = await selectAgentXScope({ baseUrl, expectedScope });
const agent = scope.agent(savedCurrentAgentAddress); // or obtain a current handle with workspace.members()
const snapshot = await agent.snapshot();
const history = await agent.messages({ limit: 20 });
const prepared = agent.prepareMessage({ text: 'Please review', intent: 'request' }, requestId);
await saveOriginalCommand(prepared); // caller-owned; save before dispatch
try {
  await agent.send(prepared);
} catch (error) {
  if (!(error instanceof OutcomeUnknown)) throw error;
}
const receipt = await scope.operation(prepared).wait();
if (receipt.status === 'Succeeded') {
  console.log(receipt.result.message_id); // 已接纳；实际投递、阅读和回复分别观察
}
```

从当前或已停止的精确 Agent 读取 `agent.launchRelations({ limit, after })`，用可用关系的 `relation.agent.address` 打开对应会话。不可用关系仍显示创建事实，但没有可跳转地址。Provider 摘录独立读取：先保存 `agent.prepareTranscript(requestId)`，再调用 `agent.requestTranscript(prepared)`；`scope.operation(prepared)` 随时可查询原回执。`Succeeded` 后的 `result.available` 区分摘录可用与不可用。该摘录无公开的翻页游标，也不会推进 AgentX 消息的读取位置。

`scope.prepareLaunch` / `scope.launch` 与 `workspace.prepareLaunch` / `workspace.launch` 接受显式启动参数；Workspace 版本固定工作区，启动时由原生解析角色选择。启动、恢复或分叉成功时，只用 `result.agentAddresses` 导航到原回执记录的精确实例。没有保留精确地址的旧回执返回 `null`，仍可读其名称和诊断。Agent 生命周期与标签操作也采用先准备再派发的方式。`scope.attachments` 注册及下载附件；`scope.inbox` 分开读取与标记所选信件；`scope.directory` 分开预览路由与实际发送；`scope.officeActivity` 只观察已接纳的活动。各分页游标仅在原作用域和选择下有效。

查看 [发送示例](examples/scoped-send.mjs)、[Transcript 与恢复示例](examples/scoped-transcript.mjs) 或 [完整对象模型](../../.docs/model/typescript-domain-sdk.md)。原有地址观察 `scope.operation(prepared.operation)` 仍返回原生回执；已知命令时的类型化回执也保留 `.receipt`。丢失响应或回执不匹配时，请保留原命令并查询原 ID。过期代次、回执缺失、等待超时与 `Unknown` 均不能作为重发依据。

## Build and public installation

```sh
npm ci --prefix packages/agentx-client
# Export three actual specs first using the commands below.
npm run generate --prefix packages/agentx-client -- /ABS/ACTUAL_SPEC_DIR
npm run build --prefix packages/agentx-client
npm test --prefix packages/agentx-client
npm pack ./packages/agentx-client --pack-destination /ABS/ARTIFACT_DIRECTORY
# Use the existing owned external consumer workspace for integration:
npm install /ABS/ARTIFACT_DIRECTORY/agentx-client-0.1.0.tgz
node --input-type=module -e "import {createAgentXClient} from '@agentx/client'; console.log(typeof createAgentXClient)"
./node_modules/.bin/agentx-http --help
```

The standalone lock pins @hey-api/openapi-ts 0.99.0, TypeScript 5.9.3 and lossless-json 4.3.3, with js-yaml 4.3.2 override. `src/generated` is regenerated, never patched. The generated SDK returns raw text to schema-directed decoding, avoiding prior Number rounding. SDK imports contain no Node filesystem/process/server dependencies; only the separate bin imports Node. Help/version are offline.

## CLI inputs, results and recovery

Use `agentx-http --help` for the complete command grammar and exact flags. Globals may appear anywhere. All business payloads, generation/revisions and request/command identities are explicit single-intent inputs rather than injected from config. Config is selected by `--config`/`AGENTX_API_CONFIG`, default `$XDG_CONFIG_HOME/agentx-http/config.json` or `~/.config/agentx-http/config.json`; an absent default is allowed, an absent explicit file is an error. Precedence is CLI > ENV > config > defaults. Only JSON format is supported. Per-attempt HTTP timeout defaults to 15 seconds; wait timeout 30 seconds and poll 0.25 seconds. Only one file operand may use stdin.

SDK/new-request workflows must save full input before dispatch. CLI does so automatically: `--pending FILE`, default `.agentx-http/requests/ENCODED_ID.json`, exclusive creation mode0600 and fsync before POST; different existing contents are never overwritten. The same exact retained file can be deliberately replayed using the corresponding command's `--input FILE|-`, which replaces target/body/identity flags. First query the original receipt/change after loss: `operations show Q --generation G` or `work command Q --generation G`; a missing result never proves no effect. No automatic retry, generation refresh or revision rebase occurs. Pending files are not removed at command exit. Nonjournal mutation errors retain their original operation/input as `MutationOutcomeUnknown`; a new random local pending filename does not create native idempotency.

`agents messages send --wait` checks `Sent`; generic journal writes with `--wait` observe the original terminal Operation. `operations wait --sent` checks identity/status and the single Sent tag, retaining a wrong receipt in `ReceiptError` or raw malformed response in `ProtocolError`. Cancellation/deadline preserves `ObservationStopped.identity/lastOperation` and stops only observation. Neither Sent nor Work MessageAccepted nor Inspector queued proves Provider delivery or model execution.

stdout contains lossless JSON, except `attachments download ID --output -` emits only bytes. stderr contains structured diagnostics/errors and pending identity. Binary uploads are raw bytes, not multipart/JSON; `attachments upload -` requires `--name`. File downloads exclusive-create their output, refusing overwrite. Exit0 means success or admission, 1 explicit service/native refusal or LocalFailure, 2 invalid local input/protocol, 3 unknown mutation outcome, 124 local deadline and130 local read/wait interruption. A submission interrupted after dispatch exits3 and retains original input because admission is uncertain. Native Work Rejected remains a returned value even though CLI reports failure; Observation native evidence/error branches are printed intact.

```sh
# Message intent with original identity retained before dispatch:
agentx-http agents messages send NAME --incarnation I --generation G --request-id Q --text hello --management-url URL
agentx-http operations show Q --generation G --management-url URL
# Work operands contain actual marker-coded native JSON:
agentx-http work create --brief brief.json --implementer-ref implementer.json --checker-ref checker.json --workbench-url URL
agentx-http work accept WORK_ID --result RESULT_ID --check CHECK_ID --if-revision REV --brief-revision BRIEF_REV --workbench-url URL
# Passive listener operations do not acquire/start the daemon:
agentx-http observe scope --observation-url URL
agentx-http observe snapshot --expected-daemon saved-daemon.json --observation-url URL
agentx-http observe events read --window window.json --observation-url URL
```


## Web consumers

Management, Agent Space and live Inspector consume named management methods. Workbench uses the public Workbench client behind its page read-lifetime wrapper; Observation delegates to the passive SDK and bundles it with the existing root Vite toolchain. Public schema-map aliases replace their older independent DTO generation. Demo and extension embed the complete Space and use the existing postMessage bridge; their configuration/iframe operations are not AgentX HTTP transports to replace. Demo `/api/config` remains demo-owned static configuration.

Embedded pages default to their own listener. Explicitly separately hosted pages select `managementUrl`, `workbenchUrl` or `observationUrl` query parameters for that application's domain; none is inferred from another. These URLs are endpoint choices, never identities or credentials. Public clients use credential-free Fetch, no-store and redirect refusal, and do not fabricate Origin; browsers supply their actual origin. Native admission/CORS and browser/platform mixed-content/local-network/extension permissions are separate constraints. No origin allowlist, handshake or mandatory proxy is required. Existing optional Vite development proxy configuration is compatibility tooling only.

SDK values include bigint. `stringifyJson(value)` emits lossless ordinary JSON for diagnostics/storage. `parseJson(text)` reads an application-owned JSON envelope without rounding unsafe integer tokens, rejecting duplicate keys/nonfinite values; schema-specific `parseManagementJson(name, stringifyJson(value))` restores the emitted field types (including safe bigint longs). This generic helper is not domain validation. Management/Space/Inspector retain their own journals and original identities; no automatic retry/rebase is added. Inspector pending mutations restore the actual emitted input component on reload. Historical stored endpoint paths are compatibility metadata only; named SDK methods own dispatch.

Prepare checkout or Cargo source-archive dependencies once before checks:

```sh
npm ci --prefix packages/agentx-client
npm ci
npm ci --prefix web/inspector
npm run build --prefix packages/agentx-client
```

The root and Inspector dependencies link to this local public package. Standalone Web typechecks/builds need its built `dist`; normal Cargo embedding builds the package once before all five apps and watches SDK source/config/scripts/manifests so changes invalidate embedded assets. It never installs packages or embeds stale dist as a fallback. SDK publication still uses its standalone generator/build/pack scripts; Node CLI stays separate from browser entry.

Current source scope and unexecuted/executed evidence are maintained by the [adoption Web Task](../../.controller/web-sdk-adoption/web.task.md) and [integration Task](../../.controller/web-sdk-adoption/integration.task.md). Earlier accepted SDK package tests remain historical evidence, not proof of this migration's browser acceptance or installed/live adoption.

Current Web integration evidence: the [Web contribution](../../.controller/web-sdk-adoption/web.task.md#complete-websdk-contribution-handoff) records the implemented five-consumer migration, source/build decisions and frozen candidate boundary. The [independent integration Task](../../.controller/web-sdk-adoption/integration.task.md) owns exact executed browser/native cases, environment, cleanup and remaining acceptance limits. Local evidence includes named SDK-backed reads, exact original receipt recovery, cross-origin preflight/errors, binary bytes and bigint mapping; it does not imply installed/live adoption or acceptance of adjacent newer vendor features. Build prerequisites above remain canonical.
