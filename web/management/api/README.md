# Native management declarations

Rust serde declarations own wire contracts. Regenerate from the repository root after changing them:

```sh
python3 web/workbench/api/derive.py --sources src/management/contract.rs src/agents/model_request.rs --roots AgentPage Descriptor WorkspaceCatalog RoleCatalog DirectoryEntry SubmitDirectoryMessage DirectorySendOutcome HumanMailbox HandleHumanMessages OfficeActivity LaunchRelations Attachment SubmitLaunch SubmitMessage SubmitTarget Operation TranscriptPage MessagePage --numbers number --output web/management/api/native.ts
```

The shared generator follows the finite type closure and fails on unsupported shapes. `RoleDefinitionView.profile` is nullable catalog metadata; `RoleView.profile` is the concrete adopted/default-route profile. Do not replace adopted facts using current catalog defaults. Typechecks and fresh embedded builds consume this file.
