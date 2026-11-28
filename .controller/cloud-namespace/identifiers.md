---
status: deployed v2
owner: Cloud Namespace integration
---

# Compact AgentX identifiers

Cloud v2 has one format. Workspace uses `WS` and ten ASCII letters or digits, for a total of 12 characters. Identity is the pair of Host and Workspace ID. The project ID is derived from the canonical path; the host journal addresses registrations by path and refuses ambiguous compact IDs.

Cloud Agent incarnations and Admin/Namespace bearers are exactly 12 case-sensitive ASCII letters or digits, generated with cryptographic randomness. Default request IDs use the same format; explicitly supplied replay keys retain their own contract. User-selected namespace and host names, native ordered keys, database generations, installation/lease/claim fences, Provider identities and Relay keys keep their domain contracts.

The [cleanup delivery](cleanup.md) owns source changes and verification. V2 uses a fresh deployment volume without in-place rewriting, aliases or mixed-version enrollment. Old volumes, configs and binaries are retained as rollback resources.
