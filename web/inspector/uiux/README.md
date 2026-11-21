# Preserved UIUX resources

[Inspector](systems/inspector/README.md) and [Recovery](systems/recovery/recovery.design.md) own the migrated production components and resources. [Original UIUX design](uiux.design.md) and [historical proposals](../design/README.md) preserve authoring context; their original deployment URLs and checkout links are historical. The maintained local runtime is [AgentX Inspector](../README.md), including `?surface=components`. No standalone workbench or native application is installed by this module.

`framework/` contains the exact pure token/resource model snapshot with adjacent MIT license and provenance. It avoids a sibling-checkout runtime dependency. See [source inventory](../docs/source-inventory.json) for file provenance.
