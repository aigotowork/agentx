---
schema: task
status: completed
agents: [{"scope":"/home/long/works/longwork/agentx/.agentx","generation":"a6195a5b-1dd4-4a6d-87d4-1f18f1b664fb","participant":{"canonical_name":"gule","incarnation_bits":4745316797062995063}}]
---
Implement exact-instance read-only production read adapter and any required service queries. Own src/tui/info_data.rs and service/domain code. Use existing native reads. Define info_data::Subject (derive Clone, Debug) with public name:String and constructor from_agent(&Agent)->Self. Subject owns exact reference, may carry projected related agent. Define enum Page { Agent, Role, Profile, Task(String), Bootstrap, Runtime, Instructions, TaskBody(String), Children } (additional variants as needed), derive Clone,Debug. Define enum Link { Page(Page), Agent(Subject) }, derive Clone,Debug. Define Row { pub label:String, pub value:String, pub link:Option<Link>, pub status:Option<AgentStatus> }. Expose pub(super) fn read(subject:&Subject,page:&Page)->Result<Vec<Row>,String> for async worker; pub(super) fn summary(subject:&Subject)->Vec<Row> for immediate Agent summary including icon and created time. Implement ALL data and actions from approved plan, including role/profile/task reader, pagination and exact historical targets. Replay agent may coordinate fixture constructor needs. Do not edit tui/mod.rs. Report validation.

## Read adapter structure

The [approved interaction](AGENTS.md) is supplied by three existing authorities: stored launch and prompt facts, immutable [creation provenance](../../src/agents/launch_relations.rs), and current [Task/Delivery documents](../../src/file_bindings/AGENTS.md). The TUI row snapshot carries optional exact scope, database generation and current incarnation or stopped event. The daemon read lane validates that reference, returns only the selected projection, and uses the existing file worker for exact assignment matching. The TUI adapter owns display rows, relation and document links, and on-demand text presentation. Remote rows retain only their already supplied facts.

The existing stopped list had no event ID and may substitute a timestamp when a legacy stop has no creation time. The new projection retains the exact event ID rather than inferring historical identity from that display field. The existing actor instruction read may persist aliases, so Info uses a pure read of the exact retained instruction record. No new task, profile, relationship or runtime mutation is introduced.

## Integration handoff

Register `info_data` alongside the navigation module. `Subject::from_agent` preserves the exact native reference; the test fixture helper exposes device, canonical name and creation bits. `summary` provides immediate rows; the loaded Agent page replaces them with the complete exact view. The renderer prepends `Row.status.icon()`; `Subject::is_request_error` supplies the existing request error color. The read worker performs all IPC and document I/O. Page breadcrumbs use `Page::caption`.

A daemon that predates this feature can still supply basic dashboard rows, but cannot supply exact Info details. Old stopped records can lack frozen facts or a usable stop snapshot. Explicit Task/Delivery paths outside discovered projects need the existing binding index to have observed them. Those limits preserve truthful data and the [file binding retention model](../../.docs/file-task-bindings.md#read-model-and-refresh).


## Validation

The shared debug build completed. The focused [read lane regression](../../src/service/application/acceptance_tests.rs) passed against the compiled test binary: a stopped incarnation retains its frozen Bootstrap, role, profile and exact binding after a same-name replacement; forged stop identity and a stale generation are rejected. All seven launch relation tests and fourteen snapshot query tests passed on the same build. The isolated [native TUI smoke](../../tests/tui_terminal/info.py) exercised saved Role/Profile, Bootstrap, a current Task body, and visits between real native launch relations using a mock Provider. Running services and installed binaries were not changed.
