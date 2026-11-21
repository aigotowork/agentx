# Context lifecycle implementation

Read root AGENTS.md, [design](design.md), [delivery](delivery.md) and your assigned task. Preserve all concurrent changes and installed/user runtimes. This follows the completed first context Delivery, not a restart of it. Mika owns core/config/native hooks; the plugin worker owns only its named files; the verifier owns only new tests and acceptance. No main Cargo build until Mika hands off a final candidate. Use existing domain/client boundaries; no managed DB access in Provider adapters.
