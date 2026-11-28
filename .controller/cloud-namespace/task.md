---
schema: task
status: completed
owner: Cloud Namespace integration
agents: [{"scope":"/home/long/works/longwork/agentx/.agentx","generation":"a6195a5b-1dd4-4a6d-87d4-1f18f1b664fb","participant":{"canonical_name":"tina","incarnation_bits":4745317679790839645}},{"scope":"/home/long/works/longwork/agentx/.agentx","generation":"a6195a5b-1dd4-4a6d-87d4-1f18f1b664fb","participant":{"canonical_name":"rave","incarnation_bits":4745317679791235696}},{"scope":"/home/long/works/longwork/agentx/.agentx","generation":"a6195a5b-1dd4-4a6d-87d4-1f18f1b664fb","participant":{"canonical_name":"mizo","incarnation_bits":4745317679791792999}},{"scope":"/home/long/works/longwork/agentx/.agentx","generation":"a6195a5b-1dd4-4a6d-87d4-1f18f1b664fb","participant":{"canonical_name":"koma","incarnation_bits":4745317679792261247}}]
---

# Cloud Namespace and project workspace

The [approved plan](../../.codex/attachments/ed3c3591-1ada-47c2-95da-6e2aed102523/approved-plan.md) owns requirements. Namespace owns a deployment generation and credentials; Host identifies a machine/installation lease in the Namespace; Workspace is a project directory identity within a Host; Agent belongs to the Workspace selected during launch preparation. Agent address remains (host, name, incarnation); cwd is a separate fact. Local Scope selects daemon/runtime data and enrolls in one Namespace.

## Contract

- Public URL is /namespaces/{id}/space/ and /namespaces/{id}/api/v1; Admin is /api/v1/admin/namespaces/{id}. Public DTOs expose namespace_id and namespace_url where applicable. Native enrollment is daemon connect --namespace-url URL [--project-root alias=path] and private cloud.toml uses namespace_url, token, host, project_roots. Local --scope stays local. New deployment data volume; old public routes and arguments retire.
- Workspace IDs are opaque, stable host-derived strings (domain-separated SHA-256 over canonical project path); identity is (host, workspace_id). Display name is independent and can repeat across hosts or within a host. Neither absolute paths nor default root aliases represent Workspace IDs. Scan only opened roots and direct children, with existing project recognition and no escape through symlinks. Explicit registration takes parent root alias, confined relative directory, name and create_directory. Persist explicit registration in the existing local control journal; it overlays automatic naming and does not duplicate the directory.
- Shared Rust control DTOs: WorkspaceCapability has workspace_id, name, default_available, default_reason, roles. HostCapabilities carries workspaces and project_roots (a list of eligible root aliases), plus existing directory_launch and workspace_management flags. RenewControlLease carries workspaces and project_roots. Launch has workspace_id instead of workspace; AddWorkspace has name instead of alias (parent, directory, create_directory retained). WorkspaceAdded returns workspace_id. Preserve exact stop semantics. Launch success is checked against authoritative registration's Workspace ID; replay/installation guards remain.
- AgentWorkspace is a tagged value with {kind:"project", workspace_id}, {kind:"global"}, {kind:"unexposed"}; field workspace on AgentRegistration, AgentView and CloudAgentSummary. Membership is launch-time metadata, independent of facts and status. Exact historical records preserve it; older local records without attributable evidence must not acquire a new project based on cwd alone. Namespace and Admin GET /agents support host and workspace_id together.
- Namespace and Admin GET /workspaces accept optional host, after and limit; response {items, next}. WorkspaceView flattens WorkspaceCapability and adds host, online. Preserve known catalogues for offline hosts; online reflects lease availability. Existing /changes roster channel wakes on catalogue/availability changes; identical renewal only moves the lease timestamp. Read-only Namespace bearer cannot use Admin mutations.
- SDK exposes createCloudNamespaceClient and generated/cloud-namespace. Reuse shared Space App: Namespace overview groups by host, project rows are office links and exact member routes retain host/name/incarnation. Project selections use host and workspace_id. Launch preselects the focused project, reads that project's roles, and opens the exact Transcript on success. Explicit management uses existing Admin command/receipt journal. Show global/unexposed Agents in the overview.

## Evidence

Independently exercise isolated HTTPS on two hosts/installations, same-name projects, known empty project, local and managed launch into one project, nested cwd, role isolation, add/create and restart, offline catalogue, permissions, and retired routes. Public cutover follows isolated acceptance; record exact rollback image/volume and original host enrollment backups separately from source acceptance. Use no existing local runtime DB or project file as disposable test data.

The launcher requires a task schema in front matter; the four initial bindings rejected the file until the schema was added.

The main integration owner owns native public Namespace naming, SDK/CLI, deployment and integration. Native host/project-role discovery and registration emission, deployment project API/storage, and Space composition are separate lanes. Coordinate shared Rust communication/admin files before replacing namespace names. Verification can prepare real two-host acceptance independently from implementation.

## Build notes

The native build embeds browser assets and the browser build consumes native OpenAPI. For the initial name break, a temporary source snapshot exported the Rust schemas with empty browser asset tables. It was never used to serve or deploy. The normal assembled binary had to emit byte-identical OpenAPI after the SDK was regenerated. A composed Workspace view initially failed SDK validation because the capability schema disallowed the flattened host fields. The native schema composition was fixed and regenerated; the SDK test covers the resulting page contract.

The [isolated acceptance](acceptance.md) and [public cutover](rollout.md#accepted-public-outcome) are complete. The deployment runs the same native candidate exercised by the final HTTPS fixture.
