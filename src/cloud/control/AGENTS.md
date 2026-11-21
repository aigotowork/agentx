# Daemon management control

Start with the [wire contract](../../../.controller/cloud-daemon/wire.md) and [API Task](../../../.controller/cloud-daemon/api.task.md). [Types](types.rs) are shared by the native uplink and both native OpenAPI projections. [HTTP adapters](http.rs) compose into the existing Admin and Scope bearer guards. [Store transitions](store.rs) use the existing communication database and transaction fence; [control tables](schema.sql) are added on first control access to a scope. Native launch, exact process ownership and local effect recovery live with the [uplink](../uplink.rs) and client, outside this deployment boundary.

Run `cargo test cloud::` for the focused native tests and use the [acceptance Task](../../../.controller/cloud-daemon/acceptance.task.md) for real daemon and Provider evidence. Export both surfaces with `agentx cloud-openapi admin` and `agentx cloud-openapi scope`; the SDK owner regenerates the consumers from those outputs.
